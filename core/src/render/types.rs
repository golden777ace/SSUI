#[cfg(windows)]
use windows::Win32::Graphics::Direct2D::Common::{D2D1_COLOR_F, D2D_RECT_F};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    pub fn hex(value: u32) -> Self {
        Self {
            r: ((value >> 16) & 0xFF) as f32 / 255.0,
            g: ((value >> 8) & 0xFF) as f32 / 255.0,
            b: (value & 0xFF) as f32 / 255.0,
            a: 1.0,
        }
    }

    /// Цвет из `0xRRGGBBAA` с альфа-каналом.
    pub fn hexa(value: u32) -> Self {
        Self {
            r: ((value >> 24) & 0xFF) as f32 / 255.0,
            g: ((value >> 16) & 0xFF) as f32 / 255.0,
            b: ((value >> 8) & 0xFF) as f32 / 255.0,
            a: (value & 0xFF) as f32 / 255.0,
        }
    }

    /// Упаковывает цвет в `0xRRGGBBAA`.
    pub fn pack(self) -> u32 {
        let r = (self.r.clamp(0.0, 1.0) * 255.0).round() as u32;
        let g = (self.g.clamp(0.0, 1.0) * 255.0).round() as u32;
        let b = (self.b.clamp(0.0, 1.0) * 255.0).round() as u32;
        let a = (self.a.clamp(0.0, 1.0) * 255.0).round() as u32;
        (r << 24) | (g << 16) | (b << 8) | a
    }

    pub fn lighten(self, amount: f32) -> Self {
        Self {
            r: self.r + (1.0 - self.r) * amount,
            g: self.g + (1.0 - self.g) * amount,
            b: self.b + (1.0 - self.b) * amount,
            a: self.a,
        }
    }

    pub fn darken(self, amount: f32) -> Self {
        Self {
            r: self.r * (1.0 - amount),
            g: self.g * (1.0 - amount),
            b: self.b * (1.0 - amount),
            a: self.a,
        }
    }

    #[cfg(windows)]
    pub(crate) fn to_d2d(self) -> D2D1_COLOR_F {
        D2D1_COLOR_F {
            r: self.r,
            g: self.g,
            b: self.b,
            a: self.a,
        }
    }
}

/// Разбирает `#RRGGBB`, `#RRGGBBAA` или `#RGB` в цвет.
pub fn parse_hex(value: &str) -> Option<Color> {
    let hex = value.trim().strip_prefix('#')?;
    match hex.len() {
        8 => u32::from_str_radix(hex, 16).ok().map(Color::hexa),
        6 => u32::from_str_radix(hex, 16).ok().map(Color::hex),
        3 => {
            let mut full = String::new();
            for ch in hex.chars() {
                full.push(ch);
                full.push(ch);
            }
            u32::from_str_radix(&full, 16).ok().map(Color::hex)
        }
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Содержит ли прямоугольник точку `(x, y)`.
    pub fn contains(self, x: f32, y: f32) -> bool {
        x >= self.x && x <= self.x + self.width && y >= self.y && y <= self.y + self.height
    }

    #[cfg(windows)]
    pub(crate) fn to_d2d(self) -> D2D_RECT_F {
        D2D_RECT_F {
            left: self.x,
            top: self.y,
            right: self.x + self.width,
            bottom: self.y + self.height,
        }
    }
}