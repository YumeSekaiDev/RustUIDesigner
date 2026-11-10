//! Design canvas: paints the form surface and handles selection / drag / resize.

use crate::model::{
    BorderStyle, Control, ControlId, ControlKind, FormDocument, Orientation, RectI,
};
use egui::{
    Align2, Color32, CornerRadius, CursorIcon, FontId, Pos2, Rect, Sense, Stroke, StrokeKind, Ui,
    Vec2,
};

const TITLE_BAR_H: f32 = 28.0;
const HANDLE_SIZE: f32 = 6.0;
const MIN_CTRL_W: i32 = 16;
const MIN_CTRL_H: i32 = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DragMode {
    Move,
    Resize(ResizeHandle),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResizeHandle {
    Nw,
    N,
    Ne,
    E,
    Se,
    S,
    Sw,
    W,
}

impl ResizeHandle {
    fn cursor(self) -> CursorIcon {
        match self {
            ResizeHandle::Nw | ResizeHandle::Se => CursorIcon::ResizeNwSe,
            ResizeHandle::Ne | ResizeHandle::Sw => CursorIcon::ResizeNeSw,
            ResizeHandle::N | ResizeHandle::S => CursorIcon::ResizeVertical,
            ResizeHandle::E | ResizeHandle::W => CursorIcon::ResizeHorizontal,
        }
    }
}

#[derive(Debug, Default)]
pub struct CanvasState {
    drag: Option<ActiveDrag>,
}

#[derive(Debug, Clone)]
struct ActiveDrag {
    id: ControlId,
    mode: DragMode,
    start_pointer: Pos2,
    start_rect: RectI,
}

#[derive(Debug, Clone, Copy)]
pub struct PlacementTool {
    pub kind: ControlKind,
}

pub struct CanvasResponse {
    pub request_repaint: bool,
}

pub fn show(
    ui: &mut Ui,
    doc: &mut FormDocument,
    state: &mut CanvasState,
    placement: &mut Option<PlacementTool>,
) -> CanvasResponse {
    let available = ui.available_size();
    let (response, painter) = ui.allocate_painter(available, Sense::click_and_drag());

    let form_w = doc.width.max(1) as f32;
    let form_h = doc.height.max(1) as f32;
    let title_h = if doc.client_area_only { 0.0 } else { TITLE_BAR_H };
    let form_outer = layout_form_rect(response.rect, form_w, form_h + title_h);
    let title_rect =
        Rect::from_min_size(form_outer.min, Vec2::new(form_outer.width(), title_h));
    let client_rect = Rect::from_min_max(
        Pos2::new(form_outer.min.x, form_outer.min.y + title_h),
        form_outer.max,
    );

    // Apply form opacity to the whole form chrome + client.
    let mut form_painter = painter.clone();
    form_painter.set_opacity(doc.opacity.clamp(0.0, 1.0));

    form_painter.rect_filled(
        form_outer.translate(Vec2::new(4.0, 4.0)),
        CornerRadius::same(2),
        Color32::from_black_alpha(40),
    );

    // Title bar + caption buttons reflecting minimize/maximize flags
    // (skipped entirely in client-area-only mode).
    if !doc.client_area_only {
    form_painter.rect_filled(title_rect, CornerRadius::ZERO, doc.theme.palette().button_back.to_egui());
    let title_font = FontId::new(doc.font_size.clamp(8.0, 20.0), egui::FontFamily::Proportional);
    form_painter.text(
        title_rect.left_center() + Vec2::new(8.0, 0.0),
        Align2::LEFT_CENTER,
        &doc.title,
        title_font,
        Color32::WHITE,
    );
    paint_caption_buttons(&form_painter, title_rect, doc.minimize_box, doc.maximize_box, doc.theme);
    }

    // Client background (rounded when borderless with a corner radius).
    let form_radius = if doc.client_area_only { doc.corner_radius } else { 0 };
    form_painter.rect_filled(client_rect, CornerRadius::same(form_radius), doc.back_color.to_egui());

    // Custom caption buttons floating at the top-right of the client area,
    // replacing the default window buttons on borderless forms.
    if doc.client_area_only && doc.custom_caption_buttons {
        let strip = Rect::from_min_max(
            Pos2::new(client_rect.right() - 84.0, client_rect.top()),
            Pos2::new(client_rect.right(), client_rect.top() + TITLE_BAR_H),
        );
        paint_caption_buttons(&form_painter, strip, doc.minimize_box, doc.maximize_box, doc.theme);
    }

    // Border style.
    paint_form_border(&form_painter, form_outer, doc.border_style);

    if doc.show_grid {
        paint_grid(&form_painter, client_rect, doc.grid_size.max(2) as f32);
    }

    let pointer_screen = ui
        .ctx()
        .pointer_interact_pos()
        .or_else(|| ui.ctx().pointer_hover_pos())
        .or_else(|| response.interact_pointer_pos());
    let pointer_form = pointer_screen.map(|p| screen_to_form(p, client_rect));
    let over_client = pointer_screen.is_some_and(|p| client_rect.contains(p));

    if response.hovered() || response.has_focus() || over_client {
        ui.input(|i| {
            if i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace) {
                doc.delete_selected();
                state.drag = None;
            }
            if i.key_pressed(egui::Key::Escape) {
                doc.selected = None;
                state.drag = None;
                *placement = None;
            }
            let step = if i.modifiers.shift {
                doc.grid_size.max(1)
            } else {
                1
            };
            if i.key_pressed(egui::Key::ArrowLeft) {
                doc.nudge_selected(-step, 0);
            }
            if i.key_pressed(egui::Key::ArrowRight) {
                doc.nudge_selected(step, 0);
            }
            if i.key_pressed(egui::Key::ArrowUp) {
                doc.nudge_selected(0, -step);
            }
            if i.key_pressed(egui::Key::ArrowDown) {
                doc.nudge_selected(0, step);
            }
        });
    }

    if let Some(tool) = *placement {
        if over_client {
            ui.ctx().set_cursor_icon(CursorIcon::Copy);
            if response.clicked() {
                if let Some(pf) = pointer_form {
                    doc.add_control(tool.kind, pf.x, pf.y);
                    *placement = None;
                }
            }
        }
    }

    let dropped = response.dnd_release_payload::<ControlKind>().or_else(|| {
        let released = ui.input(|i| i.pointer.any_released());
        if released && over_client && pointer_form.is_some() {
            egui::DragAndDrop::take_payload::<ControlKind>(ui.ctx())
        } else {
            None
        }
    });
    if let (Some(kind), Some(pf)) = (dropped, pointer_form) {
        doc.add_control(*kind, pf.x, pf.y);
        *placement = None;
    }

    let dragging_kind = egui::DragAndDrop::payload::<ControlKind>(ui.ctx()).map(|k| *k);

    let mut consumed = false;

    if let Some(id) = doc.selected {
        if let Some(ctrl) = doc.controls.iter().find(|c| c.id == id) {
            let screen_rect = form_to_screen_rect(ctrl.rect, client_rect);
            if let Some(handle) = hit_handle(screen_rect, pointer_screen) {
                ui.ctx().set_cursor_icon(handle.cursor());
                if response.drag_started() {
                    state.drag = Some(ActiveDrag {
                        id,
                        mode: DragMode::Resize(handle),
                        start_pointer: pointer_form.unwrap_or(Pos2::ZERO),
                        start_rect: ctrl.rect,
                    });
                    consumed = true;
                }
            }
        }
    }

    if !consumed {
        if let Some(pf) = pointer_form {
            if response.drag_started() || response.clicked() {
                if let Some(id) = doc.hit_test(pf.x, pf.y) {
                    doc.selected = Some(id);
                    if response.drag_started() {
                        if let Some(ctrl) = doc.controls.iter().find(|c| c.id == id) {
                            state.drag = Some(ActiveDrag {
                                id,
                                mode: DragMode::Move,
                                start_pointer: pf,
                                start_rect: ctrl.rect,
                            });
                        }
                    }
                    consumed = true;
                } else if response.clicked() && placement.is_none() {
                    doc.selected = None;
                    state.drag = None;
                }
            }
        }
    }

    if let Some(drag) = state.drag.clone() {
        if response.dragged() {
            if let Some(pf) = pointer_form {
                let dx = pf.x - drag.start_pointer.x;
                let dy = pf.y - drag.start_pointer.y;
                let snap = doc.snap_to_grid;
                let grid = doc.grid_size;
                let new_rect = match drag.mode {
                    DragMode::Move => {
                        let mut r = drag.start_rect;
                        r.x = snap_opt(drag.start_rect.x as f32 + dx, snap, grid);
                        r.y = snap_opt(drag.start_rect.y as f32 + dy, snap, grid);
                        r
                    }
                    DragMode::Resize(handle) => {
                        resize_rect(drag.start_rect, handle, dx, dy, snap, grid)
                    }
                };
                if let Some(ctrl) = doc.controls.iter_mut().find(|c| c.id == drag.id) {
                    ctrl.rect = new_rect;
                }
            }
        }
        if response.drag_stopped() {
            state.drag = None;
        }
    }

    for ctrl in &doc.controls {
        let selected = doc.selected == Some(ctrl.id);
        paint_control(&form_painter, ctrl, client_rect, selected, doc.theme);
    }

    // Per-control hover effect (Properties -> "Hover effect"): Bubbly is a
    // breathing outline, Glow is a soft layered halo. None = no animation.
    if let (Some(pf), true) = (pointer_form, over_client) {
        if let Some(id) = doc.hit_test(pf.x, pf.y) {
            if let Some(ctrl) = doc.controls.iter().find(|c| c.id == id) {
                let r = form_to_screen_rect(ctrl.rect, client_rect);
                let time = ui.ctx().input(|i| i.time);
                let pulse = ((time * 4.0).sin() * 0.5 + 0.5) as f32;
                let radius = CornerRadius::same(doc.theme.palette().button_radius);
                match ctrl.hover_effect {
                    crate::model::HoverEffect::None => {}
                    crate::model::HoverEffect::Bubbly => {
                        form_painter.rect_stroke(
                            r.expand(2.0 + pulse * 3.0),
                            radius,
                            Stroke::new(1.5, doc.theme.palette().button_back.to_egui()),
                            StrokeKind::Outside,
                        );
                        ui.ctx().request_repaint();
                    }
                    crate::model::HoverEffect::Glow => {
                        let base = ctrl.back_color.to_egui();
                        for layer in (1..=3).rev() {
                            let grow = layer as f32 * 3.0;
                            let alpha = ((30 - layer * 7) as f32 * (0.65 + 0.35 * pulse)) as u8;
                            form_painter.rect_filled(
                                r.expand(grow),
                                radius,
                                Color32::from_rgba_unmultiplied(base.r(), base.g(), base.b(), alpha),
                            );
                        }
                        form_painter.rect_stroke(
                            r.expand(1.5),
                            radius,
                            Stroke::new(1.5, Color32::from_rgba_unmultiplied(base.r(), base.g(), base.b(), 140)),
                            StrokeKind::Outside,
                        );
                        ui.ctx().request_repaint();
                    }
                    crate::model::HoverEffect::Ripple => {
                        // Two sonar rings, half a cycle apart, expanding and fading.
                        let base = doc.theme.palette().button_back.to_egui();
                        for ring in 0..2 {
                            let phase = ((time * 0.9 + ring as f64 * 0.5).fract()) as f32;
                            let alpha = ((1.0 - phase) * 160.0) as u8;
                            form_painter.rect_stroke(
                                r.expand(2.0 + phase * 14.0),
                                radius,
                                Stroke::new(2.0, Color32::from_rgba_unmultiplied(base.r(), base.g(), base.b(), alpha)),
                                StrokeKind::Outside,
                            );
                        }
                        ui.ctx().request_repaint();
                    }
                    crate::model::HoverEffect::Lift => {
                        // Growing offset shadow below + bright top rim: the control "lifts".
                        let lift = 2.0 + pulse * 5.0;
                        for layer in (1..=3).rev() {
                            let grow = layer as f32 * 2.0;
                            let alpha = 26 - layer * 6;
                            form_painter.rect_filled(
                                r.expand(grow).translate(egui::vec2(0.0, lift + grow * 0.5)),
                                radius,
                                Color32::from_rgba_unmultiplied(0, 0, 0, alpha as u8),
                            );
                        }
                        form_painter.rect_stroke(
                            r.expand(1.0),
                            radius,
                            Stroke::new(1.5, Color32::from_rgba_unmultiplied(255, 255, 255, 90 + (pulse * 60.0) as u8)),
                            StrokeKind::Outside,
                        );
                        ui.ctx().request_repaint();
                    }
                }
            }
        }
    }

    if let Some(ctrl) = doc.selected_control() {
        let screen_rect = form_to_screen_rect(ctrl.rect, client_rect);
        paint_selection(&form_painter, screen_rect);
    }

    let ghost_kind = placement.map(|t| t.kind).or(dragging_kind);
    if let (Some(kind), Some(pf)) = (ghost_kind, pointer_form) {
        if over_client {
            ui.ctx().set_cursor_icon(CursorIcon::Copy);
            let x = doc.snap_value(pf.x);
            let y = doc.snap_value(pf.y);
            let ghost = Control::new(0, kind, String::new(), x, y);
            let mut ghost_painter = form_painter.clone();
            ghost_painter.set_opacity(0.45 * doc.opacity.clamp(0.0, 1.0));
            paint_control(&ghost_painter, &ghost, client_rect, false, doc.theme);
        }
    }

    let _ = consumed;
    CanvasResponse {
        request_repaint: response.dragged()
            || state.drag.is_some()
            || dragging_kind.is_some()
            || placement.is_some(),
    }
}

