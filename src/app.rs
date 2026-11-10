//! Top-level workspace: Design / Code / Terminal / Git tabs with Explorer.

use crate::canvas::{self, CanvasState, PlacementTool};
use crate::code_editor::{self, CodeEditorState};
use crate::explorer::{self, ExplorerState};
use crate::git_panel::{self, GitState};
use crate::inspector;
use crate::model::FormDocument;
use crate::project;
use crate::terminal_panel::{self, TerminalState};
use crate::theme::{self, ACCENT, BG_DEEP, BG_ELEVATED, BORDER, TEXT_MUTED};
use crate::toolbox;
use eframe::egui;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WorkspaceTab {
    Design,
    Code,
    Terminal,
    Git,
}

pub struct FormDesignerApp {
    doc: FormDocument,
    canvas_state: CanvasState,
    placement: Option<PlacementTool>,
    status: String,

    project_folder: Option<PathBuf>,

    tab: WorkspaceTab,
    /// When true, Code tab should refresh the generated preview (no file open).
    code_preview_stale: bool,
    /// In-app confirm: create Cargo.toml / src/main.rs / src/app.rs in this folder.
    pending_create_folder: Option<PathBuf>,
    explorer: ExplorerState,
    code_editor: CodeEditorState,
    terminal: TerminalState,
    git: GitState,
    /// Fingerprint of the last document written to main.rs / app.rs.
    last_synced_doc: Option<String>,
    /// Fingerprint of the last document written to form.json.
    last_saved_form: Option<String>,
    undo_stack: Vec<FormDocument>,
    redo_stack: Vec<FormDocument>,
    /// Doc as of the last recorded history step (selection excluded).
    history_baseline: Option<FormDocument>,
    last_history_change: Option<std::time::Instant>,
    history_gesture_open: bool,
    history_last_seen: Option<String>,
}

impl Default for FormDesignerApp {
    fn default() -> Self {
        let doc = FormDocument::new_blank();
        let mut code_editor = CodeEditorState::default();
        code_editor.show_generated_preview(&doc);
        Self {
            doc,
            canvas_state: CanvasState::default(),
            placement: None,
            status: String::new(),
            project_folder: None,
            tab: WorkspaceTab::Design,
            code_preview_stale: false,
            pending_create_folder: None,
            explorer: ExplorerState::default(),
            code_editor,
            terminal: TerminalState::default(),
            git: GitState::default(),
            last_synced_doc: None,
            last_saved_form: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            history_baseline: None,
            last_history_change: None,
            history_gesture_open: false,
            history_last_seen: None,
        }
    }
}

impl FormDesignerApp {
    fn refresh_explorer(&mut self) {
        self.explorer
            .refresh(self.project_folder.as_deref());
    }

    fn fingerprint_no_selection(doc: &FormDocument) -> String {
        let mut d = doc.clone();
        d.selected = None;
        Self::doc_fingerprint(&d)
    }

    fn record_history(&mut self) {
        let fp = Self::fingerprint_no_selection(&self.doc);
        if self.history_baseline.is_none() {
            self.history_baseline = Some(self.doc.clone());
            self.history_last_seen = Some(fp);
            return;
        }
        // Settle: no change for 700ms closes the current gesture.
        if self.history_gesture_open
            && self.last_history_change.is_some_and(|t| t.elapsed() >= std::time::Duration::from_millis(700))
        {
            self.history_baseline = Some(self.doc.clone());
            self.history_gesture_open = false;
        }
        if self.history_last_seen.as_ref() == Some(&fp) {
            return;
        }
        self.history_last_seen = Some(fp);
        self.last_history_change = Some(std::time::Instant::now());
        if !self.history_gesture_open {
            if let Some(baseline) = self.history_baseline.clone() {
                if Self::fingerprint_no_selection(&baseline) != Self::fingerprint_no_selection(&self.doc) {
                    self.undo_stack.push(baseline);
                    if self.undo_stack.len() > 100 { self.undo_stack.remove(0); }
                    self.redo_stack.clear();
                    self.history_gesture_open = true;
                }
            }
        }
    }

