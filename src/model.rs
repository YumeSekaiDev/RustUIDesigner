//! Document model for the form designer (serializable, UI-agnostic).

use serde::{Deserialize, Serialize};

/// Unique control identifier. Stable for the lifetime of a control.
pub type ControlId = u64;

/// Kind of design-time control shown on the canvas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ControlKind {
    Button,
    TextBox,
    Label,
    Slider,
    DropDown,
    RadioBox,
    PictureBox,
    Timer,
}

impl ControlKind {
    pub const ALL: &'static [ControlKind] = &[
        ControlKind::Button,
        ControlKind::TextBox,
        ControlKind::Label,
        ControlKind::Slider,
        ControlKind::DropDown,
        ControlKind::RadioBox,
        ControlKind::PictureBox,
        ControlKind::Timer,
    ];

    pub fn display_name(self) -> &'static str {
        match self {
            ControlKind::Button => "Button",
            ControlKind::TextBox => "TextBox",
            ControlKind::Label => "Label",
            ControlKind::Slider => "Slider",
            ControlKind::DropDown => "DropDown",
            ControlKind::RadioBox => "RadioBox",
            ControlKind::PictureBox => "PictureBox",
            ControlKind::Timer => "Timer",
        }
    }

    pub fn name_prefix(self) -> &'static str {
        match self {
            ControlKind::Button => "Button",
            ControlKind::TextBox => "TextBox",
            ControlKind::Label => "Label",
            ControlKind::Slider => "Slider",
            ControlKind::DropDown => "DropDown",
            ControlKind::RadioBox => "Radio",
            ControlKind::PictureBox => "PictureBox",
            ControlKind::Timer => "Timer",
        }
    }

    pub fn default_size(self) -> (i32, i32) {
        match self {
            ControlKind::Button => (100, 28),
            ControlKind::TextBox => (140, 24),
            ControlKind::Label => (80, 20),
            ControlKind::Slider => (160, 24),
            ControlKind::DropDown => (140, 24),
            ControlKind::RadioBox => (120, 22),
            ControlKind::PictureBox => (120, 90),
            ControlKind::Timer => (28, 28),
        }
    }
}

/// RGBA color stored as bytes (serde-friendly).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RgbaColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl RgbaColor {
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self::new(r, g, b, 255)
    }

    pub fn to_egui(self) -> egui::Color32 {
        egui::Color32::from_rgba_unmultiplied(self.r, self.g, self.b, self.a)
    }

    pub fn from_egui(c: egui::Color32) -> Self {
        let [r, g, b, a] = c.to_array();
        Self { r, g, b, a }
    }
}

impl Default for RgbaColor {
    fn default() -> Self {
        Self::rgb(240, 240, 240)
    }
}

/// Hover animation shown in the designer canvas and the generated app.
/// Per control: set it in Properties ("Hover effect").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum HoverEffect {
    /// No hover animation (default for most controls).
    #[default]
    None,
    /// Breathing outline that expands and contracts around the control.
    Bubbly,
    /// Soft layered halo in the control's back colour, fading in on hover.
    Glow,
    /// Sonar rings that expand outwards from the control and fade away.
    Ripple,
    /// Soft drop shadow that grows under the control, as if it lifts up.
    Lift,
}