fn paint_caption_buttons(painter: &egui::Painter, title: Rect, minimize: bool, maximize: bool, theme: crate::model::FormTheme) {
    let pal = theme.palette();
    let fill = pal.button_back.to_egui();
    let fore = pal.button_fore.to_egui();
    let pill = CornerRadius::same(pal.button_radius.min(7));
    let btn_w = 16.0;
    let btn_h = 14.0;
    let gap = 4.0;
    let mut right = title.right() - 6.0;

    // Close (always shown as design chrome)
    let close = Rect::from_center_size(
        Pos2::new(right - btn_w * 0.5, title.center().y),
        Vec2::new(btn_w, btn_h),
    );
    painter.rect_filled(close, pill, Color32::from_rgb(198, 48, 49));
    painter.line_segment(
        [close.min + Vec2::splat(4.0), close.max - Vec2::splat(4.0)],
        Stroke::new(1.0, Color32::WHITE),
    );
    painter.line_segment(
        [
            Pos2::new(close.max.x - 4.0, close.min.y + 4.0),
            Pos2::new(close.min.x + 4.0, close.max.y - 4.0),
        ],
        Stroke::new(1.0, Color32::WHITE),
    );
    right -= btn_w + gap;

    if maximize {
        let r = Rect::from_center_size(
            Pos2::new(right - btn_w * 0.5, title.center().y),
            Vec2::new(btn_w, btn_h),
        );
        painter.rect_filled(r, pill, fill);
        painter.rect_stroke(
            r.shrink2(Vec2::new(5.0, 4.0)),
            CornerRadius::ZERO,
            Stroke::new(1.0, fore),
            StrokeKind::Inside,
        );
        right -= btn_w + gap;
    }

    if minimize {
        let r = Rect::from_center_size(
            Pos2::new(right - btn_w * 0.5, title.center().y),
            Vec2::new(btn_w, btn_h),
        );
        painter.rect_filled(r, pill, fill);
        painter.hline(
            (r.left() + 4.0)..=(r.right() - 4.0),
            r.center().y + 2.0,
            Stroke::new(1.5, fore),
        );
    }
}