    fn apply_history_doc(&mut self, doc: FormDocument) {
        // Keep stacks; set_doc would clear them, so restore after.
        let undo = std::mem::take(&mut self.undo_stack);
        let redo = std::mem::take(&mut self.redo_stack);
        self.set_doc(doc);
        self.undo_stack = undo;
        self.redo_stack = redo;
        self.history_baseline = Some(self.doc.clone());
        self.history_last_seen = Some(Self::fingerprint_no_selection(&self.doc));
        self.history_gesture_open = false;
        self.last_history_change = None;
    }

    fn undo(&mut self) {
        let Some(prev) = self.undo_stack.pop() else {
            self.status = "Nothing to undo".to_owned();
            return;
        };
        let current = self.doc.clone();
        self.redo_stack.push(current);
        self.status = "Undo".to_owned();
        self.apply_history_doc(prev);
    }

    fn redo(&mut self) {
        let Some(next) = self.redo_stack.pop() else {
            self.status = "Nothing to redo".to_owned();
            return;
        };
        let current = self.doc.clone();
        self.undo_stack.push(current);
        self.status = "Redo".to_owned();
        self.apply_history_doc(next);
    }

    fn set_doc(&mut self, doc: FormDocument) {
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.history_last_seen = Some(Self::fingerprint_no_selection(&doc));
        self.history_baseline = Some(doc.clone());
        self.history_gesture_open = false;
        self.last_history_change = None;
        self.doc = doc;
        self.canvas_state = CanvasState::default();
        self.placement = None;
        self.code_preview_stale = true;
        if self.code_editor.open_path.is_none() {
            self.code_editor.show_generated_preview(&self.doc);
            self.code_preview_stale = false;
        }
    }

    /// Reload the whole open project from disk (form, explorer, code, shell).
    fn reload_project(&mut self) {
        let Some(folder) = self.project_folder.clone() else {
            self.status = "Open a project folder first".to_owned();
            return;
        };
        if !folder.is_dir() {
            self.status = format!("Invalid folder: {}", folder.display());
            return;
        }

        match project::load_form_if_present(&folder) {
            Ok(Some(doc)) => self.set_doc(doc),
            Ok(None) => {
                self.set_doc(FormDocument::new_blank());
                self.status = format!(
                    "No {} found — reset to a blank form",
                    project::FORM_FILE_NAME
                );
            }
            Err(e) => {
                self.status = format!("Reload failed: {e}");
                return;
            }
        }

        self.placement = None;
        self.canvas_state = CanvasState::default();
        self.pending_create_folder = None;
        self.git.status.clear();
        self.refresh_explorer();

        // Re-open code from disk if a file was open; otherwise refresh generated preview.
        if let Some(path) = self.code_editor.open_path.clone() {
            if let Err(e) = self.code_editor.open_file(path) {
                self.code_editor.show_generated_preview(&self.doc);
                self.status = format!("Project reloaded (code file: {e})");
            }
        } else {
            self.code_editor.show_generated_preview(&self.doc);
            self.code_preview_stale = false;
        }

        // Restart the PTY shell in the project folder.
        if let Err(e) = self.terminal.shell.restart(&folder) {
            self.status = format!("Project reloaded (shell: {e})");
        } else if !self.status.starts_with("No ") {
            self.status = format!("Project reloaded from {}", folder.display());
        }

        self.tab = WorkspaceTab::Design;
    }

