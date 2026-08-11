use skia_safe::{
    gradient_shader, Canvas as SkCanvas, Color4f, Image, Paint, Path, Point as SkPoint,
    Rect as SkRect, TileMode,
};

use super::text::SharedText;
use crate::backend::{GradDir, ImageFit, Painter};
use crate::render::types::{Color, Rect};

/// Формат текста: семейство, размер, начертание и выравнивание.
#[derive(Clone, Debug, PartialEq)]
pub struct TextFormat {
    pub family: String,
    pub size: f32,
    pub bold: bool,
    /// 0 — по центру, 1 — влево.
    pub align: u8,
    /// Вертикальное центрирование в прямоугольнике.
    pub middle: bool,
    pub wrap: bool,
}

impl TextFormat {
    /// Формат по семейству и размеру; остальное по умолчанию.
    pub fn new(family: &str, size: f32) -> Self {
        Self {
            family: family.to_string(),
            size: size.max(1.0),
            bold: false,
            align: 0,
            middle: true,
            wrap: false,
        }
    }
}

/// Источник форматов Skia: базовое семейство и размер приложения.
pub struct SkiaFormats {
    pub family: String,
    pub size: f32,
}

impl SkiaFormats {
    /// Берёт базовое семейство и размер из общих настроек дерева.
    pub fn from_tree() -> Self {
        Self {
            family: String::from_utf16_lossy(&crate::tree::base_font())
                .trim_end_matches('\0')
                .to_string(),
            size: crate::tree::base_size(),
        }
    }
}

impl crate::backend::FormatSource for SkiaFormats {
    type Format = TextFormat;

    fn format(
        &self,
        style: crate::tree::Style,
        slot: u8,
        bold: bool,
        default_size: f32,
    ) -> TextFormat {
        let family = match style.font {
            Some(i) => String::from_utf16_lossy(&crate::tree::font_utf16(i))
                .trim_end_matches('\0')
                .to_string(),
            None => self.family.clone(),
        };
        let size = style.size.unwrap_or(default_size).max(1.0);
        TextFormat {
            family,
            size,
            bold,
            align: if slot == 0 { 0 } else { 1 },
            middle: slot != 2,
            wrap: slot == 2,
        }
    }
}

/// Рисовальщик поверх холста Skia.
pub struct SkiaPainter<'a> {
    canvas: &'a SkCanvas,
    text: SharedText,
}

impl<'a> SkiaPainter<'a> {
    /// Оборачивает холст кадра и общий раскладчик текста.
    pub fn new(canvas: &'a SkCanvas, text: SharedText) -> Self {
        Self { canvas, text }
    }

    fn paint(color: Color) -> Paint {
        let mut p = Paint::new(Color4f::new(color.r, color.g, color.b, color.a), None);
        p.set_anti_alias(true);
        p
    }

    fn stroke(color: Color, width: f32) -> Paint {
        let mut p = Self::paint(color);
        p.set_style(skia_safe::paint::Style::Stroke);
        p.set_stroke_width(width.max(0.1));
        p
    }

    fn rect(r: Rect) -> SkRect {
        SkRect::from_xywh(r.x, r.y, r.width, r.height)
    }

    fn build_path(pts: &[(f32, f32)], closed: bool) -> Option<Path> {
        if pts.len() < 2 {
            return None;
        }
        let mut path = Path::new();
        path.move_to(SkPoint::new(pts[0].0, pts[0].1));
        for (x, y) in &pts[1..] {
            path.line_to(SkPoint::new(*x, *y));
        }
        if closed {
            path.close();
        }
        Some(path)
    }

    /// Точки дуги; шаг и правила совпадают с Windows-версией.
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
}