fn paint_form_border(painter: &egui::Painter, outer: Rect, style: BorderStyle) {
    match style {
        BorderStyle::None => {}
        BorderStyle::FixedSingle => {
            painter.rect_stroke(
                outer,
                CornerRadius::ZERO,
                Stroke::new(1.0, Color32::from_rgb(80, 80, 80)),
                StrokeKind::Outside,
            );
        }
        BorderStyle::FixedDialog => {
            painter.rect_stroke(
                outer,
                CornerRadius::ZERO,
                Stroke::new(2.0, Color32::from_rgb(60, 60, 60)),
                StrokeKind::Outside,
            );
            painter.rect_stroke(
                outer.shrink(2.0),
                CornerRadius::ZERO,
                Stroke::new(1.0, Color32::from_rgb(180, 180, 180)),
                StrokeKind::Inside,
            );
        }
        BorderStyle::Sizable => {
            painter.rect_stroke(
                outer,
                CornerRadius::ZERO,
                Stroke::new(1.0, Color32::from_rgb(80, 80, 80)),
                StrokeKind::Outside,
            );
            // Corner grip hint.
            let grip = 10.0;
            let p = outer.right_bottom();
            painter.line_segment(
                [Pos2::new(p.x - grip, p.y), Pos2::new(p.x, p.y - grip)],
                Stroke::new(1.0, Color32::from_rgb(120, 120, 120)),
            );
            painter.line_segment(
                [
                    Pos2::new(p.x - grip + 3.0, p.y),
                    Pos2::new(p.x, p.y - grip + 3.0),
                ],
                Stroke::new(1.0, Color32::from_rgb(120, 120, 120)),
            );
        }
    }
}

