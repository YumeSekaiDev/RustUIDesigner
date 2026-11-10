//! Real interactive PTY terminal (instant keystrokes, live shell in project cwd).

use egui::{Color32, FontId, Pos2, Rect, Sense, Ui, Vec2};
use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use std::io::{Read, Write};
use std::num::NonZeroU16;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

const DEFAULT_ROWS: u16 = 28;
const DEFAULT_COLS: u16 = 100;

struct PtyHandles {
    writer: Box<dyn Write + Send>,
    master: Box<dyn MasterPty + Send>,
    _child: Box<dyn Child + Send + Sync>,
}

/// Interactive shell bound to a project folder.
pub struct PtyTerminal {
    cwd: Option<PathBuf>,
    parser: vt100::Parser,
    pty: Option<PtyHandles>,
    /// Bytes arriving from the PTY reader thread.
    rx: Option<Receiver<Vec<u8>>>,
    alive: Arc<AtomicBool>,
    rows: u16,
    cols: u16,
    status: String,
    focused: bool,
}

impl Default for PtyTerminal {
    fn default() -> Self {
        Self {
            cwd: None,
            parser: make_parser(DEFAULT_ROWS, DEFAULT_COLS),
            pty: None,
            rx: None,
            alive: Arc::new(AtomicBool::new(false)),
            rows: DEFAULT_ROWS,
            cols: DEFAULT_COLS,
            status: String::from("Open a project folder to start the shell."),
            focused: false,
        }
    }
}

/// Single-quote a path for POSIX shells (bash/zsh/fish all accept this).
pub fn shell_quote(path: &Path) -> String {
    format!("'{}'", path.display().to_string().replace('\'', "'\\''"))
}

fn make_parser(rows: u16, cols: u16) -> vt100::Parser {
    let rows = NonZeroU16::new(rows.max(2)).unwrap();
    let cols = NonZeroU16::new(cols.max(2)).unwrap();
    vt100::Parser::new(rows, cols, 0)
}

impl PtyTerminal {
    pub fn ensure_running(&mut self, folder: Option<&Path>) {
        let Some(folder) = folder.filter(|p| p.is_dir()) else {
            if self.pty.is_some() {
                self.shutdown();
                self.status = "Open a project folder to start the shell.".to_owned();
            }
            return;
        };

        let same_cwd = self.cwd.as_ref().is_some_and(|c| c == folder);
        if same_cwd && self.pty.is_some() && self.alive.load(Ordering::SeqCst) {
            return;
        }

        if let Err(e) = self.start(folder) {
            self.status = format!("Failed to start shell: {e}");
        }
    }

    /// Force-restart the shell in `folder` (used by project Reset).
    pub fn restart(&mut self, folder: &Path) -> Result<(), String> {
        self.start(folder)
    }

