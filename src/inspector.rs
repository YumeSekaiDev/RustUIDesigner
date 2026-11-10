//! Right sidebar: property inspector for the form and selected control.

use crate::model::{
    BorderStyle, ControlKind, FormDocument, FormTheme, Orientation, RgbaColor, StartPosition,
};
use crate::theme::{self, TEXT_MUTED};
use egui::{Color32, ComboBox, RichText, Ui};

pub fn show(ui: &mut Ui, doc: &mut FormDocument) {
    theme::section_title(ui, "Properties");

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            show_form_props(ui, doc);
            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);
            show_control_props(ui, doc);
        });
}

fn show_form_props(ui: &mut Ui, doc: &mut FormDocument) {
    ui.label(RichText::new("FORM").small().strong().color(TEXT_MUTED));
    ui.add_space(6.0);

    prop_row(ui, "Title", |ui| {
        ui.text_edit_singleline(&mut doc.title);
    });

    prop_row(ui, "Width", |ui| {
        ui.add(egui::DragValue::new(&mut doc.width).range(160..=2000).speed(1));
    });
    prop_row(ui, "Height", |ui| {
        ui.add(egui::DragValue::new(&mut doc.height).range(120..=1600).speed(1));
    });

    color_row(ui, "Back color", &mut doc.back_color);

    prop_row(ui, "Font name", |ui| {
        ui.text_edit_singleline(&mut doc.font_name);
    });
    prop_row(ui, "Font size", |ui| {
        ui.add(egui::DragValue::new(&mut doc.font_size).range(6.0..=72.0).speed(0.5));
    });

    prop_row(ui, "Start position", |ui| {
        ComboBox::from_id_salt("form_start_pos")
            .selected_text(doc.start_position.as_str())
            .show_ui(ui, |ui| {
                for &sp in StartPosition::ALL {
                    ui.selectable_value(&mut doc.start_position, sp, sp.as_str());
                }
            });
    });

    ui.checkbox(&mut doc.minimize_box, "Minimize box");
    ui.checkbox(&mut doc.maximize_box, "Maximize box");
    ui.checkbox(&mut doc.client_area_only, "Client area only")
        .on_hover_text("No title bar or window decorations — only the client area is shown");

    prop_row(ui, "Theme", |ui| {
        ComboBox::from_id_salt("form_theme")
            .selected_text(doc.theme.as_str())
            .show_ui(ui, |ui| {
                for &theme in FormTheme::ALL {
                    if ui
                        .selectable_label(doc.theme == theme, theme.as_str())
                        .clicked()
                        && doc.theme != theme
                    {
                        doc.apply_theme(theme);
                    }
                }
            });
    });

    prop_row(ui, "Border style", |ui| {
        ComboBox::from_id_salt("form_border")
            .selected_text(doc.border_style.as_str())
            .show_ui(ui, |ui| {
                for &bs in BorderStyle::ALL {
                    ui.selectable_value(&mut doc.border_style, bs, bs.as_str());
                }
            });
    });

    prop_row(ui, "Opacity", |ui| {
        ui.add(egui::Slider::new(&mut doc.opacity, 0.0..=1.0));
    });

    ui.add_space(6.0);
    ui.collapsing("Designer grid", |ui| {
        ui.checkbox(&mut doc.show_grid, "Show grid");
        ui.checkbox(&mut doc.snap_to_grid, "Snap to grid");
        prop_row(ui, "Grid size", |ui| {
            ui.add(egui::DragValue::new(&mut doc.grid_size).range(2..=64));
        });
    });
}

