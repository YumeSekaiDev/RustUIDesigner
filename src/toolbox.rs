//! Left sidebar: control toolbox with click-to-place and drag-onto-canvas.

use crate::canvas::PlacementTool;
use crate::model::ControlKind;
use crate::theme::{self, ACCENT_SOFT, BG_ELEVATED, BG_HOVER, BORDER, TEXT_MUTED};
use egui::{Color32, CornerRadius, Frame, Margin, RichText, Sense, Stroke, Ui};

pub fn show(ui: &mut Ui, placement: &mut Option<PlacementTool>) {
    theme::section_title(ui, "Toolbox");
    ui.label(
        RichText::new("Click a tool, then click the form — or drag onto the canvas.")
            .small()
            .color(TEXT_MUTED),
    );
    ui.add_space(10.0);

    for &kind in ControlKind::ALL {
        let selected = placement.map(|p| p.kind) == Some(kind);
        let text = kind.display_name();
        let id = egui::Id::unique(("toolbox", text));

        let frame = Frame::new()
            .fill(if selected {
                Color32::from_rgba_unmultiplied(99, 102, 241, 40)
            } else {
                BG_ELEVATED
            })
            .stroke(Stroke::new(
                1.0,
                if selected { ACCENT_SOFT } else { BORDER },
            ))
            .corner_radius(CornerRadius::same(10))
            .inner_margin(Margin::symmetric(10, 8));

        let inner = ui.dnd_drag_source(id, kind, |ui| {
            frame
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(tool_glyph(kind))
                                .size(16.0)
                                .color(if selected { ACCENT_SOFT } else { TEXT_MUTED }),
                        );
                        ui.add_space(6.0);
                        let label = if selected {
                            RichText::new(text).strong().color(Color32::WHITE)
                        } else {
                            RichText::new(text).color(Color32::from_rgb(203, 213, 225))
                        };
                        ui.add(egui::Label::new(label).sense(Sense::click()));
                    });
                })
                .response
        });

        if inner.response.clicked() || inner.inner.clicked() {
            if selected {
                *placement = None;
            } else {
                *placement = Some(PlacementTool { kind });
            }
        }

        ui.add_space(6.0);
    }

    ui.add_space(10.0);
    if placement.is_some() {
        if ui
            .add(
                egui::Button::new(RichText::new("Cancel placement").color(Color32::WHITE))
                    .fill(BG_HOVER)
                    .stroke(Stroke::new(1.0, BORDER))
                    .corner_radius(CornerRadius::same(8)),
            )
            .clicked()
        {
            *placement = None;
        }
        ui.add_space(6.0);
        ui.label(
            RichText::new("Esc also cancels")
                .small()
                .color(TEXT_MUTED),
        );
    } else {
        ui.label(
            RichText::new("Tip: drop tools onto the form surface.")
                .small()
                .color(TEXT_MUTED),
        );
    }
}

fn tool_glyph(kind: ControlKind) -> &'static str {
    match kind {
        ControlKind::Button => "▣",
        ControlKind::TextBox => "▭",
        ControlKind::Label => "Aa",
        ControlKind::Slider => "═",
        ControlKind::DropDown => "▾",
        ControlKind::RadioBox => "◎",
        ControlKind::PictureBox => "🖼",
        ControlKind::Timer => "⏱",
    }
}