    fn open_folder(&mut self) {
        let mut dialog = rfd::FileDialog::new().set_title("Open Folder");
        if let Some(ref current) = self.project_folder {
            dialog = dialog.set_directory(current);
        }

        let Some(folder) = dialog.pick_folder() else {
            self.status = "Open Folder cancelled".to_owned();
            return;
        };

        self.project_folder = Some(folder.clone());
        self.pending_create_folder = None;

        if project::has_rust_sources(&folder) {
            match project::load_form_if_present(&folder) {
                Ok(Some(doc)) => {
                    self.set_doc(doc);
                    self.status = format!(
                        "Loaded project {} ({})",
                        folder.display(),
                        project::FORM_FILE_NAME
                    );
                }
                Ok(None) => {
                    // Start blank instead of keeping the previous project's
                    // design, which would otherwise be saved into this folder.
                    self.set_doc(FormDocument::new_blank());
                    self.status = format!(
                        "Loaded project {} (no {} — edits autosave to it)",
                        folder.display(),
                        project::FORM_FILE_NAME
                    );
                }
                Err(e) => {
                    self.status = e;
                }
            }
            self.mark_doc_synced();
            self.refresh_explorer();
            self.code_editor.show_generated_preview(&self.doc);
        } else {
            // Ask in-app (rfd Yes/No is unreliable on some Linux portals).
            if let Ok(Some(doc)) = project::load_form_if_present(&folder) {
                self.set_doc(doc);
            }
            self.mark_doc_synced();
            self.pending_create_folder = Some(folder.clone());
            self.status = format!(
                "Opened {} — no Rust sources found; confirm to create project files",
                folder.display()
            );
            self.refresh_explorer();
            self.code_editor.show_generated_preview(&self.doc);
        }
    }

