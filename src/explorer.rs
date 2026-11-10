//! Explorer panel: lists project files; opens `.rs` files in the Code tab.

use crate::project;
use crate::theme::{self, ACCENT_SOFT, BG_ELEVATED, BORDER, TEXT_MUTED};
use egui::{Color32, CornerRadius, RichText, Stroke, Ui};
use std::path::{Path, PathBuf};

pub struct ExplorerState {
    pub files: Vec<PathBuf>,
    pub selected: Option<PathBuf>,
}

impl Default for ExplorerState {
    fn default() -> Self {
        Self {
            files: Vec::new(),
            selected: None,
        }
    }
}

impl ExplorerState {
    pub fn refresh(&mut self, folder: Option<&Path>) {
        self.files = folder
            .map(project::list_project_files)
            .unwrap_or_default();
        if let Some(sel) = &self.selected {
            if !self.files.iter().any(|f| f == sel) {
                self.selected = None;
            }
        }
    }
}

pub struct ExplorerAction {
    pub open_rs: Option<PathBuf>,
}

pub fn show(ui: &mut Ui, folder: Option<&Path>, state: &mut ExplorerState) -> ExplorerAction {
    let mut action = ExplorerAction { open_rs: None };

    theme::section_title(ui, "Explorer");

    let Some(root) = folder else {
        ui.label(
            RichText::new("Open a folder to browse project files.")
                .color(TEXT_MUTED)
                .italics(),
        );
        return action;
    };

    ui.horizontal(|ui| {
        ui.label(
            RichText::new(shorten_path(&root.display().to_string(), 28))
                .small()
                .monospace()
                .color(TEXT_MUTED),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .add(
                    egui::Button::new(RichText::new("↻").color(Color32::WHITE))
                        .fill(BG_ELEVATED)
                        .stroke(Stroke::new(1.0, BORDER))
                        .corner_radius(CornerRadius::same(6))
                        .min_size(egui::vec2(28.0, 22.0)),
                )
                .on_hover_text("Refresh")
                .clicked()
            {
                state.refresh(Some(root));
            }
        });
    });
    ui.add_space(6.0);

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            if state.files.is_empty() {
                ui.label(RichText::new("(empty project)").color(TEXT_MUTED));
            }
            for path in state.files.clone() {
                let label = project::relative_display(root, &path);
                let is_rs = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .is_some_and(|e| e.eq_ignore_ascii_case("rs"));
                let selected = state.selected.as_ref() == Some(&path);

                let icon = if is_rs {
                    "🦀"
                } else if label.ends_with("Cargo.toml") {
                    "📦"
                } else if label.ends_with(".json") {
                    "🗒"
                } else {
                    "📄"
                };

                let fill = if selected {
                    Color32::from_rgba_unmultiplied(99, 102, 241, 45)
                } else {
                    Color32::TRANSPARENT
                };
                let frame = egui::Frame::new()
                    .fill(fill)
                    .stroke(if selected {
                        Stroke::new(1.0, ACCENT_SOFT)
                    } else {
                        Stroke::NONE
                    })
                    .corner_radius(CornerRadius::same(6))
                    .inner_margin(egui::Margin::symmetric(8, 4));

                let resp = frame
                    .show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        ui.label(
                            RichText::new(format!("{icon}  {label}"))
                                .color(if selected {
                                    Color32::WHITE
                                } else if is_rs {
                                    Color32::from_rgb(203, 213, 225)
                                } else {
                                    TEXT_MUTED
                                }),
                        );
                    })
                    .response
                    .interact(egui::Sense::click());

                if resp.clicked() {
                    state.selected = Some(path.clone());
                    if is_rs {
                        action.open_rs = Some(path);
                    }
                }
                ui.add_space(2.0);
            }
        });

    action
}

fn shorten_path(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_owned()
    } else {
        let tail: String = s.chars().rev().take(max.saturating_sub(1)).collect();
        format!("…{}", tail.chars().rev().collect::<String>())
    }
}