fn layout_form_rect(area: Rect, form_w: f32, form_h: f32) -> Rect {
    let pad = 16.0;
    let origin = area.min + Vec2::splat(pad);
    Rect::from_min_size(
        origin,
        Vec2::new(form_w, form_h).min(area.size() - Vec2::splat(pad * 2.0)),
    )
}

fn screen_to_form(screen: Pos2, client: Rect) -> Pos2 {
    Pos2::new(screen.x - client.min.x, screen.y - client.min.y)
}

fn form_to_screen_rect(r: RectI, client: Rect) -> Rect {
    Rect::from_min_size(
        Pos2::new(client.min.x + r.x as f32, client.min.y + r.y as f32),
        Vec2::new(r.width as f32, r.height as f32),
    )
}

fn paint_grid(painter: &egui::Painter, client: Rect, grid: f32) {
    if grid < 2.0 {
        return;
    }
    let stroke = Stroke::new(1.0, Color32::from_rgb(210, 210, 210));
    let mut x = client.min.x;
    while x <= client.max.x {
        painter.line_segment(
            [Pos2::new(x, client.min.y), Pos2::new(x, client.max.y)],
            stroke,
        );
        x += grid;
    }
    let mut y = client.min.y;
    while y <= client.max.y {
        painter.line_segment(
            [Pos2::new(client.min.x, y), Pos2::new(client.max.x, y)],
            stroke,
        );
        y += grid;
    }
}