    fn write_project_files(&mut self, folder: &std::path::Path) {
        match project::create_cargo_project(folder, &self.doc) {
            Ok(paths) => {
                let list = paths
                    .iter()
                    .map(|p| p.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                self.status = format!("Wrote project files: {list}");
                self.mark_doc_synced();
                self.refresh_explorer();
                self.code_preview_stale = true;
                // Open main.rs in the Code tab so the user sees the saved file.
                let main_rs = folder.join("src/main.rs");
                if let Err(e) = self.code_editor.open_file(main_rs) {
                    self.code_editor.show_generated_preview(&self.doc);
                    self.status = format!("{}; open file: {e}", self.status);
                } else {
                    self.tab = WorkspaceTab::Code;
                }
            }
            Err(e) => {
                self.status = format!("Failed to write project files: {e}");
            }
        }
    }

    fn doc_fingerprint(doc: &FormDocument) -> String {
        doc.to_json().unwrap_or_else(|_| format!("{doc:?}"))
    }

    fn mark_doc_synced(&mut self) {
        let fp = Self::doc_fingerprint(&self.doc);
        self.last_synced_doc = Some(fp.clone());
        self.last_saved_form = Some(fp);
    }

    fn is_dirty(&self) -> bool {
        self.project_folder.is_some()
            && self.last_saved_form.as_ref() != Some(&Self::doc_fingerprint(&self.doc))
    }

    /// Autosave: persist form.json whenever the design changed.
    /// Runs even before Cargo sources exist, so work in a freshly opened
    /// folder is never lost by closing the app without pressing Save.
    fn autosave_form(&mut self) {
        let Some(folder) = self.project_folder.clone() else {
            return;
        };
        if !folder.is_dir() || !self.is_dirty() {
            return;
        }
        match project::save_form(&folder, &self.doc) {
            Ok(_) => {
                self.last_saved_form = Some(Self::doc_fingerprint(&self.doc));
            }
            Err(e) => self.status = format!("Autosave failed: {e}"),
        }
    }

    /// Whenever the form/document changes, regenerate main.rs and app.rs together.
    fn sync_sources_on_form_change(&mut self) {
        let Some(folder) = self.project_folder.clone() else {
            return;
        };
        if !folder.is_dir() {
            return;
        }
        // Only sync once the project already has generated sources.
        if !project::has_rust_sources(&folder) && !folder.join("Cargo.toml").is_file() {
            return;
        }

        let fingerprint = Self::doc_fingerprint(&self.doc);
        if self.last_synced_doc.as_ref() == Some(&fingerprint) {
            return;
        }

        match project::write_generated_sources(&folder, &self.doc) {
            Ok(_) => {
                self.last_synced_doc = Some(fingerprint);
                self.code_preview_stale = true;
                if self.code_editor.open_path.is_none() {
                    self.code_editor.show_generated_preview(&self.doc);
                    self.code_preview_stale = false;
                } else if let Some(path) = self.code_editor.open_path.clone() {
                    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    if name == "main.rs" || name == "app.rs" {
                        let _ = self.code_editor.open_file(path);
                    }
                }
                self.refresh_explorer();
                self.status = format!(
                    "Synced main.rs + app.rs (\"{}\", {}×{})",
                    self.doc.title, self.doc.width, self.doc.height
                );
            }
            Err(e) => {
                self.status = format!("Could not sync sources: {e}");
            }
        }
    }

    fn ensure_project_folder(&mut self) -> bool {
        if self.project_folder.as_ref().is_some_and(|p| p.is_dir()) {
            return true;
        }
        let Some(folder) = rfd::FileDialog::new()
            .set_title("Choose Folder to Save Project")
            .pick_folder()
        else {
            self.status = "Save cancelled".to_owned();
            return false;
        };
        self.project_folder = Some(folder);
        true
    }

    /// Save everything: unsaved Code-tab file first (so regeneration below
    /// cannot discard it), then form.json + src/main.rs + src/app.rs
    /// (Cargo.toml only if missing).
    fn save(&mut self) {
        if !self.ensure_project_folder() {
            return;
        }
        let folder = self.project_folder.clone().expect("folder set");
        let mut saved_code = false;
        if self.code_editor.dirty && self.code_editor.open_path.is_some() {
            match self.code_editor.save() {
                Ok(()) => saved_code = true,
                Err(e) => {
                    self.status = format!("Save failed (code file): {e}");
                    return;
                }
            }
        }
        self.write_project_files(&folder);
        if saved_code && !self.status.starts_with("Failed") {
            self.status = format!("Saved everything (incl. code file): {}", self.status);
        }
    }

    fn show_create_project_modal(&mut self, ctx: &egui::Context) {
        let Some(folder) = self.pending_create_folder.clone() else {
            return;
        };

        let mut open = true;
        egui::Modal::new(egui::Id::unique("create_project_modal")).show(ctx, |ui| {
            ui.heading("Create Cargo project?");
            ui.add_space(6.0);
            ui.label("No Rust files found in:");
            ui.monospace(folder.display().to_string());
            ui.add_space(4.0);
            ui.label("Create a new Cargo project here?");
            ui.label(
                egui::RichText::new(
                    "Creates Cargo.toml (eframe/egui) once, plus src/main.rs and src/app.rs.",
                )
                .small()
                .weak(),
            );
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if ui.button("Create project files").clicked() {
                    self.write_project_files(&folder);
                    self.pending_create_folder = None;
                    open = false;
                }
                if ui.button("Not now").clicked() {
                    self.status = format!(
                        "Opened {} without creating project files (use File → Save later)",
                        folder.display()
                    );
                    self.pending_create_folder = None;
                    open = false;
                }
            });
        });

        if !open {
            // already cleared above
        }
    }

    fn window_title(&self) -> String {
        match &self.project_folder {
            Some(folder) => {
                let name = folder
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("project");
                format!("Form Designer — {name}{}", if self.is_dirty() { " •" } else { "" })
            }
            None => "Form Designer".to_owned(),
        }
    }

    fn open_rs_file(&mut self, path: PathBuf) {
        match self.code_editor.open_file(path) {
            Ok(()) => {
                self.tab = WorkspaceTab::Code;
                self.status = "Opened file in Code tab".to_owned();
            }
            Err(e) => self.status = e,
        }
    }
}

