use windows::Win32::Graphics::Direct2D::*;
use windows::Win32::Graphics::DirectWrite::*;

use super::types::{Color, Rect};

pub struct Canvas<'a> {
    rt: &'a ID2D1RenderTarget,
}

impl<'a> Canvas<'a> {
    pub fn new(rt: &'a ID2D1RenderTarget) -> Self {
        Self { rt }
    }

    /// Заливает всю поверхность указанным цветом.
    pub fn clear(&self, color: Color) {
        let c = color.to_d2d();
        unsafe {
            self.rt.Clear(Some(&c));
        }
    }

    /// Рисует залитый прямоугольник со скруглёнными углами.
    pub fn fill_rounded_rect(&self, rect: Rect, radius: f32, color: Color) {
        let c = color.to_d2d();
        unsafe {
            if let Ok(brush) = self.rt.CreateSolidColorBrush(&c, None) {
                let rr = D2D1_ROUNDED_RECT {
                    rect: rect.to_d2d(),
                    radiusX: radius,
                    radiusY: radius,
                };
                self.rt.FillRoundedRectangle(&rr, &brush);
            }
        }
    }

    /// Рисует строку текста внутри прямоугольника `rect`.
    pub fn draw_text(&self, text: &[u16], format: &IDWriteTextFormat, rect: Rect, color: Color) {
        let c = color.to_d2d();
        let layout = rect.to_d2d();
        unsafe {
            if let Ok(brush) = self.rt.CreateSolidColorBrush(&c, None) {
                self.rt.DrawText(
                    text,
                    format,
                    &layout,
                    &brush,
                    D2D1_DRAW_TEXT_OPTIONS_NONE,
                    DWRITE_MEASURING_MODE_NATURAL,
                );
            }
        }
    }
}