fn ctrl_font(ctrl: &Control) -> FontId {
    // Font name is stored for codegen / WinForms parity; egui preview uses proportional/monospace.
    let family = if ctrl.font_name.to_ascii_lowercase().contains("mono")
        || ctrl.font_name.to_ascii_lowercase().contains("consolas")
        || ctrl.font_name.to_ascii_lowercase().contains("courier")
    {
        egui::FontFamily::Monospace
    } else {
        egui::FontFamily::Proportional
    };
    FontId::new(ctrl.font_size.clamp(6.0, 72.0), family)
}

fn dim_if_disabled(color: Color32, enabled: bool) -> Color32 {
    if enabled {
        color
    } else {
        let [r, g, b, a] = color.to_array();
        Color32::from_rgba_unmultiplied(
            ((r as u16 + 180) / 2) as u8,
            ((g as u16 + 180) / 2) as u8,
            ((b as u16 + 180) / 2) as u8,
            a,
        )
    }
}

fn display_text(ctrl: &Control) -> String {
    if let Some(ch) = ctrl.password_char {
        ch.to_string().repeat(ctrl.text.chars().count().max(1))
    } else {
        ctrl.text.clone()
    }
}

fn paint_control(painter: &egui::Painter, ctrl: &Control, client: Rect, selected: bool, theme: crate::model::FormTheme) {
    let rect = form_to_screen_rect(ctrl.rect, client);
    let font = ctrl_font(ctrl);
    let back = dim_if_disabled(ctrl.back_color.to_egui(), ctrl.enabled);
    let fore = dim_if_disabled(ctrl.fore_color.to_egui(), ctrl.enabled);
    let text = display_text(ctrl);
    let text = if ctrl.kind == ControlKind::Button && theme.palette().uppercase_buttons {
        text.to_uppercase()
    } else {
        text
    };

    match ctrl.kind {
        ControlKind::Button => {
            painter.rect(
                rect,
                CornerRadius::same(theme.palette().button_radius.min((rect.height() / 2.0) as u8)),
                back,
                Stroke::new(1.0, Color32::from_rgb(100, 100, 100)),
                StrokeKind::Inside,
            );
            if ctrl.enabled {
                painter.line_segment(
                    [
                        rect.left_top() + Vec2::new(1.0, 1.0),
                        rect.right_top() + Vec2::new(-1.0, 1.0),
                    ],
                    Stroke::new(1.0, Color32::WHITE),
                );
            }
            painter.text(rect.center(), Align2::CENTER_CENTER, text, font, fore);
        }
        ControlKind::TextBox => {
            painter.rect(
                rect,
                CornerRadius::same(theme.palette().input_radius),
                back,
                Stroke::new(
                    1.0,
                    if ctrl.read_only {
                        Color32::from_rgb(160, 160, 160)
                    } else {
                        Color32::from_rgb(100, 100, 100)
                    },
                ),
                StrokeKind::Inside,
            );
            let pad = Vec2::new(4.0, 2.0);
            if ctrl.multiline {
                painter.text(
                    rect.left_top() + pad,
                    Align2::LEFT_TOP,
                    text,
                    font,
                    fore,
                );
            } else {
                painter.text(
                    rect.left_center() + Vec2::new(4.0, 0.0),
                    Align2::LEFT_CENTER,
                    text,
                    font,
                    fore,
                );
            }
        }
        ControlKind::Label => {
            painter.text(rect.left_center(), Align2::LEFT_CENTER, text, font, fore);
        }
        ControlKind::Slider => {
            painter.rect(
                rect,
                CornerRadius::same(theme.palette().input_radius),
                back,
                Stroke::new(1.0, Color32::from_rgb(160, 160, 160)),
                StrokeKind::Inside,
            );
            let t = ctrl.slider_normalized().clamp(0.0, 1.0);
            let knob = fore;
            match ctrl.orientation {
                Orientation::Horizontal => {
                    let y = rect.center().y;
                    let left = rect.left() + 8.0;
                    let right = rect.right() - 8.0;
                    painter.line_segment(
                        [Pos2::new(left, y), Pos2::new(right, y)],
                        Stroke::new(3.0, Color32::from_rgb(180, 180, 180)),
                    );
                    let x = left + (right - left) * t;
                    painter.circle_filled(Pos2::new(x, y), 6.0, knob);
                }
                Orientation::Vertical => {
                    let x = rect.center().x;
                    let top = rect.top() + 8.0;
                    let bottom = rect.bottom() - 8.0;
                    painter.line_segment(
                        [Pos2::new(x, top), Pos2::new(x, bottom)],
                        Stroke::new(3.0, Color32::from_rgb(180, 180, 180)),
                    );
                    // Value increases upward for vertical track.
                    let y = bottom - (bottom - top) * t;
                    painter.circle_filled(Pos2::new(x, y), 6.0, knob);
                }
            }
        }
        ControlKind::DropDown => {
            painter.rect(
                rect,
                CornerRadius::same(theme.palette().input_radius),
                back,
                Stroke::new(1.0, Color32::from_rgb(100, 100, 100)),
                StrokeKind::Inside,
            );
            painter.text(
                rect.left_center() + Vec2::new(6.0, 0.0),
                Align2::LEFT_CENTER,
                text,
                font,
                fore,
            );
            let cx = rect.right() - 12.0;
            let cy = rect.center().y;
            painter.line_segment(
                [Pos2::new(cx - 4.0, cy - 2.0), Pos2::new(cx, cy + 3.0)],
                Stroke::new(1.5, Color32::DARK_GRAY),
            );
            painter.line_segment(
                [Pos2::new(cx, cy + 3.0), Pos2::new(cx + 4.0, cy - 2.0)],
                Stroke::new(1.5, Color32::DARK_GRAY),
            );
        }
        ControlKind::PictureBox => {
            painter.rect(
                rect,
                CornerRadius::same(theme.palette().input_radius),
                back,
                Stroke::new(1.0, Color32::from_rgb(100, 100, 100)),
                StrokeKind::Inside,
            );
            // Simple picture glyph: sun + mountains, plus the file name.
            let sun = Pos2::new(rect.left() + rect.width() * 0.28, rect.top() + rect.height() * 0.30);
            painter.circle_filled(sun, (rect.height() * 0.09).clamp(3.0, 8.0), fore);
            let base = rect.bottom() - 4.0;
            painter.add(egui::Shape::convex_polygon(
                vec![
                    Pos2::new(rect.left() + 4.0, base),
                    Pos2::new(rect.left() + rect.width() * 0.38, rect.top() + rect.height() * 0.45),
                    Pos2::new(rect.left() + rect.width() * 0.58, base),
                ],
                fore,
                Stroke::NONE,
            ));
            painter.add(egui::Shape::convex_polygon(
                vec![
                    Pos2::new(rect.left() + rect.width() * 0.45, base),
                    Pos2::new(rect.left() + rect.width() * 0.72, rect.top() + rect.height() * 0.55),
                    Pos2::new(rect.right() - 4.0, base),
                ],
                fore,
                Stroke::NONE,
            ));
            let label = if ctrl.image_path.is_empty() {
                "PictureBox".to_owned()
            } else {
                std::path::Path::new(&ctrl.image_path)
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| ctrl.image_path.clone())
            };
            painter.text(rect.center_bottom() + Vec2::new(0.0, -2.0), Align2::CENTER_BOTTOM, label, font, fore);
        }
        ControlKind::Timer => {
            // Non-visual at runtime; shown as a small clock badge on the canvas.
            painter.rect(
                rect,
                CornerRadius::same(4),
                back,
                Stroke::new(1.0, Color32::from_rgb(160, 130, 40)),
                StrokeKind::Inside,
            );
            let c = rect.center();
            let r = (rect.width().min(rect.height()) * 0.32).clamp(4.0, 12.0);
            painter.circle_stroke(c, r, Stroke::new(1.5, fore));
            painter.line_segment([c, Pos2::new(c.x, c.y - r * 0.7)], Stroke::new(1.5, fore));
            painter.line_segment([c, Pos2::new(c.x + r * 0.6, c.y)], Stroke::new(1.5, fore));
        }
        ControlKind::RadioBox => {
            let r = (rect.height() * 0.35).clamp(5.0, 9.0);
            let center = Pos2::new(rect.left() + r + 4.0, rect.center().y);
            painter.circle_stroke(
                center,
                r,
                Stroke::new(1.5, Color32::from_rgb(80, 80, 80)),
            );
            if ctrl.checked {
                painter.circle_filled(center, r * 0.45, Color32::from_rgb(0, 120, 215));
            }
            painter.text(
                Pos2::new(center.x + r + 6.0, rect.center().y),
                Align2::LEFT_CENTER,
                text,
                font,
                fore,
            );
        }
    }

    if selected {
        painter.rect_stroke(
            rect,
            CornerRadius::ZERO,
            Stroke::new(1.0, Color32::from_rgb(0, 120, 215)),
            StrokeKind::Outside,
        );
    }
}