impl eframe::App for FormDesignerApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(self.window_title()));
        self.show_create_project_modal(&ctx);
        if ctx.input_mut(|i| {
            i.consume_key(egui::Modifiers::COMMAND, egui::Key::S)
        }) {
            self.save();
        }
        // When a text field has focus, egui's own text undo handles
        // Ctrl+Z/Ctrl+Shift+Z; only do form-level undo otherwise.
        let text_focused = ctx.memory(|m| m.focused().is_some());
        if !text_focused {
            let (undo, redo) = ctx.input_mut(|i| {
                (
                    i.consume_key(egui::Modifiers::COMMAND, egui::Key::Z),
                    i.consume_key(
                        egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
                        egui::Key::Z,
                    ),
                )
            });
            if redo {
                self.redo();
            } else if undo {
                self.undo();
            }
        }
        self.autosave_form();
        self.sync_sources_on_form_change();

        // —— Menu bar ——
        egui::Panel::top("menu_bar").show(ui, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("New Form").clicked() {
                        self.set_doc(FormDocument::new_blank());
                        self.status = "New form".to_owned();
                        ui.close();
                    }
                    if ui.button("Open Folder…").clicked() {
                        ui.close();
                        self.open_folder();
                    }
                    ui.separator();
                    if ui.button("Save").clicked() {
                        ui.close();
                        self.save();
                    }
                    if ui
                        .add_enabled(
                            self.project_folder.is_some(),
                            egui::Button::new("Write Project Files"),
                        )
                        .on_hover_text(
                            "Regenerate src/main.rs and src/app.rs (Cargo.toml only if missing)",
                        )
                        .clicked()
                    {
                        ui.close();
                        if let Some(folder) = self.project_folder.clone() {
                            self.write_project_files(&folder);
                        }
                    }
                    if ui
                        .add_enabled(
                            self.project_folder.is_some(),
                            egui::Button::new(format!("Reload {}", project::FORM_FILE_NAME)),
                        )
                        .clicked()
                    {
                        ui.close();
                        if let Some(folder) = self.project_folder.clone() {
                            match project::load_form_if_present(&folder) {
                                Ok(Some(doc)) => {
                                    self.set_doc(doc);
                                    self.status = format!("Reloaded {}", project::FORM_FILE_NAME);
                                }
                                Ok(None) => {
                                    self.status =
                                        format!("No {} in folder", project::FORM_FILE_NAME);
                                }
                                Err(e) => self.status = e,
                            }
                        }
                    }
                    ui.separator();
                    if ui.button("Quit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
                ui.menu_button("Edit", |ui| {
                    if ui
                        .add_enabled(!self.undo_stack.is_empty(), egui::Button::new("Undo (Ctrl+Z)"))
                        .clicked()
                    {
                        self.undo();
                        ui.close();
                    }
                    if ui
                        .add_enabled(!self.redo_stack.is_empty(), egui::Button::new("Redo (Ctrl+Shift+Z)"))
                        .clicked()
                    {
                        self.redo();
                        ui.close();
                    }
                    ui.separator();
                    if ui
                        .add_enabled(self.doc.selected.is_some(), egui::Button::new("Delete"))
                        .clicked()
                    {
                        self.doc.delete_selected();
                        ui.close();
                    }
                    if ui.button("Deselect").clicked() {
                        self.doc.selected = None;
                        self.placement = None;
                        ui.close();
                    }
                });
                ui.menu_button("View", |ui| {
                    ui.checkbox(&mut self.doc.show_grid, "Show grid");
                    ui.checkbox(&mut self.doc.snap_to_grid, "Snap to grid");
                    if ui
                        .selectable_label(self.tab == WorkspaceTab::Design, "Design tab")
                        .clicked()
                    {
                        self.tab = WorkspaceTab::Design;
                        ui.close();
                    }
                    if ui
                        .selectable_label(self.tab == WorkspaceTab::Code, "Code tab")
                        .clicked()
                    {
                        self.tab = WorkspaceTab::Code;
                        ui.close();
                    }
                    if ui
                        .selectable_label(self.tab == WorkspaceTab::Terminal, "Terminal tab")
                        .clicked()
                    {
                        self.tab = WorkspaceTab::Terminal;
                        ui.close();
                    }
                    let git_ok = self.project_folder.is_some();
                    if git_ok {
                        if ui
                            .selectable_label(self.tab == WorkspaceTab::Git, "Git tab")
                            .clicked()
                        {
                            self.tab = WorkspaceTab::Git;
                            ui.close();
                        }
                    } else {
                        ui.add_enabled(false, egui::Button::new("Git tab"))
                            .on_disabled_hover_text("Open a project folder first");
                    }
                });

                ui.separator();
                ui.add_enabled_ui(self.project_folder.is_some(), |ui| {
                    if theme::primary_button(ui, "↺ Reset")
                        .on_hover_text("Reload the whole project from disk")
                        .clicked()
                    {
                        self.reload_project();
                    }
                });
            });
        });

        // If Git was selected but the folder closed, leave the tab.
        let folder_open = self.project_folder.as_ref().is_some_and(|p| p.is_dir());
        if self.tab == WorkspaceTab::Git && !folder_open {
            self.tab = WorkspaceTab::Design;
        }

        // —— Workspace tabs ——
        egui::Panel::top("workspace_tabs")
            .frame(
                egui::Frame::new()
                    .fill(BG_ELEVATED)
                    .stroke(egui::Stroke::new(1.0, BORDER))
                    .inner_margin(egui::Margin::symmetric(12, 8)),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("FORM DESIGNER")
                            .small()
                            .strong()
                            .color(ACCENT),
                    );
                    ui.add_space(12.0);
                    if theme::tab_button(ui, self.tab == WorkspaceTab::Design, " Design ").clicked()
                    {
                        self.tab = WorkspaceTab::Design;
                    }
                    if theme::tab_button(ui, self.tab == WorkspaceTab::Code, " Code ").clicked() {
                        self.tab = WorkspaceTab::Code;
                        self.code_preview_stale = true;
                    }
                    if theme::tab_button(ui, self.tab == WorkspaceTab::Terminal, " Terminal ")
                        .clicked()
                    {
                        self.tab = WorkspaceTab::Terminal;
                    }
                    ui.add_enabled_ui(folder_open, |ui| {
                        if theme::tab_button(ui, self.tab == WorkspaceTab::Git, " Git ").clicked() {
                            self.tab = WorkspaceTab::Git;
                        }
                    });
                });
            });

        // —— Explorer (always when useful) ——
        egui::Panel::left("explorer")
            .default_size(220.0)
            .resizable(true)
            .frame(theme::card_frame())
            .show(ui, |ui| {
                let action =
                    explorer::show(ui, self.project_folder.as_deref(), &mut self.explorer);
                if let Some(path) = action.open_rs {
                    self.open_rs_file(path);
                }
            });

        // —— Tab-specific side panels + center ——
        match self.tab {
            WorkspaceTab::Design => {
                egui::Panel::left("toolbox")
                    .default_size(180.0)
                    .resizable(true)
                    .frame(theme::card_frame())
                    .show(ui, |ui| {
                        toolbox::show(ui, &mut self.placement);
                    });

                egui::Panel::right("inspector")
                    .default_size(260.0)
                    .resizable(true)
                    .frame(theme::card_frame())
                    .show(ui, |ui| {
                        inspector::show(ui, &mut self.doc);
                    });

                egui::Panel::bottom("status_bar")
                    .frame(
                        egui::Frame::new()
                            .fill(BG_ELEVATED)
                            .stroke(egui::Stroke::new(1.0, BORDER))
                            .inner_margin(egui::Margin::symmetric(12, 6)),
                    )
                    .show(ui, |ui| {
                        self.draw_status(ui);
                    });

                egui::CentralPanel::default()
                    .frame(
                        egui::Frame::new()
                            .fill(BG_DEEP)
                            .inner_margin(egui::Margin::same(10)),
                    )
                    .show(ui, |ui| {
                        let resp = canvas::show(
                            ui,
                            &mut self.doc,
                            &mut self.canvas_state,
                            &mut self.placement,
                        );
                        if resp.request_repaint {
                            ctx.request_repaint();
                        }
                    });
            }
            WorkspaceTab::Code => {
                if self.code_editor.open_path.is_none() && self.code_preview_stale {
                    self.code_editor.show_generated_preview(&self.doc);
                    self.code_preview_stale = false;
                }
                egui::Panel::bottom("status_bar")
                    .frame(status_frame())
                    .show(ui, |ui| {
                        self.draw_status(ui);
                    });
                egui::CentralPanel::default()
                    .frame(center_frame())
                    .show(ui, |ui| {
                        code_editor::show(
                            ui,
                            &mut self.code_editor,
                            &self.doc,
                            self.project_folder.as_deref(),
                        );
                    });
            }
            WorkspaceTab::Terminal => {
                egui::Panel::bottom("status_bar")
                    .frame(status_frame())
                    .show(ui, |ui| {
                        self.draw_status(ui);
                    });
                egui::CentralPanel::default()
                    .frame(center_frame())
                    .show(ui, |ui| {
                        terminal_panel::show(
                            ui,
                            self.project_folder.as_ref(),
                            &mut self.terminal,
                            &ctx,
                        );
                    });
            }
            WorkspaceTab::Git => {
                egui::Panel::bottom("status_bar")
                    .frame(status_frame())
                    .show(ui, |ui| {
                        self.draw_status(ui);
                    });
                egui::CentralPanel::default()
                    .frame(center_frame())
                    .show(ui, |ui| {
                        git_panel::show(ui, self.project_folder.as_deref(), &mut self.git);
                    });
            }
        }
        self.record_history();
        self.autosave_form();
    }
}

