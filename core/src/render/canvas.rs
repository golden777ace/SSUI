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

    /// Заливает скруглённый прямоугольник линейным градиентом по направлению `dir`.
    pub fn fill_rounded_gradient(&self, rect: Rect, radius: f32, c0: Color, c1: Color, dir: u8) {
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
            let (sx, sy, ex, ey) = match dir {
                1 => (rect.x, rect.y, rect.x + rect.width, rect.y),
                2 => (rect.x, rect.y, rect.x + rect.width, rect.y + rect.height),
                3 => (rect.x, rect.y + rect.height, rect.x + rect.width, rect.y),
                _ => (rect.x, rect.y, rect.x, rect.y + rect.height),
            };
            let mut props = D2D1_LINEAR_GRADIENT_BRUSH_PROPERTIES::default();
            props.startPoint.X = sx;
            props.startPoint.Y = sy;
            props.endPoint.X = ex;
            props.endPoint.Y = ey;
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

    /// Рисует битмап в прямоугольник по режиму `fit`.
    pub fn draw_bitmap(&self, bitmap: &ID2D1Bitmap, rect: Rect, fit: u8) {
        unsafe {
            let size = bitmap.GetSize();
            if size.width <= 0.0 || size.height <= 0.0 {
                return;
            }
            let (dest, clip) = fit_dest(rect, size.width, size.height, fit);
            if clip {
                self.rt
                    .PushAxisAlignedClip(&rect.to_d2d(), D2D1_ANTIALIAS_MODE_ALIASED);
            }
            self.rt.DrawBitmap(
                bitmap,
                Some(&dest.to_d2d()),
                1.0,
                D2D1_BITMAP_INTERPOLATION_MODE_LINEAR,
                None,
            );
            if clip {
                self.rt.PopAxisAlignedClip();
            }
        }
    }
}

fn fit_dest(rect: Rect, iw: f32, ih: f32, fit: u8) -> (Rect, bool) {
    match fit {
        2 => (rect, false),
        3 => {
            let x = rect.x + (rect.width - iw) / 2.0;
            let y = rect.y + (rect.height - ih) / 2.0;
            (Rect::new(x, y, iw, ih), true)
        }
        1 => {
            let s = (rect.width / iw).max(rect.height / ih);
            let w = iw * s;
            let h = ih * s;
            let x = rect.x + (rect.width - w) / 2.0;
            let y = rect.y + (rect.height - h) / 2.0;
            (Rect::new(x, y, w, h), true)
        }
        _ => {
            let s = (rect.width / iw).min(rect.height / ih);
            let w = iw * s;
            let h = ih * s;
            let x = rect.x + (rect.width - w) / 2.0;
            let y = rect.y + (rect.height - h) / 2.0;
            (Rect::new(x, y, w, h), false)
        }
    }
}