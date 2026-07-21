use windows::Win32::Graphics::Direct2D::Common::*;
use windows::Win32::Graphics::Direct2D::*;
use windows::Win32::Graphics::DirectWrite::*;
use windows::core::Interface;
use windows::Win32::Graphics::Direct2D::{ID2D1Factory, ID2D1PathGeometry};

use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use hashbrown::Equivalent;

use super::types::{Color, Rect};

pub type LayoutCache = hashbrown::HashMap<TextKey, IDWriteTextLayout>;

struct KeyRef<'a> {
    text: &'a [u16],
    fmt: usize,
    w: u32,
    h: u32,
}

impl Hash for KeyRef<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.text.hash(state);
        self.fmt.hash(state);
        self.w.hash(state);
        self.h.hash(state);
    }
}

impl Equivalent<TextKey> for KeyRef<'_> {
    fn equivalent(&self, k: &TextKey) -> bool {
        self.fmt == k.fmt && self.w == k.w && self.h == k.h && self.text == k.text.as_slice()
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct TextKey {
    text: Vec<u16>,
    fmt: usize,
    w: u32,
    h: u32,
}

pub struct Canvas<'a> {
    rt: &'a ID2D1RenderTarget,
    brush: Option<ID2D1SolidColorBrush>,
    grad: &'a RefCell<HashMap<(u32, u32), ID2D1LinearGradientBrush>>,
    layouts: &'a RefCell<LayoutCache>,
    dwrite: &'a IDWriteFactory,
}

impl<'a> Canvas<'a> {
    pub fn new(
        rt: &'a ID2D1RenderTarget,
        grad: &'a RefCell<HashMap<(u32, u32), ID2D1LinearGradientBrush>>,
        layouts: &'a RefCell<LayoutCache>,
        dwrite: &'a IDWriteFactory,
    ) -> Self {
        let c = Color::rgb(0.0, 0.0, 0.0).to_d2d();
        let brush = unsafe { rt.CreateSolidColorBrush(&c, None).ok() };
        Self {
            rt,
            brush,
            grad,
            layouts,
            dwrite,
        }
    }

    fn solid(&self, color: Color) -> Option<&ID2D1SolidColorBrush> {
        let b = self.brush.as_ref()?;
        unsafe { b.SetColor(&color.to_d2d()) };
        Some(b)
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
        if let Some(brush) = self.solid(color) {
            let rr = D2D1_ROUNDED_RECT {
                rect: rect.to_d2d(),
                radiusX: radius,
                radiusY: radius,
            };
            unsafe { self.rt.FillRoundedRectangle(&rr, brush) };
        }
    }

    /// Рисует контур прямоугольника заданной толщины.
    pub fn stroke_rect(&self, rect: Rect, width: f32, color: Color) {
        let r = rect.to_d2d();
        if let Some(brush) = self.solid(color) {
            unsafe { self.rt.DrawRectangle(&r, brush, width, None) };
        }
    }

    /// Рисует отрезок из (x1,y1) в (x2,y2) толщиной `width`.
    pub fn draw_line(&self, x1: f32, y1: f32, x2: f32, y2: f32, width: f32, color: Color) {
        if let Some(brush) = self.solid(color) {
            let mut p1 = D2D1_ELLIPSE::default().point;
            p1.X = x1;
            p1.Y = y1;
            let mut p2 = D2D1_ELLIPSE::default().point;
            p2.X = x2;
            p2.Y = y2;
            unsafe { self.rt.DrawLine(p1, p2, brush, width.max(0.1), None) };
        }
    }

    /// Заливает эллипс, вписанный в прямоугольник `rect`.
    pub fn fill_ellipse(&self, rect: Rect, color: Color) {
        if let Some(brush) = self.solid(color) {
            let mut e = D2D1_ELLIPSE::default();
            e.point.X = rect.x + rect.width / 2.0;
            e.point.Y = rect.y + rect.height / 2.0;
            e.radiusX = rect.width / 2.0;
            e.radiusY = rect.height / 2.0;
            unsafe { self.rt.FillEllipse(&e, brush) };
        }
    }

