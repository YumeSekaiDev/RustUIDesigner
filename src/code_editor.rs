//! Code view: open project `.rs` files, or show generated main.rs / app.rs preview.
//! Includes lightweight keyword auto-complete for `.rs` editing (no extra crates).

use crate::codegen;
use crate::model::FormDocument;
use egui::text::{LayoutJob, TextFormat};
use egui::{Color32, FontId, RichText, TextEdit, Ui};
use std::path::{Path, PathBuf};

/// Common Rust + egui tokens for simple completion.
const KEYWORDS: &[&str] = &[
    // Rust
    "as",
    "async",
    "await",
    "break",
    "const",
    "continue",
    "crate",
    "dyn",
    "else",
    "enum",
    "extern",
    "false",
    "fn",
    "for",
    "if",
    "impl",
    "in",
    "let",
    "loop",
    "match",
    "mod",
    "move",
    "mut",
    "pub",
    "ref",
    "return",
    "self",
    "Self",
    "static",
    "struct",
    "super",
    "trait",
    "true",
    "type",
    "unsafe",
    "use",
    "where",
    "while",
    "String",
    "Vec",
    "Option",
    "Result",
    "Some",
    "None",
    "Ok",
    "Err",
    "Default",
    "Clone",
    "Debug",
    "Box",
    "Arc",
    "Rc",
    // egui / eframe
    "ui",
    "ctx",
    "egui",
    "eframe",
    "App",
    "Frame",
    "Context",
    "Ui",
    "Button",
    "TextEdit",
    "Label",
    "RichText",
    "Checkbox",
    "RadioButton",
    "Slider",
    "DragValue",
    "ComboBox",
    "CentralPanel",
    "SidePanel",
    "TopBottomPanel",
    "Window",
    "ScrollArea",
    "Grid",
    "Layout",
    "Align",
    "Align2",
    "Vec2",
    "Pos2",
    "Rect",
    "Sense",
    "Response",
    "Color32",
    "Stroke",
    "FontId",
    "TextStyle",
    "Id",
    "ViewportBuilder",
    "NativeOptions",
    "run_native",
    "selectable_label",
    "selectable_value",
    "horizontal",
    "vertical",
    "add_sized",
    "available_size",
    "allocate_ui_at_rect",
];

pub struct CodeEditorState {
    pub open_path: Option<PathBuf>,
    pub buffer: String,
    pub dirty: bool,
    pub status: String,
    /// Char range of the word under the cursor (for highlight), if any.
    highlight_range: Option<std::ops::Range<usize>>,
}

impl Default for CodeEditorState {
    fn default() -> Self {
        Self {
            open_path: None,
            buffer: String::new(),
            dirty: false,
            status: String::new(),
            highlight_range: None,
        }
    }
}

impl CodeEditorState {
    pub fn open_file(&mut self, path: PathBuf) -> Result<(), String> {
        let text = std::fs::read_to_string(&path).map_err(|e| format!("Read failed: {e}"))?;
        self.open_path = Some(path);
        self.buffer = text;
        self.dirty = false;
        self.status.clear();
        self.highlight_range = None;
        Ok(())
    }

    pub fn show_generated_preview(&mut self, doc: &FormDocument) {
        self.open_path = None;
        self.buffer = codegen::generate_preview(doc);
        self.dirty = false;
        self.status = "Showing generated main.rs + app.rs preview".to_owned();
        self.highlight_range = None;
    }

    pub fn save(&mut self) -> Result<(), String> {
        let Some(path) = &self.open_path else {
            return Err("No file open to save".to_owned());
        };
        std::fs::write(path, &self.buffer).map_err(|e| format!("Write failed: {e}"))?;
        self.dirty = false;
        self.status = format!("Saved {}", path.display());
        Ok(())
    }

    fn is_rs_file(&self) -> bool {
        self.open_path
            .as_ref()
            .and_then(|p| p.extension())
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("rs"))
    }
}

fn is_ident_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

fn rust_word_boundary(c: char) -> bool {
    !is_ident_char(c)
}

fn suggest_keywords(word: &str) -> Vec<egui::Suggestion> {
    if word.is_empty() {
        return Vec::new();
    }
    let mut matches: Vec<&str> = KEYWORDS
        .iter()
        .copied()
        .filter(|kw| kw.starts_with(word) && *kw != word)
        .collect();
    // Prefer shorter / exact-prefix Rust keywords first.
    matches.sort_by_key(|kw| (kw.len(), *kw));
    matches.truncate(12);
    matches
        .into_iter()
        .map(|kw| {
            let kind = if kw.chars().next().is_some_and(|c| c.is_uppercase()) {
                "type"
            } else if matches!(
                kw,
                "fn" | "let" | "mut" | "pub" | "struct" | "impl" | "use" | "mod" | "match" | "if"
            ) {
                "keyword"
            } else {
                "ident"
            };
            egui::Suggestion::new(kw.to_owned()).description(kind)
        })
        .collect()
}