    fn start(&mut self, folder: &Path) -> Result<(), String> {
        self.shutdown();

        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows: self.rows,
                cols: self.cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| e.to_string())?;

        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_owned());
        let mut cmd = CommandBuilder::new(&shell);
        cmd.cwd(folder);
        // Interactive login-ish env; keep it a normal interactive shell.
        cmd.env("TERM", "xterm-256color");

        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| e.to_string())?;

        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| e.to_string())?;
        let writer = pair.master.take_writer().map_err(|e| e.to_string())?;

        let (tx, rx): (Sender<Vec<u8>>, Receiver<Vec<u8>>) = mpsc::channel();
        let alive = Arc::new(AtomicBool::new(true));
        let alive_flag = alive.clone();

        thread::spawn(move || {
            let mut buf = [0u8; 4096];
            while alive_flag.load(Ordering::SeqCst) {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        if tx.send(buf[..n].to_vec()).is_err() {
                            break;
                        }
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5));
                    }
                    Err(_) => break,
                }
            }
            alive_flag.store(false, Ordering::SeqCst);
        });

        // The PTY line discipline buffers this until the shell is ready to
        // read, so it lands after any rc file runs: the prompt always starts
        // in the project folder and `cargo run` works immediately.
        let mut writer = writer;
        let _ = writer.write_all(format!("cd -- {}\r", shell_quote(folder)).as_bytes());
        let _ = writer.flush();

        self.parser = make_parser(self.rows, self.cols);
        self.pty = Some(PtyHandles {
            writer,
            master: pair.master,
            _child: child,
        });
        self.rx = Some(rx);
        self.alive = alive;
        self.cwd = Some(folder.to_path_buf());
        self.status = format!("Shell in {}", folder.display());
        Ok(())
    }

    fn shutdown(&mut self) {
        self.alive.store(false, Ordering::SeqCst);
        self.pty = None;
        self.rx = None;
        self.cwd = None;
    }

    /// Type a command line into the shell as if the user typed it.
    pub fn send_line(&mut self, line: &str) {
        self.write_bytes(format!("{line}\r").as_bytes());
    }

    /// Jump the running shell back to the project folder.
    pub fn cd_to_project(&mut self, folder: &Path) {
        self.send_line(&format!("cd -- {}", shell_quote(folder)));
    }

    fn write_bytes(&mut self, bytes: &[u8]) {
        if let Some(pty) = self.pty.as_mut() {
            let _ = pty.writer.write_all(bytes);
            let _ = pty.writer.flush();
        }
    }

    fn drain_output(&mut self) -> bool {
        let Some(rx) = &self.rx else {
            return false;
        };
        let mut got = false;
        loop {
            match rx.try_recv() {
                Ok(chunk) => {
                    self.parser.process(&chunk);
                    got = true;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    self.alive.store(false, Ordering::SeqCst);
                    break;
                }
            }
        }
        got
    }

    fn resize_if_needed(&mut self, rows: u16, cols: u16) {
        let rows = rows.max(2);
        let cols = cols.max(2);
        if rows == self.rows && cols == self.cols {
            return;
        }
        self.rows = rows;
        self.cols = cols;
        self.parser.set_size(
            NonZeroU16::new(rows).unwrap(),
            NonZeroU16::new(cols).unwrap(),
        );
        if let Some(pty) = &self.pty {
            let _ = pty.master.resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            });
        }
    }

    pub fn show(&mut self, ui: &mut Ui, folder: Option<&Path>, ctx: &egui::Context) {
        self.ensure_running(folder);

        ui.horizontal(|ui| {
            ui.heading("Shell");
            ui.label(egui::RichText::new(&self.status).small().weak());
            if ui
                .add_enabled(folder.is_some_and(|p| p.is_dir()), egui::Button::new("Restart"))
                .clicked()
            {
                if let Some(dir) = folder {
                    let _ = self.start(dir);
                }
            }
            if ui
                .add_enabled(
                    folder.is_some_and(|p| p.is_dir()),
                    egui::Button::new("📁 Project folder"),
                )
                .on_hover_text("cd back to the open project folder")
                .clicked()
            {
                if let Some(dir) = folder {
                    self.cd_to_project(dir);
                }
            }
            if ui
                .add_enabled(
                    folder.is_some_and(|p| p.is_dir()),
                    egui::Button::new("▶ cargo run"),
                )
                .clicked()
            {
                if let Some(dir) = folder {
                    self.cd_to_project(dir);
                    self.send_line("cargo run");
                }
            }
            if ui
                .add_enabled(
                    folder.is_some_and(|p| p.is_dir()),
                    egui::Button::new("🔨 cargo build"),
                )
                .clicked()
            {
                if let Some(dir) = folder {
                    self.cd_to_project(dir);
                    self.send_line("cargo build");
                }
            }
            if ui
                .add_enabled(
                    folder.is_some_and(|p| p.is_dir()),
                    egui::Button::new("Open External Terminal"),
                )
                .clicked()
            {
                if let Some(dir) = folder {
                    match crate::terminal_panel::open_external_terminal(dir) {
                        Ok(msg) => self.status = msg,
                        Err(e) => self.status = format!("Failed: {e}"),
                    }
                }
            }
        });
        ui.separator();

        let font = FontId::monospace(13.0);
        let cell_h = ui.text_style_height(&egui::TextStyle::Monospace).max(12.0);
        let cell_w = (cell_h * 0.62).max(6.0);

        let avail = ui.available_size();
        let cols = ((avail.x / cell_w).floor() as u16).clamp(20, 200);
        let rows = ((avail.y / cell_h).floor() as u16).clamp(8, 80);
        self.resize_if_needed(rows, cols);

        let term_size = Vec2::new(cols as f32 * cell_w, rows as f32 * cell_h);
        let (response, painter) = ui.allocate_painter(term_size, Sense::click_and_drag());

        if response.clicked() {
            response.request_focus();
            self.focused = true;
        }
        if response.gained_focus() {
            self.focused = true;
        }
        if response.lost_focus() {
            self.focused = false;
        }

        // Instant keystrokes → PTY (no command buffering / no loading spinner).
        if response.has_focus() || self.focused {
            let mut out_bytes: Vec<Vec<u8>> = Vec::new();
            ui.input(|i| {
                for event in &i.events {
                    match event {
                        egui::Event::Text(text) => {
                            out_bytes.push(text.as_bytes().to_vec());
                        }
                        egui::Event::Key {
                            key,
                            pressed: true,
                            modifiers,
                            ..
                        } => {
                            if let Some(seq) = key_to_bytes(*key, *modifiers) {
                                out_bytes.push(seq);
                            }
                        }
                        egui::Event::Paste(text) => {
                            out_bytes.push(text.as_bytes().to_vec());
                        }
                        _ => {}
                    }
                }
            });
            for chunk in out_bytes {
                self.write_bytes(&chunk);
            }
        }

        let got_data = self.drain_output();
        if got_data || self.pty.is_some() {
            // Keep reading while the shell is alive.
            ctx.request_repaint_after(Duration::from_millis(16));
        }

        // Paint screen.
        let origin = response.rect.min;
        painter.rect_filled(response.rect, 0.0, Color32::from_rgb(12, 12, 16));

        let screen = self.parser.screen();
        let (s_rows, s_cols) = screen.size();
        for row in 0..s_rows.get() {
            for col in 0..s_cols.get() {
                let Some(cell) = screen.cell(row, col) else {
                    continue;
                };
                let ch = cell.contents();
                if ch.is_empty() || ch == " " {
                    // Still paint bg if non-default.
                    let bg = vt_color_to_egui(cell.bgcolor(), true);
                    if bg != Color32::from_rgb(12, 12, 16) {
                        let r = Rect::from_min_size(
                            Pos2::new(
                                origin.x + col as f32 * cell_w,
                                origin.y + row as f32 * cell_h,
                            ),
                            Vec2::new(cell_w, cell_h),
                        );
                        painter.rect_filled(r, 0.0, bg);
                    }
                    continue;
                }
                let fg = vt_color_to_egui(cell.fgcolor(), false);
                let bg = vt_color_to_egui(cell.bgcolor(), true);
                let pos = Pos2::new(
                    origin.x + col as f32 * cell_w,
                    origin.y + row as f32 * cell_h,
                );
                if bg != Color32::from_rgb(12, 12, 16) {
                    painter.rect_filled(
                        Rect::from_min_size(pos, Vec2::new(cell_w, cell_h)),
                        0.0,
                        bg,
                    );
                }
                painter.text(
                    pos,
                    egui::Align2::LEFT_TOP,
                    ch,
                    font.clone(),
                    fg,
                );
            }
        }

        // Cursor.
        if self.focused || response.has_focus() {
            let (crow, ccol) = screen.cursor_position();
            let cpos = Pos2::new(
                origin.x + ccol as f32 * cell_w,
                origin.y + crow as f32 * cell_h,
            );
            painter.rect_filled(
                Rect::from_min_size(cpos, Vec2::new(cell_w.max(2.0) * 0.15, cell_h * 0.9)),
                0.0,
                Color32::from_rgb(200, 200, 200),
            );
        }

        if !response.has_focus() && !self.focused {
            ui.label(
                egui::RichText::new("Click the terminal to focus and type.")
                    .small()
                    .weak(),
            );
        }
    }
}