fn show_control_props(ui: &mut Ui, doc: &mut FormDocument) {
    let Some(id) = doc.selected else {
        ui.label("No control selected.");
        return;
    };
    let Some(idx) = doc.controls.iter().position(|c| c.id == id) else {
        ui.label("Selected control no longer exists.");
        doc.selected = None;
        return;
    };

    let kind = doc.controls[idx].kind;
    ui.label(
        RichText::new(format!("CONTROL · {}", kind.display_name().to_uppercase()))
            .small()
            .strong()
            .color(TEXT_MUTED),
    );
    ui.add_space(6.0);

    // Shared geometry / identity for all kinds.
    prop_row(ui, "Name", |ui| {
        ui.text_edit_singleline(&mut doc.controls[idx].name);
    });

    match kind {
        ControlKind::Button => edit_button(ui, &mut doc.controls[idx]),
        ControlKind::RadioBox => edit_radiobox(ui, &mut doc.controls[idx]),
        ControlKind::Slider => edit_slider(ui, &mut doc.controls[idx]),
        ControlKind::TextBox => edit_textbox(ui, &mut doc.controls[idx]),
        ControlKind::Label | ControlKind::DropDown => edit_generic(ui, &mut doc.controls[idx]),
        ControlKind::PictureBox => edit_picturebox(ui, &mut doc.controls[idx]),
        ControlKind::Timer => edit_timer(ui, &mut doc.controls[idx]),
    }

    ui.add_space(8.0);
    if ui.button("Delete").clicked() {
        doc.delete_selected();
    }
}

fn edit_geometry(ui: &mut Ui, ctrl: &mut crate::model::Control) {
    prop_row(ui, "X", |ui| {
        ui.add(egui::DragValue::new(&mut ctrl.rect.x).speed(1));
    });
    prop_row(ui, "Y", |ui| {
        ui.add(egui::DragValue::new(&mut ctrl.rect.y).speed(1));
    });
    prop_row(ui, "Width", |ui| {
        ui.add(egui::DragValue::new(&mut ctrl.rect.width).range(16..=2000).speed(1));
    });
    prop_row(ui, "Height", |ui| {
        ui.add(egui::DragValue::new(&mut ctrl.rect.height).range(16..=1600).speed(1));
    });
}

fn edit_font(ui: &mut Ui, ctrl: &mut crate::model::Control) {
    prop_row(ui, "Font name", |ui| {
        ui.text_edit_singleline(&mut ctrl.font_name);
    });
    prop_row(ui, "Font size", |ui| {
        ui.add(egui::DragValue::new(&mut ctrl.font_size).range(6.0..=72.0).speed(0.5));
    });
}

fn edit_button(ui: &mut Ui, ctrl: &mut crate::model::Control) {
    prop_row(ui, "Text", |ui| {
        ui.text_edit_singleline(&mut ctrl.text);
    });
    edit_geometry(ui, ctrl);
    ui.checkbox(&mut ctrl.enabled, "Enabled");
    edit_font(ui, ctrl);
    color_row(ui, "Back color", &mut ctrl.back_color);
    color_row(ui, "Fore color", &mut ctrl.fore_color);
}

fn edit_radiobox(ui: &mut Ui, ctrl: &mut crate::model::Control) {
    prop_row(ui, "Text", |ui| {
        ui.text_edit_singleline(&mut ctrl.text);
    });
    edit_geometry(ui, ctrl);
    ui.checkbox(&mut ctrl.checked, "Checked");
    ui.checkbox(&mut ctrl.enabled, "Enabled");
    edit_font(ui, ctrl);
}

fn edit_slider(ui: &mut Ui, ctrl: &mut crate::model::Control) {
    edit_geometry(ui, ctrl);

    prop_row(ui, "Minimum", |ui| {
        if ui
            .add(egui::DragValue::new(&mut ctrl.minimum).speed(1))
            .changed()
        {
            ctrl.clamp_slider_value();
        }
    });
    prop_row(ui, "Maximum", |ui| {
        if ui
            .add(egui::DragValue::new(&mut ctrl.maximum).speed(1))
            .changed()
        {
            ctrl.clamp_slider_value();
        }
    });
    prop_row(ui, "Value", |ui| {
        if ui
            .add(
                egui::DragValue::new(&mut ctrl.value)
                    .range(ctrl.minimum..=ctrl.maximum)
                    .speed(1),
            )
            .changed()
        {
            ctrl.clamp_slider_value();
        }
    });

    ui.checkbox(&mut ctrl.enabled, "Enabled");

    prop_row(ui, "Orientation", |ui| {
        ComboBox::from_id_salt(("slider_orient", ctrl.id))
            .selected_text(ctrl.orientation.as_str())
            .show_ui(ui, |ui| {
                for &o in Orientation::ALL {
                    ui.selectable_value(&mut ctrl.orientation, o, o.as_str());
                }
            });
    });
}