fn paint_selection(painter: &egui::Painter, rect: Rect) {
    for handle_rect in handle_rects(rect) {
        painter.rect(
            handle_rect,
            CornerRadius::ZERO,
            Color32::WHITE,
            Stroke::new(1.0, Color32::from_rgb(0, 120, 215)),
            StrokeKind::Inside,
        );
    }
}

fn handle_rects(rect: Rect) -> [Rect; 8] {
    let s = HANDLE_SIZE;
    let c = rect.center();
    [
        Rect::from_center_size(rect.left_top(), Vec2::splat(s)),
        Rect::from_center_size(Pos2::new(c.x, rect.top()), Vec2::splat(s)),
        Rect::from_center_size(rect.right_top(), Vec2::splat(s)),
        Rect::from_center_size(Pos2::new(rect.right(), c.y), Vec2::splat(s)),
        Rect::from_center_size(rect.right_bottom(), Vec2::splat(s)),
        Rect::from_center_size(Pos2::new(c.x, rect.bottom()), Vec2::splat(s)),
        Rect::from_center_size(rect.left_bottom(), Vec2::splat(s)),
        Rect::from_center_size(Pos2::new(rect.left(), c.y), Vec2::splat(s)),
    ]
}

fn hit_handle(rect: Rect, pointer: Option<Pos2>) -> Option<ResizeHandle> {
    let p = pointer?;
    let handles = [
        ResizeHandle::Nw,
        ResizeHandle::N,
        ResizeHandle::Ne,
        ResizeHandle::E,
        ResizeHandle::Se,
        ResizeHandle::S,
        ResizeHandle::Sw,
        ResizeHandle::W,
    ];
    for (i, hr) in handle_rects(rect).iter().enumerate() {
        if hr.expand(2.0).contains(p) {
            return Some(handles[i]);
        }
    }
    None
}

