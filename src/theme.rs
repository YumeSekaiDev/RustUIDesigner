//! Visual themes for the Form Designer and generated apps.
// Each preset changes more than colors — shapes, spacing, fonts, and soft animations.

use egui::{Color32, CornerRadius, FontFamily, FontId, Stroke, Style, Visuals};
use crate::model::FormTheme;

/* ---------- Color constants (used by UI helpers; theme take precedence via apply()) ---------- */
pub const ACCENT: Color32 = Color32::from_rgb(99, 102, 241); // indigo
pub const ACCENT_SOFT: Color32 = Color32::from_rgb(129, 140, 248);
pub const ACCENT_DIM: Color32 = Color32::from_rgb(67, 56, 202);

pub const BG_DEEP: Color32 = Color32::from_rgb(15, 17, 23);
pub const BG_PANEL: Color32 = Color32::from_rgb(22, 25, 34);
pub const BG_ELEVATED: Color32 = Color32::from_rgb(30, 34, 46);
pub const BG_HOVER: Color32 = Color32::from_rgb(38, 43, 58);
pub const BORDER: Color32 = Color32::from_rgb(48, 54, 72);
pub const TEXT: Color32 = Color32::from_rgb(226, 232, 240);
pub const TEXT_MUTED: Color32 = Color32::from_rgb(148, 163, 184);
pub const SUCCESS: Color32 = Color32::from_rgb(52, 211, 153);
pub const DANGER: Color32 = Color32::from_rgb(248, 113, 113);

/// Apply the designer shell theme. This is always the dark shell palette:
/// the per-form `FormTheme` colours the canvas and generated app only, so
/// light form themes (Corporate/Cute/Sunny/...) never make shell text
/// white-on-white in the Properties/Toolbox inputs.
pub fn apply(ctx: &egui::Context, _theme: FormTheme) {
    let mut visuals = Visuals::dark();
    visuals.dark_mode = true;
    visuals.window_fill = BG_PANEL;
    visuals.panel_fill = BG_PANEL;
    visuals.extreme_bg_color = BG_DEEP;
    visuals.faint_bg_color = BG_ELEVATED;
    visuals.code_bg_color = Color32::from_rgb(12, 14, 20);
    visuals.override_text_color = Some(TEXT);
    visuals.hyperlink_color = ACCENT_SOFT;
    visuals.warn_fg_color = Color32::from_rgb(251, 191, 36);
    visuals.error_fg_color = DANGER;
    visuals.window_corner_radius = CornerRadius::same(10);
    visuals.menu_corner_radius = CornerRadius::same(8);
    visuals.window_stroke = Stroke::new(1.0, BORDER);
    visuals.widgets.noninteractive.bg_fill = BG_PANEL;
    visuals.widgets.noninteractive.weak_bg_fill = BG_ELEVATED;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, BORDER);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, TEXT_MUTED);
    visuals.widgets.noninteractive.corner_radius = CornerRadius::same(6);
    visuals.widgets.inactive.bg_fill = BG_ELEVATED;
    visuals.widgets.inactive.weak_bg_fill = BG_ELEVATED;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, BORDER);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, TEXT);
    visuals.widgets.inactive.corner_radius = CornerRadius::same(8);
    visuals.widgets.hovered.bg_fill = BG_HOVER;
    visuals.widgets.hovered.weak_bg_fill = BG_HOVER;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, ACCENT_SOFT);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, Color32::WHITE);
    visuals.widgets.hovered.corner_radius = CornerRadius::same(8);
    visuals.widgets.active.bg_fill = ACCENT_DIM;
    visuals.widgets.active.weak_bg_fill = ACCENT;
    visuals.widgets.active.bg_stroke = Stroke::new(1.0, ACCENT_SOFT);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, Color32::WHITE);
    visuals.widgets.active.corner_radius = CornerRadius::same(8);
    visuals.widgets.open.bg_fill = BG_HOVER;
    visuals.widgets.open.bg_stroke = Stroke::new(1.0, ACCENT);
    visuals.widgets.open.fg_stroke = Stroke::new(1.0, TEXT);
    visuals.widgets.open.corner_radius = CornerRadius::same(8);
    visuals.selection.bg_fill = Color32::from_rgba_unmultiplied(99, 102, 241, 90);
    visuals.selection.stroke = Stroke::new(1.0, ACCENT_SOFT);

    let mut style = Style { visuals, ..Style::default() };
    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    style.spacing.button_padding = egui::vec2(12.0, 6.0);
    style.spacing.window_margin = egui::Margin::same(12);
    style.spacing.menu_margin = egui::Margin::same(8);
    style.spacing.indent = 18.0;
    style.interaction.show_tooltips_only_when_still = true;
    style.text_styles.insert(egui::TextStyle::Heading, FontId::new(18.0, FontFamily::Proportional));
    style.text_styles.insert(egui::TextStyle::Body, FontId::new(13.5, FontFamily::Proportional));
    style.text_styles.insert(egui::TextStyle::Button, FontId::new(13.0, FontFamily::Proportional));
    style.text_styles.insert(egui::TextStyle::Small, FontId::new(11.5, FontFamily::Proportional));
    style.text_styles.insert(egui::TextStyle::Monospace, FontId::new(12.5, FontFamily::Monospace));

    ctx.set_theme(egui::ThemePreference::Dark);
    ctx.set_style_of(egui::Theme::Dark, style);
}

/// Card-like panel frame used by sidebars.
pub fn card_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(BG_PANEL)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(egui::Margin::same(12))
}

/// Soft section header label.
pub fn section_title(ui: &mut egui::Ui, text: &str) {
    ui.label(
        egui::RichText::new(text)
            .strong()
            .size(14.0)
            .color(Color32::WHITE),
    );
    ui.add_space(2.0);
    ui.label(
        egui::RichText::new("─".repeat(18))
            .small()
            .color(BORDER),
    );
    ui.add_space(6.0);
}

/// Pill-style tab button.
pub fn tab_button(ui: &mut egui::Ui, selected: bool, label: &str) -> egui::Response {
    let fill = if selected { ACCENT } else { BG_ELEVATED };
    let text = if selected {
        Color32::WHITE
    } else {
        TEXT_MUTED
    };
    let stroke = if selected {
        Stroke::new(1.0, ACCENT_SOFT)
    } else {
        Stroke::new(1.0, BORDER)
    };

    ui.add(
        egui::Button::new(egui::RichText::new(label).color(text).strong())
            .fill(fill)
            .stroke(stroke)
            .corner_radius(CornerRadius::same(8))
            .min_size(egui::vec2(0.0, 28.0)),
    )
}

/// Primary action button (Save / Reset / Push).
pub fn primary_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    ui.add(
        egui::Button::new(egui::RichText::new(label).color(Color32::WHITE).strong())
            .fill(ACCENT)
            .stroke(Stroke::new(1.0, ACCENT_SOFT))
            .corner_radius(CornerRadius::same(8))
            .min_size(egui::vec2(0.0, 28.0)),
    )
}