fn edit_textbox(ui: &mut Ui, ctrl: &mut crate::model::Control) {
    prop_row(ui, "Text", |ui| {
        if ctrl.multiline {
            ui.add(
                egui::TextEdit::multiline(&mut ctrl.text)
                    .desired_width(ui.available_width())
                    .desired_rows(3),
            );
        } else {
            ui.text_edit_singleline(&mut ctrl.text);
        }
    });
    edit_geometry(ui, ctrl);
    ui.checkbox(&mut ctrl.multiline, "Multiline");
    ui.checkbox(&mut ctrl.read_only, "Read only");
    ui.checkbox(&mut ctrl.enabled, "Enabled");
    edit_font(ui, ctrl);

    prop_row(ui, "Password char", |ui| {
        let mut buf = ctrl
            .password_char
            .map(|c| c.to_string())
            .unwrap_or_default();
        let resp = ui.add(
            egui::TextEdit::singleline(&mut buf)
                .desired_width(40.0)
                .hint_text("none"),
        );
        if resp.changed() {
            ctrl.password_char = buf.chars().next();
        }
        if ctrl.password_char.is_some() && ui.small_button("Clear").clicked() {
            ctrl.password_char = None;
        }
    });
}

fn edit_picturebox(ui: &mut Ui, ctrl: &mut crate::model::Control) {
    prop_row(ui, "Image path", |ui| {
        ui.text_edit_singleline(&mut ctrl.image_path);
    });
    ui.horizontal(|ui| {
        if ui.button("Browse…").clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .set_title("Choose Image")
                .add_filter("Images", &["png", "jpg", "jpeg", "gif", "bmp", "webp", "svg"])
                .pick_file()
            {
                ctrl.image_path = path.display().to_string();
            }
        }
        if !ctrl.image_path.is_empty() && ui.small_button("Clear").clicked() {
            ctrl.image_path.clear();
        }
    });
    edit_geometry(ui, ctrl);
    ui.checkbox(&mut ctrl.enabled, "Enabled");
    edit_font(ui, ctrl);
    color_row(ui, "Back color", &mut ctrl.back_color);
    color_row(ui, "Fore color", &mut ctrl.fore_color);
}

fn edit_timer(ui: &mut Ui, ctrl: &mut crate::model::Control) {
    prop_row(ui, "Interval (ms)", |ui| {
        ui.add(egui::DragValue::new(&mut ctrl.interval_ms).range(1..=3_600_000).speed(10));
    });
    ui.checkbox(&mut ctrl.enabled, "Enabled (timer runs)");
    ui.label(RichText::new("Non-visual at runtime: fires a tick every interval. Position/size only affect the designer canvas.").small().weak());
    edit_geometry(ui, ctrl);
}

fn edit_generic(ui: &mut Ui, ctrl: &mut crate::model::Control) {
    prop_row(ui, "Text", |ui| {
        ui.text_edit_singleline(&mut ctrl.text);
    });
    edit_geometry(ui, ctrl);
    ui.checkbox(&mut ctrl.enabled, "Enabled");
    edit_font(ui, ctrl);
    color_row(ui, "Back color", &mut ctrl.back_color);
    color_row(ui, "Fore color", &mut ctrl.fore_color);
}

fn prop_row(ui: &mut Ui, label: &str, add: impl FnOnce(&mut Ui)) {
    ui.horizontal(|ui| {
        ui.set_min_width(96.0);
        ui.label(label);
        add(ui);
    });
}

fn color_row(ui: &mut Ui, label: &str, color: &mut RgbaColor) {
    ui.horizontal(|ui| {
        ui.set_min_width(96.0);
        ui.label(label);
        let mut egui_c = color.to_egui();
        if ui.color_edit_button_srgba(&mut egui_c).changed() {
            *color = RgbaColor::from_egui(egui_c);
        }
        ui.colored_label(
            Color32::GRAY,
            format!("{},{},{}", color.r, color.g, color.b),
        );
    });
}