fn snap_opt(v: f32, snap: bool, grid: i32) -> i32 {
    if snap && grid > 0 {
        let g = grid as f32;
        ((v / g).round() * g) as i32
    } else {
        v.round() as i32
    }
}

fn resize_rect(
    start: RectI,
    handle: ResizeHandle,
    dx: f32,
    dy: f32,
    snap: bool,
    grid: i32,
) -> RectI {
    let mut x1 = start.x as f32;
    let mut y1 = start.y as f32;
    let mut x2 = start.right() as f32;
    let mut y2 = start.bottom() as f32;

    match handle {
        ResizeHandle::Nw => {
            x1 += dx;
            y1 += dy;
        }
        ResizeHandle::N => {
            y1 += dy;
        }
        ResizeHandle::Ne => {
            x2 += dx;
            y1 += dy;
        }
        ResizeHandle::E => {
            x2 += dx;
        }
        ResizeHandle::Se => {
            x2 += dx;
            y2 += dy;
        }
        ResizeHandle::S => {
            y2 += dy;
        }
        ResizeHandle::Sw => {
            x1 += dx;
            y2 += dy;
        }
        ResizeHandle::W => {
            x1 += dx;
        }
    }

    let mut xi1 = snap_opt(x1, snap, grid);
    let mut yi1 = snap_opt(y1, snap, grid);
    let mut xi2 = snap_opt(x2, snap, grid);
    let mut yi2 = snap_opt(y2, snap, grid);

    if xi2 - xi1 < MIN_CTRL_W {
        match handle {
            ResizeHandle::Nw | ResizeHandle::W | ResizeHandle::Sw => xi1 = xi2 - MIN_CTRL_W,
            _ => xi2 = xi1 + MIN_CTRL_W,
        }
    }
    if yi2 - yi1 < MIN_CTRL_H {
        match handle {
            ResizeHandle::Nw | ResizeHandle::N | ResizeHandle::Ne => yi1 = yi2 - MIN_CTRL_H,
            _ => yi2 = yi1 + MIN_CTRL_H,
        }
    }

    RectI::new(xi1, yi1, xi2 - xi1, yi2 - yi1).clamp_size(MIN_CTRL_W, MIN_CTRL_H)
}