/// Layout the buffer, highlighting the current identifier under the cursor.
fn highlight_layouter(
    ui: &Ui,
    text: &str,
    wrap_width: f32,
    highlight: Option<std::ops::Range<usize>>,
) -> std::sync::Arc<egui::Galley> {
    let font_id = FontId::monospace(13.0);
    let normal = TextFormat {
        font_id: font_id.clone(),
        color: ui.visuals().text_color(),
        ..Default::default()
    };
    let highlighted = TextFormat {
        font_id,
        color: ui.visuals().strong_text_color(),
        background: Color32::from_rgba_unmultiplied(80, 140, 255, 60),
        ..Default::default()
    };

    let mut job = LayoutJob::default();
    job.wrap.max_width = wrap_width;

    if let Some(range) = highlight.filter(|r| r.start <= r.end && r.end <= text.len()) {
        if range.start > 0 {
            job.append(&text[..range.start], 0.0, normal.clone());
        }
        job.append(&text[range.start..range.end], 0.0, highlighted);
        if range.end < text.len() {
            job.append(&text[range.end..], 0.0, normal);
        }
    } else {
        job.append(text, 0.0, normal);
    }

    ui.ctx().fonts_mut(|f| f.layout_job(job))
}

pub fn show(ui: &mut Ui, state: &mut CodeEditorState, doc: &FormDocument, folder: Option<&Path>) {
    ui.horizontal(|ui| {
        ui.heading("Code");
        if let Some(path) = &state.open_path {
            let label = folder
                .and_then(|root| path.strip_prefix(root).ok())
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| path.display().to_string());
            ui.label(RichText::new(label).monospace());
            if state.dirty {
                ui.colored_label(egui::Color32::YELLOW, "• modified");
            }
            if state.is_rs_file() {
                ui.label(RichText::new("autocomplete on").small().weak());
            }
        } else {
            ui.label(RichText::new("Generated preview").weak());
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .add_enabled(state.open_path.is_some(), egui::Button::new("Save File"))
                .clicked()
            {
                match state.save() {
                    Ok(()) => {}
                    Err(e) => state.status = e,
                }
            }
            if ui.button("Show Generated").clicked() {
                state.show_generated_preview(doc);
            }
            if ui.button("Copy").clicked() {
                ui.ctx().copy_text(state.buffer.clone());
                state.status = "Copied to clipboard".to_owned();
            }
        });
    });

    if !state.status.is_empty() {
        ui.label(RichText::new(&state.status).small().weak());
    }

    ui.add_space(4.0);

    let before = state.buffer.clone();
    let enable_completion = state.is_rs_file();
    let highlight = state.highlight_range.clone();

    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let mut layouter = |ui: &egui::Ui, text: &dyn egui::TextBuffer, wrap_width: f32| {
                highlight_layouter(ui, text.as_str(), wrap_width, highlight.clone())
            };

            let text_edit = TextEdit::multiline(&mut state.buffer)
                .code_editor()
                .desired_width(f32::INFINITY)
                .desired_rows(24)
                .font(egui::TextStyle::Monospace)
                .layouter(&mut layouter);

            if enable_completion {
                let id = ui.make_persistent_id(("code_completion", state.open_path.as_ref()));
                let output = egui::CompletionPopup::new(id)
                    .word_boundary(rust_word_boundary)
                    .max_height(180.0)
                    .align(egui::RectAlign::BOTTOM_START)
                    .show(ui, text_edit, |query| suggest_keywords(query.word));

                state.highlight_range =
                    current_word_byte_range(&state.buffer, output.text_edit.cursor_range);

                if let Some(accepted) = output.accepted {
                    state.status = format!("Inserted `{}`", accepted.insert);
                }
            } else {
                ui.add(text_edit);
                state.highlight_range = None;
            }
        });

    if state.buffer != before && state.open_path.is_some() {
        state.dirty = true;
    }
}

fn current_word_byte_range(
    text: &str,
    cursor: Option<egui::text::CCursorRange>,
) -> Option<std::ops::Range<usize>> {
    let cursor = cursor?;
    let primary = cursor.primary.index.0;
    // Convert char index to byte index.
    let byte_cursor: usize = text.chars().take(primary).map(|c| c.len_utf8()).sum();
    if byte_cursor > text.len() {
        return None;
    }

    let before = &text[..byte_cursor];
    let start = before
        .char_indices()
        .rev()
        .find(|(_, c)| rust_word_boundary(*c))
        .map(|(i, c)| i + c.len_utf8())
        .unwrap_or(0);

    let after = &text[byte_cursor..];
    let end_rel = after
        .char_indices()
        .find(|(_, c)| rust_word_boundary(*c))
        .map(|(i, _)| i)
        .unwrap_or(after.len());
    let end = byte_cursor + end_rel;

    if start >= end {
        None
    } else {
        Some(start..end)
    }
}