fn key_to_bytes(key: egui::Key, modifiers: egui::Modifiers) -> Option<Vec<u8>> {
    use egui::Key;
    // Prefer Text events for printable chars; handle controls here.
    match key {
        Key::Enter => Some(b"\r".to_vec()),
        Key::Backspace => Some(vec![0x7f]),
        Key::Tab => Some(b"\t".to_vec()),
        Key::Escape => Some(b"\x1b".to_vec()),
        Key::ArrowUp => Some(b"\x1b[A".to_vec()),
        Key::ArrowDown => Some(b"\x1b[B".to_vec()),
        Key::ArrowRight => Some(b"\x1b[C".to_vec()),
        Key::ArrowLeft => Some(b"\x1b[D".to_vec()),
        Key::Home => Some(b"\x1b[H".to_vec()),
        Key::End => Some(b"\x1b[F".to_vec()),
        Key::Delete => Some(b"\x1b[3~".to_vec()),
        Key::PageUp => Some(b"\x1b[5~".to_vec()),
        Key::PageDown => Some(b"\x1b[6~".to_vec()),
        Key::C if modifiers.ctrl => Some(vec![0x03]), // Ctrl+C
        Key::D if modifiers.ctrl => Some(vec![0x04]), // Ctrl+D
        Key::L if modifiers.ctrl => Some(vec![0x0c]), // Ctrl+L
        Key::Z if modifiers.ctrl => Some(vec![0x1a]), // Ctrl+Z
        _ => None,
    }
}