impl Painter for SkiaPainter<'_> {
    type Format = TextFormat;
    type Image = Image;

    fn clear(&mut self, color: Color) {
        self.canvas
            .clear(Color4f::new(color.r, color.g, color.b, color.a));
    }

    fn fill_rounded_rect(&mut self, rect: Rect, radius: f32, color: Color) {
        let p = Self::paint(color);
        self.canvas
            .draw_round_rect(Self::rect(rect), radius, radius, &p);
    }

    fn stroke_rect(&mut self, rect: Rect, width: f32, color: Color) {
        let p = Self::stroke(color, width);
        self.canvas.draw_rect(Self::rect(rect), &p);
    }

    fn fill_ellipse(&mut self, rect: Rect, color: Color) {
        let p = Self::paint(color);
        self.canvas.draw_oval(Self::rect(rect), &p);
    }

    fn stroke_ellipse(&mut self, rect: Rect, width: f32, color: Color) {
        let p = Self::stroke(color, width);
        self.canvas.draw_oval(Self::rect(rect), &p);
    }

    fn draw_line(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, width: f32, color: Color) {
        let p = Self::stroke(color, width.max(0.1));
        self.canvas
            .draw_line(SkPoint::new(x1, y1), SkPoint::new(x2, y2), &p);
    }

    fn fill_rounded_gradient(
        &mut self,
        rect: Rect,
        radius: f32,
        from: Color,
        to: Color,
        dir: GradDir,
    ) {
        let (sx, sy, ex, ey) = match dir {
            1 => (rect.x, rect.y, rect.x + rect.width, rect.y),
            2 => (rect.x, rect.y, rect.x + rect.width, rect.y + rect.height),
            3 => (rect.x, rect.y + rect.height, rect.x + rect.width, rect.y),
            _ => (rect.x, rect.y, rect.x, rect.y + rect.height),
        };
        let colors = [
            Color4f::new(from.r, from.g, from.b, from.a).to_color(),
            Color4f::new(to.r, to.g, to.b, to.a).to_color(),
        ];
        let shader = gradient_shader::linear(
            (SkPoint::new(sx, sy), SkPoint::new(ex, ey)),
            colors.as_slice(),
            None,
            TileMode::Clamp,
            None,
            None,
        );
        match shader {
            Some(s) => {
                let mut p = Paint::default();
                p.set_anti_alias(true);
                p.set_shader(s);
                self.canvas
                    .draw_round_rect(Self::rect(rect), radius, radius, &p);
            }
            None => self.fill_rounded_rect(rect, radius, from),
        }
    }

    fn fill_polygon(&mut self, pts: &[(f32, f32)], color: Color) {
        if let Some(path) = Self::build_path(pts, true) {
            let p = Self::paint(color);
            self.canvas.draw_path(&path, &p);
        }
    }

    fn stroke_polyline(&mut self, pts: &[(f32, f32)], width: f32, color: Color) {
        if let Some(path) = Self::build_path(pts, false) {
            let p = Self::stroke(color, width);
            self.canvas.draw_path(&path, &p);
        }
    }

    fn stroke_polygon(&mut self, pts: &[(f32, f32)], width: f32, color: Color) {
        if let Some(path) = Self::build_path(pts, true) {
            let p = Self::stroke(color, width);
            self.canvas.draw_path(&path, &p);
        }
    }

    fn draw_arrow(
        &mut self,
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        width: f32,
        head: f32,
        color: Color,
    ) {
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

    fn stroke_arc(
        &mut self,
        cx: f32,
        cy: f32,
        r: f32,
        start: f32,
        sweep: f32,
        width: f32,
        color: Color,
    ) {
        let pts = Self::arc_points(cx, cy, r, start, sweep);
        self.stroke_polyline(&pts, width.max(1.0), color);
    }

    fn fill_sector(&mut self, cx: f32, cy: f32, r: f32, start: f32, sweep: f32, color: Color) {
        let mut pts = vec![(cx, cy)];
        pts.extend(Self::arc_points(cx, cy, r, start, sweep));
        self.fill_polygon(&pts, color);
    }

    fn draw_text(&mut self, text: &[u16], format: &TextFormat, rect: Rect, color: Color) {
        self.text
            .0
            .borrow_mut()
            .draw(self.canvas, text, format, rect, color);
    }

    fn push_clip(&mut self, rect: Rect) {
        self.canvas.save();
        self.canvas
            .clip_rect(Self::rect(rect), skia_safe::ClipOp::Intersect, false);
    }

    fn pop_clip(&mut self) {
        self.canvas.restore();
    }

    fn draw_bitmap(&mut self, image: &Image, rect: Rect, fit: ImageFit) {
        let iw = image.width() as f32;
        let ih = image.height() as f32;
        if iw <= 0.0 || ih <= 0.0 {
            return;
        }
        let (dest, clip) = fit_dest(rect, iw, ih, fit);
        if clip {
            self.canvas.save();
            self.canvas
                .clip_rect(Self::rect(rect), skia_safe::ClipOp::Intersect, false);
        }
        let mut p = Paint::default();
        p.set_anti_alias(true);
        self.canvas.draw_image_rect(image, None, Self::rect(dest), &p);
        if clip {
            self.canvas.restore();
        }
    }
}

/// Вписывает картинку в прямоугольник; правила общие с Windows.
fn fit_dest(rect: Rect, iw: f32, ih: f32, fit: ImageFit) -> (Rect, bool) {
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