impl HoverEffect {
    pub const ALL: &'static [HoverEffect] = &[
        HoverEffect::None,
        HoverEffect::Bubbly,
        HoverEffect::Glow,
        HoverEffect::Ripple,
        HoverEffect::Lift,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            HoverEffect::None => "None",
            HoverEffect::Bubbly => "Bubbly",
            HoverEffect::Glow => "Glow",
            HoverEffect::Ripple => "Ripple",
            HoverEffect::Lift => "Lift",
        }
    }

    /// Default when a control is dropped: Bubbly for Button/TextBox only.
    pub fn default_for_kind(kind: ControlKind) -> Self {
        match kind {
            ControlKind::Button | ControlKind::TextBox => HoverEffect::Bubbly,
            _ => HoverEffect::None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum StartPosition {
    Manual,
    #[default]
    CenterScreen,
    CenterParent,
}

impl StartPosition {
    pub const ALL: &'static [StartPosition] = &[
        StartPosition::Manual,
        StartPosition::CenterScreen,
        StartPosition::CenterParent,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            StartPosition::Manual => "Manual",
            StartPosition::CenterScreen => "CenterScreen",
            StartPosition::CenterParent => "CenterParent",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum BorderStyle {
    #[default]
    Sizable,
    FixedSingle,
    FixedDialog,
    None,
}

impl BorderStyle {
    pub const ALL: &'static [BorderStyle] = &[
        BorderStyle::Sizable,
        BorderStyle::FixedSingle,
        BorderStyle::FixedDialog,
        BorderStyle::None,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            BorderStyle::Sizable => "Sizable",
            BorderStyle::FixedSingle => "FixedSingle",
            BorderStyle::FixedDialog => "FixedDialog",
            BorderStyle::None => "None",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Orientation {
    #[default]
    Horizontal,
    Vertical,
}

impl Orientation {
    pub const ALL: &'static [Orientation] =
        &[Orientation::Horizontal, Orientation::Vertical];

    pub fn as_str(self) -> &'static str {
        match self {
            Orientation::Horizontal => "Horizontal",
            Orientation::Vertical => "Vertical",
        }
    }
}

/// Axis-aligned rectangle in form (client) coordinates (integer pixels).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RectI {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl RectI {
    pub fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn right(self) -> i32 {
        self.x + self.width
    }

    pub fn bottom(self) -> i32 {
        self.y + self.height
    }

    pub fn contains(self, px: f32, py: f32) -> bool {
        px >= self.x as f32
            && px <= self.right() as f32
            && py >= self.y as f32
            && py <= self.bottom() as f32
    }

    pub fn clamp_size(mut self, min_w: i32, min_h: i32) -> Self {
        self.width = self.width.max(min_w);
        self.height = self.height.max(min_h);
        self
    }
}

/// A single design-time control on the form.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Control {
    pub id: ControlId,
    pub kind: ControlKind,
    pub name: String,
    pub text: String,
    pub rect: RectI,

    #[serde(default = "default_true")]
    pub enabled: bool,

    #[serde(default = "default_font_name")]
    pub font_name: String,

    #[serde(default = "default_font_size")]
    pub font_size: f32,

    #[serde(default = "default_ctrl_back")]
    pub back_color: RgbaColor,

    #[serde(default = "default_ctrl_fore")]
    pub fore_color: RgbaColor,

    /// RadioBox: checked state.
    #[serde(default)]
    pub checked: bool,

    /// Slider range / value.
    #[serde(default)]
    pub minimum: i32,
    #[serde(default = "default_slider_max")]
    pub maximum: i32,
    #[serde(default)]
    pub value: i32,
    #[serde(default)]
    pub orientation: Orientation,

    /// TextBox options.
    #[serde(default)]
    pub multiline: bool,
    #[serde(default)]
    pub read_only: bool,
    /// Optional single password character (e.g. `*`). `None` = normal text.
    #[serde(default)]
    pub password_char: Option<char>,

    /// PictureBox: path to the image file (may be empty = no image).
    #[serde(default)]
    pub image_path: String,

    /// Timer: tick interval in milliseconds.
    #[serde(default = "default_timer_interval_ms")]
    pub interval_ms: u32,

    /// Hover animation for this control only (None/Bubbly/Glow).
    /// Old form JSON without this field loads as None.
    #[serde(default)]
    pub hover_effect: HoverEffect,
}

fn default_true() -> bool {
    true
}
fn default_font_name() -> String {
    "Segoe UI".to_owned()
}
fn default_font_size() -> f32 {
    12.0
}
fn default_ctrl_back() -> RgbaColor {
    RgbaColor::rgb(225, 225, 225)
}
fn default_ctrl_fore() -> RgbaColor {
    RgbaColor::rgb(0, 0, 0)
}
fn default_slider_max() -> i32 {
    100
}
fn default_timer_interval_ms() -> u32 {
    1000
}

impl Control {
    pub fn new(id: ControlId, kind: ControlKind, name: String, x: i32, y: i32) -> Self {
        let (w, h) = kind.default_size();
        let (text, back, fore) = match kind {
            ControlKind::Button => (
                "Button".to_owned(),
                RgbaColor::rgb(225, 225, 225),
                RgbaColor::rgb(0, 0, 0),
            ),
            ControlKind::TextBox => (
                String::new(),
                RgbaColor::rgb(255, 255, 255),
                RgbaColor::rgb(0, 0, 0),
            ),
            ControlKind::Label => (
                "Label".to_owned(),
                RgbaColor::rgb(240, 240, 240),
                RgbaColor::rgb(0, 0, 0),
            ),
            ControlKind::Slider => (
                String::new(),
                RgbaColor::rgb(235, 235, 235),
                RgbaColor::rgb(0, 120, 215),
            ),
            ControlKind::DropDown => (
                "Option 1".to_owned(),
                RgbaColor::rgb(255, 255, 255),
                RgbaColor::rgb(0, 0, 0),
            ),
            ControlKind::RadioBox => (
                "Option".to_owned(),
                RgbaColor::rgb(240, 240, 240),
                RgbaColor::rgb(0, 0, 0),
            ),
            ControlKind::PictureBox => (
                String::new(),
                RgbaColor::rgb(255, 255, 255),
                RgbaColor::rgb(100, 100, 100),
            ),
            ControlKind::Timer => (
                String::new(),
                RgbaColor::rgb(255, 243, 200),
                RgbaColor::rgb(120, 90, 0),
            ),
        };

        Self {
            id,
            kind,
            name,
            text,
            rect: RectI::new(x, y, w, h),
            enabled: true,
            font_name: default_font_name(),
            font_size: default_font_size(),
            back_color: back,
            fore_color: fore,
            checked: false,
            minimum: 0,
            maximum: 100,
            value: 50,
            orientation: Orientation::Horizontal,
            multiline: false,
            read_only: false,
            password_char: None,
            image_path: String::new(),
            interval_ms: default_timer_interval_ms(),
            hover_effect: HoverEffect::default_for_kind(kind),
        }
    }

    pub fn clamp_slider_value(&mut self) {
        if self.maximum < self.minimum {
            std::mem::swap(&mut self.minimum, &mut self.maximum);
        }
        self.value = self.value.clamp(self.minimum, self.maximum);
    }

    pub fn slider_normalized(&self) -> f32 {
        let span = (self.maximum - self.minimum) as f32;
        if span <= 0.0 {
            0.0
        } else {
            (self.value - self.minimum) as f32 / span
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum FormTheme {
    #[default]
    Corporate,
    Cute,
    Modern,
    Cool,
    Sunny,
    Fun,
}

pub struct ThemePalette {
    pub form_back: RgbaColor,
    pub form_font_name: &'static str,
    pub form_font_size: f32,
    pub button_back: RgbaColor,
    pub button_fore: RgbaColor,
    pub input_back: RgbaColor,
    pub input_fore: RgbaColor,
    pub surface_back: RgbaColor,
    pub surface_fore: RgbaColor,
    /// Corner radius used for buttons in generated code.
    pub button_radius: u8,
    /// Corner radius for input-like controls (TextBox, DropDown, PictureBox).
    pub input_radius: u8,
    /// Button inner padding (x, y) in generated code / spacing feel.
    pub button_pad_x: f32,
    pub button_pad_y: f32,
    /// General widget spacing in the generated app.
    pub item_spacing: f32,
    /// egui animation time (s): higher = softer/bubblier hover & click.
    pub animation_secs: f32,
    pub bold_buttons: bool,
    pub uppercase_buttons: bool,
}

impl FormTheme {
    pub const ALL: &'static [FormTheme] = &[
        FormTheme::Corporate, FormTheme::Cute, FormTheme::Modern,
        FormTheme::Cool, FormTheme::Sunny, FormTheme::Fun,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            FormTheme::Corporate => "Corporate",
            FormTheme::Cute => "Cute",
            FormTheme::Modern => "Modern",
            FormTheme::Cool => "Cool",
            FormTheme::Sunny => "Sunny",
            FormTheme::Fun => "Fun",
        }
    }

    pub fn palette(self) -> ThemePalette {
        match self {
            FormTheme::Corporate => ThemePalette {
                form_back: RgbaColor::rgb(237, 241, 245), form_font_name: "Segoe UI", form_font_size: 9.0,
                button_back: RgbaColor::rgb(0, 84, 140), button_fore: RgbaColor::rgb(255, 255, 255),
                input_back: RgbaColor::rgb(255, 255, 255), input_fore: RgbaColor::rgb(30, 40, 50),
                surface_back: RgbaColor::rgb(220, 228, 236), surface_fore: RgbaColor::rgb(30, 40, 50),
                button_radius: 2, input_radius: 2, button_pad_x: 10.0, button_pad_y: 4.0, item_spacing: 6.0, animation_secs: 0.08, bold_buttons: false, uppercase_buttons: false,
            },
            FormTheme::Cute => ThemePalette {
                form_back: RgbaColor::rgb(255, 240, 246), form_font_name: "Comic Sans MS", form_font_size: 10.0,
                button_back: RgbaColor::rgb(255, 150, 185), button_fore: RgbaColor::rgb(120, 30, 60),
                input_back: RgbaColor::rgb(255, 255, 255), input_fore: RgbaColor::rgb(140, 60, 90),
                surface_back: RgbaColor::rgb(255, 220, 235), surface_fore: RgbaColor::rgb(140, 60, 90),
                button_radius: 10, input_radius: 10, button_pad_x: 14.0, button_pad_y: 7.0, item_spacing: 10.0, animation_secs: 0.30, bold_buttons: true, uppercase_buttons: false,
            },
            FormTheme::Modern => ThemePalette {
                form_back: RgbaColor::rgb(32, 34, 40), form_font_name: "Segoe UI", form_font_size: 10.0,
                button_back: RgbaColor::rgb(0, 190, 170), button_fore: RgbaColor::rgb(10, 25, 23),
                input_back: RgbaColor::rgb(50, 54, 63), input_fore: RgbaColor::rgb(235, 240, 245),
                surface_back: RgbaColor::rgb(42, 45, 53), surface_fore: RgbaColor::rgb(235, 240, 245),
                button_radius: 6, input_radius: 6, button_pad_x: 12.0, button_pad_y: 6.0, item_spacing: 8.0, animation_secs: 0.18, bold_buttons: false, uppercase_buttons: true,
            },
            FormTheme::Cool => ThemePalette {
                form_back: RgbaColor::rgb(219, 238, 252), form_font_name: "Segoe UI", form_font_size: 9.0,
                button_back: RgbaColor::rgb(28, 118, 200), button_fore: RgbaColor::rgb(255, 255, 255),
                input_back: RgbaColor::rgb(245, 251, 255), input_fore: RgbaColor::rgb(15, 60, 100),
                surface_back: RgbaColor::rgb(190, 220, 245), surface_fore: RgbaColor::rgb(15, 60, 100),
                button_radius: 6, input_radius: 4, button_pad_x: 12.0, button_pad_y: 5.0, item_spacing: 7.0, animation_secs: 0.15, bold_buttons: false, uppercase_buttons: false,
            },
            FormTheme::Sunny => ThemePalette {
                form_back: RgbaColor::rgb(255, 247, 205), form_font_name: "Verdana", form_font_size: 10.0,
                button_back: RgbaColor::rgb(255, 160, 40), button_fore: RgbaColor::rgb(90, 50, 0),
                input_back: RgbaColor::rgb(255, 253, 240), input_fore: RgbaColor::rgb(100, 70, 10),
                surface_back: RgbaColor::rgb(255, 230, 150), surface_fore: RgbaColor::rgb(100, 70, 10),
                button_radius: 8, input_radius: 8, button_pad_x: 13.0, button_pad_y: 6.0, item_spacing: 9.0, animation_secs: 0.22, bold_buttons: true, uppercase_buttons: false,
            },
            FormTheme::Fun => ThemePalette {
                form_back: RgbaColor::rgb(243, 231, 255), form_font_name: "Comic Sans MS", form_font_size: 11.0,
                button_back: RgbaColor::rgb(150, 80, 220), button_fore: RgbaColor::rgb(255, 255, 255),
                input_back: RgbaColor::rgb(255, 255, 255), input_fore: RgbaColor::rgb(80, 30, 120),
                surface_back: RgbaColor::rgb(180, 240, 200), surface_fore: RgbaColor::rgb(40, 90, 60),
                button_radius: 24, input_radius: 12, button_pad_x: 16.0, button_pad_y: 8.0, item_spacing: 12.0, animation_secs: 0.35, bold_buttons: true, uppercase_buttons: false,
            },
        }
    }
}

/// The editable form document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormDocument {
    pub title: String,
    pub width: i32,
    pub height: i32,

    #[serde(default = "default_form_back")]
    pub back_color: RgbaColor,

    #[serde(default = "default_font_name")]
    pub font_name: String,

    #[serde(default = "default_form_font_size")]
    pub font_size: f32,

    #[serde(default)]
    pub start_position: StartPosition,

    #[serde(default = "default_true")]
    pub minimize_box: bool,

    #[serde(default = "default_true")]
    pub maximize_box: bool,

    #[serde(default)]
    pub border_style: BorderStyle,

    /// Window opacity 0.0–1.0.
    #[serde(default = "default_opacity")]
    pub opacity: f32,

    /// Visual theme preset applied to this form.
    #[serde(default)]
    pub theme: FormTheme,

    /// Show only the client area: no title bar / window decorations.
    #[serde(default)]
    pub client_area_only: bool,

    /// Rounded corner radius (px) of the borderless (client-area-only) form.
    #[serde(default)]
    pub corner_radius: u8,

    /// Draw custom minimize/maximize/close buttons inside the client area,
    /// replacing the default window decorations (client-area-only forms).
    #[serde(default)]
    pub custom_caption_buttons: bool,

    pub controls: Vec<Control>,
    pub selected: Option<ControlId>,

    #[serde(default = "default_grid")]
    pub grid_size: i32,
    #[serde(default = "default_true")]
    pub show_grid: bool,
    #[serde(default = "default_true")]
    pub snap_to_grid: bool,

    pub next_id: ControlId,
}

fn default_form_back() -> RgbaColor {
    RgbaColor::rgb(240, 240, 240)
}
fn default_form_font_size() -> f32 {
    9.0
}
fn default_opacity() -> f32 {
    1.0
}
fn default_grid() -> i32 {
    8
}

impl Default for FormDocument {
    fn default() -> Self {
        Self {
            title: "Form1".to_owned(),
            width: 640,
            height: 480,
            back_color: default_form_back(),
            font_name: default_font_name(),
            font_size: default_form_font_size(),
            start_position: StartPosition::CenterScreen,
            minimize_box: true,
            maximize_box: true,
            border_style: BorderStyle::Sizable,
            opacity: 1.0,
            theme: FormTheme::default(),
            client_area_only: false,
            corner_radius: 0,
            custom_caption_buttons: false,
            controls: Vec::new(),
            selected: None,
            grid_size: 8,
            show_grid: true,
            snap_to_grid: true,
            next_id: 1,
        }
    }
}

impl FormDocument {
    pub fn new_blank() -> Self {
        Self::default()
    }

    pub fn selected_control(&self) -> Option<&Control> {
        let id = self.selected?;
        self.controls.iter().find(|c| c.id == id)
    }

    pub fn selected_control_mut(&mut self) -> Option<&mut Control> {
        let id = self.selected?;
        self.controls.iter_mut().find(|c| c.id == id)
    }

    pub fn hit_test(&self, x: f32, y: f32) -> Option<ControlId> {
        self.controls
            .iter()
            .rev()
            .find(|c| c.rect.contains(x, y))
            .map(|c| c.id)
    }

    pub fn add_control(&mut self, kind: ControlKind, x: f32, y: f32) -> ControlId {
        let id = self.next_id;
        self.next_id += 1;
        let count = self.controls.iter().filter(|c| c.kind == kind).count() + 1;
        let name = format!("{}{}", kind.name_prefix(), count);
        let (x, y) = if self.snap_to_grid {
            (snap_i(x, self.grid_size), snap_i(y, self.grid_size))
        } else {
            (x.round() as i32, y.round() as i32)
        };
        self.controls.push(Control::new(id, kind, name, x, y));
        self.selected = Some(id);
        id
    }

    pub fn delete_selected(&mut self) {
        if let Some(id) = self.selected.take() {
            self.controls.retain(|c| c.id != id);
        }
    }

    pub fn nudge_selected(&mut self, dx: i32, dy: i32) {
        if let Some(ctrl) = self.selected_control_mut() {
            ctrl.rect.x += dx;
            ctrl.rect.y += dy;
        }
    }

    pub fn snap_value(&self, v: f32) -> i32 {
        if self.snap_to_grid {
            snap_i(v, self.grid_size)
        } else {
            v.round() as i32
        }
    }

    /// Apply a theme preset: recolors the form and every control and
    /// switches fonts, so canvas and generated code update immediately.
    pub fn apply_theme(&mut self, theme: FormTheme) {
        self.theme = theme;
        let pal = theme.palette();
        self.back_color = pal.form_back;
        self.font_name = pal.form_font_name.to_owned();
        self.font_size = pal.form_font_size;
        for ctrl in &mut self.controls {
            ctrl.font_name = pal.form_font_name.to_owned();
            match ctrl.kind {
                ControlKind::Button => {
                    ctrl.back_color = pal.button_back;
                    ctrl.fore_color = pal.button_fore;
                }
                ControlKind::TextBox | ControlKind::DropDown | ControlKind::PictureBox => {
                    ctrl.back_color = pal.input_back;
                    ctrl.fore_color = pal.input_fore;
                }
                ControlKind::Label | ControlKind::Slider | ControlKind::RadioBox | ControlKind::Timer => {
                    ctrl.back_color = pal.surface_back;
                    ctrl.fore_color = pal.surface_fore;
                }
            }
        }
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

fn snap_i(v: f32, grid: i32) -> i32 {
    if grid <= 0 {
        return v.round() as i32;
    }
    let g = grid as f32;
    ((v / g).round() * g) as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picturebox_and_timer_roundtrip_and_old_json_still_loads() {
        let mut doc = FormDocument::new_blank();
        let pic = doc.add_control(ControlKind::PictureBox, 10.0, 10.0);
        doc.selected_control_mut().unwrap().image_path = "assets/cat.png".to_owned();
        let _ = pic;
        doc.add_control(ControlKind::Timer, 200.0, 10.0);
        doc.selected_control_mut().unwrap().interval_ms = 250;

        let loaded = FormDocument::from_json(&doc.to_json().unwrap()).unwrap();
        assert_eq!(loaded.controls.len(), 2);
        assert_eq!(loaded.controls[0].image_path, "assets/cat.png");
        assert_eq!(loaded.controls[1].interval_ms, 250);

        // JSON written before these fields existed must still parse.
        let old = r#"{"title":"Old","width":640,"height":480,"controls":[],"selected":null,"next_id":1}"#;
        assert!(FormDocument::from_json(old).is_ok());
    }
}

#[cfg(test)]
mod theme_tests {
    use super::*;

    #[test]
    fn apply_theme_recolors_and_roundtrips() {
        let mut doc = FormDocument::new_blank();
        doc.add_control(ControlKind::Button, 10.0, 10.0);
        doc.apply_theme(FormTheme::Cute);
        assert_eq!(doc.theme, FormTheme::Cute);
        let pal = FormTheme::Cute.palette();
        assert_eq!(doc.back_color, pal.form_back);
        assert_eq!(doc.controls[0].back_color, pal.button_back);
        assert_eq!(doc.controls[0].font_name, pal.form_font_name);

        doc.client_area_only = true;
        let loaded = FormDocument::from_json(&doc.to_json().unwrap()).unwrap();
        assert_eq!(loaded.theme, FormTheme::Cute);
        assert!(loaded.client_area_only);

        // Presets must differ in shape/motion, not just color.
        assert!(FormTheme::Corporate.palette().button_radius < FormTheme::Fun.palette().button_radius);
        assert!(FormTheme::Fun.palette().animation_secs > FormTheme::Corporate.palette().animation_secs);
        assert!(FormTheme::Modern.palette().uppercase_buttons);
        assert_ne!(
            FormTheme::Cute.palette().button_pad_y,
            FormTheme::Corporate.palette().button_pad_y
        );

        let old = r#"{"title":"Old","width":640,"height":480,"controls":[],"selected":null,"next_id":1}"#;
        let old_doc = FormDocument::from_json(old).unwrap();
        assert_eq!(old_doc.theme, FormTheme::Corporate);
        assert!(!old_doc.client_area_only);
    }
}

#[cfg(test)]
mod hover_effect_tests {
    use super::*;

    #[test]
    fn hover_effect_defaults_and_roundtrip() {
        // Bubbly by default only for Button and TextBox.
        assert_eq!(Control::new(1, ControlKind::Button, "B".into(), 0, 0).hover_effect, HoverEffect::Bubbly);
        assert_eq!(Control::new(2, ControlKind::TextBox, "T".into(), 0, 0).hover_effect, HoverEffect::Bubbly);
        for kind in [ControlKind::Label, ControlKind::Slider, ControlKind::DropDown, ControlKind::RadioBox, ControlKind::PictureBox, ControlKind::Timer] {
            assert_eq!(Control::new(3, kind, "X".into(), 0, 0).hover_effect, HoverEffect::None, "{kind:?}");
        }
        // Per-control choice survives save/load, and old JSON (no field) loads as None.
        let mut doc = FormDocument::new_blank();
        doc.add_control(ControlKind::Slider, 10.0, 10.0);
        doc.selected_control_mut().unwrap().hover_effect = HoverEffect::Glow;
        let loaded = FormDocument::from_json(&doc.to_json().unwrap()).unwrap();
        assert_eq!(loaded.controls[0].hover_effect, HoverEffect::Glow);
        let old = FormDocument::from_json(r#"{"title":"F","width":320,"height":200,"controls":[{"id":1,"kind":"Button","name":"Button1","text":"OK","rect":{"x":1,"y":2,"width":80,"height":24}}],"selected":1,"next_id":2}"#).unwrap();
        assert_eq!(old.controls[0].hover_effect, HoverEffect::None);
    }
}