    /// Рисует контур эллипса, вписанного в `rect`, толщиной `width`.
    pub fn stroke_ellipse(&self, rect: Rect, width: f32, color: Color) {
        if let Some(brush) = self.solid(color) {
            let mut e = D2D1_ELLIPSE::default();
            e.point.X = rect.x + rect.width / 2.0;
            e.point.Y = rect.y + rect.height / 2.0;
            e.radiusX = rect.width / 2.0;
            e.radiusY = rect.height / 2.0;
            unsafe { self.rt.DrawEllipse(&e, brush, width.max(0.1), None) };
        }
    }

    /// Заливает скруглённый прямоугольник линейным градиентом по направлению `dir`.
    pub fn fill_rounded_gradient(&self, rect: Rect, radius: f32, c0: Color, c1: Color, dir: u8) {
        let (sx, sy, ex, ey) = match dir {
            1 => (rect.x, rect.y, rect.x + rect.width, rect.y),
            2 => (rect.x, rect.y, rect.x + rect.width, rect.y + rect.height),
            3 => (rect.x, rect.y + rect.height, rect.x + rect.width, rect.y),
            _ => (rect.x, rect.y, rect.x, rect.y + rect.height),
        };
        let key = (c0.pack(), c1.pack());
        let brush = {
            let mut cache = self.grad.borrow_mut();
            match cache.get(&key) {
                Some(b) => b.clone(),
                None => match self.make_gradient(c0, c1) {
                    Some(b) => {
                        cache.insert(key, b.clone());
                        b
                    }
                    None => {
                        self.fill_rounded_rect(rect, radius, c0);
                        return;
                    }
                },
            }
        };
        unsafe {
            let mut sp = D2D1_ELLIPSE::default().point;
            sp.X = sx;
            sp.Y = sy;
            let mut ep = D2D1_ELLIPSE::default().point;
            ep.X = ex;
            ep.Y = ey;
            brush.SetStartPoint(sp);
            brush.SetEndPoint(ep);
            let rr = D2D1_ROUNDED_RECT {
                rect: rect.to_d2d(),
                radiusX: radius,
                radiusY: radius,
            };
            self.rt.FillRoundedRectangle(&rr, &brush);
        }
    }

