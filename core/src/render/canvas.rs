use windows::Win32::Graphics::Direct2D::Common::*;
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

    /// Рисует контур прямоугольника заданной толщины.
    pub fn stroke_rect(&self, rect: Rect, width: f32, color: Color) {
        let c = color.to_d2d();
        let r = rect.to_d2d();
        unsafe {
            if let Ok(brush) = self.rt.CreateSolidColorBrush(&c, None) {
                self.rt.DrawRectangle(&r, &brush, width, None);
            }
        }
    }

    /// Заливает скруглённый прямоугольник вертикальным градиентом.
    pub fn fill_rounded_gradient(&self, rect: Rect, radius: f32, c0: Color, c1: Color) {
        unsafe {
            let stops = [
                D2D1_GRADIENT_STOP {
                    position: 0.0,
                    color: c0.to_d2d(),
                },
                D2D1_GRADIENT_STOP {
                    position: 1.0,
                    color: c1.to_d2d(),
                },
            ];
            let coll = match self.rt.CreateGradientStopCollection(
                &stops,
                D2D1_GAMMA_2_2,
                D2D1_EXTEND_MODE_CLAMP,
            ) {
                Ok(c) => c,
                Err(_) => {
                    self.fill_rounded_rect(rect, radius, c0);
                    return;
                }
            };
            let mut props = D2D1_LINEAR_GRADIENT_BRUSH_PROPERTIES::default();
            props.startPoint.X = rect.x;
            props.startPoint.Y = rect.y;
            props.endPoint.X = rect.x;
            props.endPoint.Y = rect.y + rect.height;
            match self.rt.CreateLinearGradientBrush(&props, None, &coll) {
                Ok(brush) => {
                    let rr = D2D1_ROUNDED_RECT {
                        rect: rect.to_d2d(),
                        radiusX: radius,
                        radiusY: radius,
                    };
                    self.rt.FillRoundedRectangle(&rr, &brush);
                }
                Err(_) => self.fill_rounded_rect(rect, radius, c0),
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

    /// Ограничивает последующую отрисовку прямоугольником `rect`.
    pub fn push_clip(&self, rect: Rect) {
        let r = rect.to_d2d();
        unsafe {
            self.rt
                .PushAxisAlignedClip(&r, D2D1_ANTIALIAS_MODE_ALIASED);
        }
    }

    /// Снимает последнее ограничение отрисовки.
    pub fn pop_clip(&self) {
        unsafe {
            self.rt.PopAxisAlignedClip();
        }
    }
}