fn vt_color_to_egui(color: vt100::Color, is_bg: bool) -> Color32 {
    match color {
        vt100::Color::Default => {
            if is_bg {
                Color32::from_rgb(12, 12, 16)
            } else {
                Color32::from_rgb(220, 220, 220)
            }
        }
        vt100::Color::Idx(i) => ansi_idx(i),
        vt100::Color::Rgb(r, g, b) => Color32::from_rgb(r, g, b),
    }
}

fn ansi_idx(i: u8) -> Color32 {
    match i {
        0 => Color32::from_rgb(0, 0, 0),
        1 => Color32::from_rgb(205, 49, 49),
        2 => Color32::from_rgb(13, 188, 121),
        3 => Color32::from_rgb(229, 229, 16),
        4 => Color32::from_rgb(36, 114, 200),
        5 => Color32::from_rgb(188, 63, 188),
        6 => Color32::from_rgb(17, 168, 205),
        7 => Color32::from_rgb(229, 229, 229),
        8 => Color32::from_rgb(102, 102, 102),
        9 => Color32::from_rgb(241, 76, 76),
        10 => Color32::from_rgb(35, 209, 139),
        11 => Color32::from_rgb(245, 245, 67),
        12 => Color32::from_rgb(59, 142, 234),
        13 => Color32::from_rgb(214, 112, 214),
        14 => Color32::from_rgb(41, 184, 219),
        15 => Color32::from_rgb(255, 255, 255),
        _ => Color32::from_rgb(200, 200, 200),
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_quote_handles_spaces_and_quotes() {
        assert_eq!(shell_quote(Path::new("/tmp/my proj")), "'/tmp/my proj'");
        assert_eq!(shell_quote(Path::new("/tmp/it's")), "'/tmp/it'\\''s'");
    }
}

#[cfg(test)]
mod pty_integration_tests {
    use super::*;

    fn screen_text(term: &mut PtyTerminal) -> String {
        term.drain_output();
        let screen = term.parser.screen();
        let (rows, cols) = screen.size();
        let mut out = String::new();
        for row in 0..rows.get() {
            for col in 0..cols.get() {
                if let Some(cell) = screen.cell(row, col) {
                    out.push_str(cell.contents());
                }
            }
            out.push('\n');
        }
        out
    }

    fn wait_for(term: &mut PtyTerminal, needle: &str, secs: u64) -> String {
        let start = std::time::Instant::now();
        loop {
            let text = screen_text(term);
            if text.contains(needle) || start.elapsed().as_secs() > secs {
                return text;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    #[test]
    fn pty_lands_in_project_folder_and_follows_switches() {
        let base = std::env::temp_dir().join(format!("fd term test {}", std::process::id()));
        let dir_a = base.join("proj a");
        let dir_b = base.join("proj b");
        std::fs::create_dir_all(&dir_a).unwrap();
        std::fs::create_dir_all(&dir_b).unwrap();

        let mut term = PtyTerminal::default();
        // Start: auto-cd must land in a folder whose path contains spaces.
        term.ensure_running(Some(&dir_a));
        term.send_line("pwd");
        let text = wait_for(&mut term, &dir_a.display().to_string(), 10);
        assert!(text.contains(&dir_a.display().to_string()), "start pwd:\n{text}");

        // Wander away, then the Project-folder jump returns.
        term.send_line("cd /");
        term.send_line("pwd");
        let text = wait_for(&mut term, "\n/\n", 5);
        term.cd_to_project(&dir_a);
        term.send_line("pwd");
        let text = wait_for(&mut term, &dir_a.display().to_string(), 5);
        assert!(text.contains(&dir_a.display().to_string()), "cd back:\n{text}");

        // Typed commands execute (what the cargo toolbar buttons send).
        term.send_line("echo MARKER_$((40+2))");
        let text = wait_for(&mut term, "MARKER_42", 5);
        assert!(text.contains("MARKER_42"), "echo:\n{text}");

        // Folder switch restarts the shell in the new folder.
        term.ensure_running(Some(&dir_b));
        term.send_line("pwd");
        let text = wait_for(&mut term, &dir_b.display().to_string(), 10);
        assert!(text.contains(&dir_b.display().to_string()), "switch pwd:\n{text}");

        // Explicit restart also lands in the folder.
        term.restart(&dir_a).unwrap();
        term.send_line("pwd");
        let text = wait_for(&mut term, &dir_a.display().to_string(), 10);
        assert!(text.contains(&dir_a.display().to_string()), "restart pwd:\n{text}");

        term.shutdown();
        let _ = std::fs::remove_dir_all(&base);
    }
}