    fn make_gradient(&self, c0: Color, c1: Color) -> Option<ID2D1LinearGradientBrush> {
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
            let coll = self
                .rt
                .CreateGradientStopCollection(&stops, D2D1_GAMMA_2_2, D2D1_EXTEND_MODE_CLAMP)
                .ok()?;
            let props = D2D1_LINEAR_GRADIENT_BRUSH_PROPERTIES::default();
            self.rt.CreateLinearGradientBrush(&props, None, &coll).ok()
        }
    }

    /// Рисует строку текста внутри прямоугольника `rect`.
    pub fn draw_text(&self, text: &[u16], format: &IDWriteTextFormat, rect: Rect, color: Color) {
        if text.is_empty() {
            return;
        }
        let brush = match self.solid(color) {
            Some(b) => b,
            None => return,
        };
        let w = rect.width.max(1.0);
        let h = rect.height.max(1.0);
        let kref = KeyRef {
            text,
            fmt: format.as_raw() as usize,
            w: w.to_bits(),
            h: h.to_bits(),
        };
        let layout = {
            let mut cache = self.layouts.borrow_mut();
            match cache.get(&kref) {
                Some(l) => l.clone(),
                None => {
                    let made =
                        unsafe { self.dwrite.CreateTextLayout(text, format, w, h).ok() };
                    match made {
                        Some(l) => {
                            if cache.len() >= 8192 {
                                cache.clear();
                            }
                            let key = TextKey {
                                text: text.to_vec(),
                                fmt: kref.fmt,
                                w: kref.w,
                                h: kref.h,
                            };
                            cache.insert(key, l.clone());
                            l
                        }
                        None => return,
                    }
                }
            }
        };
        let mut origin = D2D1_ELLIPSE::default().point;
        origin.X = rect.x;
        origin.Y = rect.y;
        unsafe {
            self.rt
                .DrawTextLayout(origin, &layout, brush, D2D1_DRAW_TEXT_OPTIONS_NONE);
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

    fn path(&self, pts: &[(f32, f32)], closed: bool, filled: bool) -> Option<ID2D1PathGeometry> {
        if pts.len() < 2 {
            return None;
        }
        unsafe {
            let factory: ID2D1Factory = self.rt.GetFactory().ok()?;
            let geo = factory.CreatePathGeometry().ok()?;
            let sink = geo.Open().ok()?;
            let begin = if filled {
                D2D1_FIGURE_BEGIN_FILLED
            } else {
                D2D1_FIGURE_BEGIN_HOLLOW
            };
            let mut e = D2D1_ELLIPSE::default();
            e.point.X = pts[0].0;
            e.point.Y = pts[0].1;
            sink.BeginFigure(e.point, begin);
            for (x, y) in &pts[1..] {
                e.point.X = *x;
                e.point.Y = *y;
                sink.AddLine(e.point);
            }
            sink.EndFigure(if closed {
                D2D1_FIGURE_END_CLOSED
            } else {
                D2D1_FIGURE_END_OPEN
            });
            let _ = sink.Close();
            Some(geo)
        }
    }

    /// Заливает произвольный многоугольник по списку вершин.
    pub fn fill_polygon(&self, pts: &[(f32, f32)], color: Color) {
        if let (Some(geo), Some(brush)) = (self.path(pts, true, true), self.solid(color)) {
            unsafe { self.rt.FillGeometry(&geo, brush, None) };
        }
    }

    /// Рисует ломаную по списку вершин заданной толщины.
    pub fn stroke_polyline(&self, pts: &[(f32, f32)], width: f32, color: Color) {
        if let (Some(geo), Some(brush)) = (self.path(pts, false, false), self.solid(color)) {
            unsafe { self.rt.DrawGeometry(&geo, brush, width, None) };
        }
    }

    /// Рисует замкнутый контур многоугольника заданной толщины.
    pub fn stroke_polygon(&self, pts: &[(f32, f32)], width: f32, color: Color) {
        if let (Some(geo), Some(brush)) = (self.path(pts, true, false), self.solid(color)) {
            unsafe { self.rt.DrawGeometry(&geo, brush, width.max(0.1), None) };
        }
    }

    /// Рисует стрелку из (x1,y1) в (x2,y2); `head` — длина наконечника.
    pub fn draw_arrow(&self, x1: f32, y1: f32, x2: f32, y2: f32, width: f32, head: f32, color: Color) {
        let dx = x2 - x1;
        let dy = y2 - y1;
        let len = (dx * dx + dy * dy).sqrt().max(0.001);
        let (ux, uy) = (dx / len, dy / len);
        let hl = if head > 0.0 { head } else { 12.0 };
        let hw = hl * 0.5;
        let bx = x2 - ux * hl;
        let by = y2 - uy * hl;
        self.draw_line(x1, y1, bx, by, width.max(1.0), color);
        self.fill_polygon(
            &[
                (x2, y2),
                (bx - uy * hw, by + ux * hw),
                (bx + uy * hw, by - ux * hw),
            ],
            color,
        );
    }

    fn arc_points(cx: f32, cy: f32, r: f32, start: f32, sweep: f32) -> Vec<(f32, f32)> {
        let steps = ((sweep.abs() / 4.0).ceil() as usize).clamp(2, 180);
        let a0 = start.to_radians();
        let da = sweep.to_radians() / steps as f32;
        (0..=steps)
            .map(|i| {
                let a = a0 + da * i as f32;
                (cx + r * a.cos(), cy + r * a.sin())
            })
            .collect()
    }

    /// Рисует дугу: центр, радиус, начальный угол и разворот в градусах.
    pub fn stroke_arc(&self, cx: f32, cy: f32, r: f32, start: f32, sweep: f32, width: f32, color: Color) {
        let pts = Self::arc_points(cx, cy, r, start, sweep);
        self.stroke_polyline(&pts, width.max(1.0), color);
    }

    /// Заливает сектор: центр, радиус, начальный угол и разворот в градусах.
    pub fn fill_sector(&self, cx: f32, cy: f32, r: f32, start: f32, sweep: f32, color: Color) {
        let mut pts = vec![(cx, cy)];
        pts.extend(Self::arc_points(cx, cy, r, start, sweep));
        self.fill_polygon(&pts, color);
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