fn status_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(BG_ELEVATED)
        .stroke(egui::Stroke::new(1.0, BORDER))
        .inner_margin(egui::Margin::symmetric(12, 6))
}

fn center_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(BG_DEEP)
        .inner_margin(egui::Margin::same(12))
}

impl FormDesignerApp {
    fn draw_status(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if let Some(folder) = &self.project_folder {
                ui.label(
                    egui::RichText::new("●")
                        .color(theme::SUCCESS)
                        .small(),
                );
                ui.label(
                    egui::RichText::new(folder.display().to_string())
                        .monospace()
                        .small()
                        .color(TEXT_MUTED),
                );
                ui.label(
                    egui::RichText::new(if self.is_dirty() { "● unsaved" } else { "✓ saved" })
                        .small()
                        .color(if self.is_dirty() { egui::Color32::YELLOW } else { TEXT_MUTED }),
                );
                ui.separator();
            } else {
                ui.label(egui::RichText::new("○ no folder").small().color(TEXT_MUTED));
                ui.separator();
            }
            if self.status.is_empty() {
                ui.label(egui::RichText::new("Ready").color(TEXT_MUTED));
            } else {
                ui.label(egui::RichText::new(&self.status).color(egui::Color32::WHITE));
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ControlKind;

    #[test]
    fn undo_redo_restores_controls() {
        let mut app = FormDesignerApp::default();
        app.record_history();
        app.doc.add_control(ControlKind::Button, 10.0, 10.0);
        app.record_history();
        assert_eq!(app.doc.controls.len(), 1);
        app.undo();
        assert_eq!(app.doc.controls.len(), 0, "undo should remove the added control");
        app.redo();
        assert_eq!(app.doc.controls.len(), 1, "redo should restore it");
    }
}
