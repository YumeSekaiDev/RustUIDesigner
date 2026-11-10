//! Terminal tab: real PTY shell opened in the project folder.

use crate::pty_term::PtyTerminal;
use egui::Ui;
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct TerminalState {
    pub shell: PtyTerminal,
}

impl Default for TerminalState {
    fn default() -> Self {
        Self {
            shell: PtyTerminal::default(),
        }
    }
}

/// Launch the OS terminal with working directory set to `folder`.
pub fn open_external_terminal(folder: &Path) -> Result<String, String> {
    if !folder.is_dir() {
        return Err(format!("Not a directory: {}", folder.display()));
    }

    let candidates: &[(&str, &[&str])] = &[
        ("konsole", &["--workdir"]),
        ("gnome-terminal", &["--working-directory"]),
        ("xfce4-terminal", &["--working-directory"]),
        ("mate-terminal", &["--working-directory"]),
        ("tilix", &["--working-directory"]),
        ("kitty", &["--directory"]),
        ("alacritty", &["--working-directory"]),
        ("x-terminal-emulator", &[]),
    ];

    let mut last_err = String::from("No terminal emulator found");
    for &(bin, args) in candidates {
        let mut cmd = Command::new(bin);
        if args.is_empty() {
            cmd.current_dir(folder);
        } else {
            cmd.args(args).arg(folder);
        }
        match cmd.spawn() {
            Ok(_) => return Ok(format!("Opened {bin} in {}", folder.display())),
            Err(e) => last_err = format!("{bin}: {e}"),
        }
    }

    match Command::new("xdg-terminal-exec")
        .current_dir(folder)
        .spawn()
    {
        Ok(_) => Ok(format!(
            "Opened xdg-terminal-exec in {}",
            folder.display()
        )),
        Err(_) => Err(last_err),
    }
}

pub fn show(
    ui: &mut Ui,
    folder: Option<&PathBuf>,
    state: &mut TerminalState,
    ctx: &egui::Context,
) {
    state.shell.show(ui, folder.map(|p| p.as_path()), ctx);
}
