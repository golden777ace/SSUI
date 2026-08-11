use crate::backend::{FormatSource, Painter, TextEngine};
use crate::render::types::{Color, Rect};
use crate::theme::Theme;
use crate::tree::Style;

/// Кэш загруженных изображений бэкенда.
pub trait ImageSource {
    /// Тип изображения бэкенда.
    type Image;

    /// Изображение по ключу; `None`, если ещё не загружено.
    fn image(&self, key: &str) -> Option<&Self::Image>;
}

/// Состояние узла на момент отрисовки.
#[derive(Clone, Copy, Default)]
pub struct NodeState {
    pub hovered: bool,
    pub pressed: bool,
    pub focused: bool,
    /// Узел выключен модификатором `enable`.
    pub off: bool,
}

/// Контекст кадра: рисовальщик, форматы, измерение и картинки.
pub struct PaintCtx<'a, P, F, T, I>
where
    P: Painter,
    F: FormatSource<Format = P::Format>,
    T: TextEngine<Format = P::Format>,
    I: ImageSource<Image = P::Image>,
{
    pub painter: &'a mut P,
    pub formats: &'a F,
    pub text: &'a mut T,
    pub images: &'a I,
    pub theme: Theme,
}

impl<P, F, T, I> PaintCtx<'_, P, F, T, I>
where
    P: Painter,
    F: FormatSource<Format = P::Format>,
    T: TextEngine<Format = P::Format>,
    I: ImageSource<Image = P::Image>,
{
    /// Мягкая тень под скруглённым прямоугольником.
    pub fn soft_shadow(&mut self, rect: Rect, radius: f32, elev: f32) {
        if elev <= 0.0 {
            return;
        }
        let steps = (elev.round() as i32).clamp(1, 12);
        for i in (1..=steps).rev() {
            let k = i as f32;
            let spread = k * 1.5;
            let alpha = 0.05 * (1.0 - k / (steps as f32 + 1.0));
            let r = Rect::new(
                rect.x - spread,
                rect.y - spread + elev * 0.35,
                rect.width + spread * 2.0,
                rect.height + spread * 2.0,
            );
            self.painter
                .fill_rounded_rect(r, radius + spread, Color::rgba(0.0, 0.0, 0.0, alpha));
        }
    }

    /// Рисует панель: тень, градиент или сплошную заливку.
    pub fn frame(&mut self, rect: Rect, style: Style, radius: f32) {
        let rad = style.radius.unwrap_or(radius);
        if let Some(e) = style.elev {
            self.soft_shadow(rect, rad, e);
        }
        match style.grad {
            Some((a, b)) => self
                .painter
                .fill_rounded_gradient(rect, rad, a, b, style.grad_dir),
            None => {
                let fill = style.fill.unwrap_or(self.theme.surface);
                self.painter.fill_rounded_rect(rect, rad, fill);
            }
        }
    }

    /// Рисует изображение по ключу кэша.
    pub fn image(&mut self, rect: Rect, key: &str, fit: u8) {
        if let Some(bmp) = self.images.image(key) {
            self.painter.draw_bitmap(bmp, rect, fit);
        }
    }

    /// Рисует подпись с необязательной иконкой слева.
    pub fn label(&mut self, rect: Rect, style: Style, state: NodeState, text: &[u16], icon: Option<&str>) {
        let color = if state.off {
            self.theme.track
        } else {
            style.text.unwrap_or(self.theme.content)
        };
        let wrap = style.wrap == Some(true);
        let slot = if wrap { 2 } else { 1 };
        let format = self.formats.format(style, slot, false, 20.0);
        let mut tr = rect;
        if let Some(key) = icon {
            if self.images.image(key).is_some() {
                let sz = (rect.height - 6.0).clamp(12.0, 28.0);
                let iy = rect.y + (rect.height - sz) / 2.0;
                self.image(Rect::new(rect.x, iy, sz, sz), key, 0);
                let off = sz + 8.0;
                tr = Rect::new(rect.x + off, rect.y, (rect.width - off).max(0.0), rect.height);
            }
        }
        self.painter.draw_text(text, &format, tr, color);
    }

    /// Рисует кнопку-переключатель; пустой текст с иконкой даёт
    /// иконочную кнопку тулбара.
    pub fn toggle(
        &mut self,
        rect: Rect,
        style: Style,
        state: NodeState,
        label: &[u16],
        on: bool,
        icon: Option<&str>,
    ) {
        let radius = style.radius.unwrap_or(10.0);
        let icon_only = label.is_empty() && icon.is_some();
        if state.focused {
            let ring = Rect::new(
                rect.x - 3.0,
                rect.y - 3.0,
                rect.width + 6.0,
                rect.height + 6.0,
            );
            self.painter
                .fill_rounded_rect(ring, radius + 3.0, self.theme.content);
        }
        let base = if on {
            style.fill.unwrap_or(self.theme.accent)
        } else {
            style.fill.unwrap_or(self.theme.surface)
        };
        let fill = if state.pressed {
            base.darken(0.1)
        } else if state.hovered {
            base.lighten(0.1)
        } else {
            base
        };
        self.painter.fill_rounded_rect(rect, radius, fill);

        let color = if on {
            self.theme.on_accent
        } else {
            style.text.unwrap_or(self.theme.content)
        };
        let mut tr = rect;
        if let Some(key) = icon {
            if self.images.image(key).is_some() {
                if icon_only {
                    let side = (rect.width.min(rect.height) - 12.0).max(0.0);
                    let ix = rect.x + (rect.width - side) / 2.0;
                    let iy = rect.y + (rect.height - side) / 2.0;
                    self.image(Rect::new(ix, iy, side, side), key, 0);
                } else {
                    let sz = (rect.height - 10.0).clamp(12.0, 26.0);
                    let iy = rect.y + (rect.height - sz) / 2.0;
                    self.image(Rect::new(rect.x + 12.0, iy, sz, sz), key, 0);
                    let off = sz + 20.0;
                    tr = Rect::new(rect.x + off, rect.y, (rect.width - off).max(0.0), rect.height);
                }
            }
        }
        if !icon_only {
            let format = self.formats.format(style, 0, true, 24.0);
            self.painter.draw_text(label, &format, tr, color);
        }
    }

    /// Рисует кнопку; `hover_fill` — заливка из стиля наведения.
    #[allow(clippy::too_many_arguments)]
    pub fn button(
        &mut self,
        rect: Rect,
        style: Style,
        state: NodeState,
        label: &[u16],
        radius: f32,
        icon: Option<&str>,
        hover_fill: Option<Color>,
    ) {
        let rad = style.radius.unwrap_or(radius);
        let icon_only = label.is_empty() && icon.is_some();
        if !icon_only {
            if let Some(e) = style.elev {
                self.soft_shadow(rect, rad, e);
            }
        }
        let (base, hov, prs) = match style.fill {
            Some(f) => (f, f.lighten(0.1), f.darken(0.1)),
            None => (
                self.theme.accent,
                self.theme.accent_hover,
                self.theme.accent_pressed,
            ),
        };
        let hov = hover_fill.unwrap_or(hov);
        let fill = if state.pressed {
            prs
        } else if state.hovered {
            hov
        } else {
            base
        };
        let text_color = style.text.unwrap_or(self.theme.on_accent);
        if !icon_only {
            if state.focused {
                let ring = Rect::new(
                    rect.x - 3.0,
                    rect.y - 3.0,
                    rect.width + 6.0,
                    rect.height + 6.0,
                );
                self.painter
                    .fill_rounded_rect(ring, rad + 3.0, self.theme.content);
            }
            match style.grad {
                Some((a, b)) => {
                    self.painter
                        .fill_rounded_gradient(rect, rad, a, b, style.grad_dir)
                }
                None => self.painter.fill_rounded_rect(rect, rad, fill),
            }
        }
        let mut tr = rect;
        if let Some(key) = icon {
            if self.images.image(key).is_some() {
                if icon_only {
                    let pad = 4.0;
                    let r = Rect::new(
                        rect.x + pad,
                        rect.y + pad,
                        (rect.width - pad * 2.0).max(0.0),
                        (rect.height - pad * 2.0).max(0.0),
                    );
                    self.image(r, key, 0);
                } else {
                    let sz = (rect.height - 10.0).clamp(12.0, 26.0);
                    let iy = rect.y + (rect.height - sz) / 2.0;
                    self.image(Rect::new(rect.x + 12.0, iy, sz, sz), key, 0);
                    let off = sz + 20.0;
                    tr = Rect::new(rect.x + off, rect.y, (rect.width - off).max(0.0), rect.height);
                }
            }
        }
        if !icon_only {
            let format = self.formats.format(style, 0, true, 24.0);
            self.painter.draw_text(label, &format, tr, text_color);
        }
    }

    /// Рисует ползунок; `active` подсвечивает ручку.
    pub fn slider(&mut self, rect: Rect, style: Style, value: f32, active: bool) {
        let v = value.clamp(0.0, 1.0);
        let track_h = 6.0;
        let cy = rect.y + rect.height / 2.0;
        let track = Rect::new(rect.x, cy - track_h / 2.0, rect.width, track_h);
        self.painter
            .fill_rounded_rect(track, track_h / 2.0, self.theme.track);
        let filled_w = rect.width * v;
        let fill = style.fill.unwrap_or(self.theme.accent);
        let filled = Rect::new(rect.x, cy - track_h / 2.0, filled_w, track_h);
        self.painter.fill_rounded_rect(filled, track_h / 2.0, fill);
        let knob_d = 16.0;
        let hi = (rect.x + rect.width - knob_d).max(rect.x);
        let knob_x = (rect.x + filled_w - knob_d / 2.0).clamp(rect.x, hi);
        let knob = Rect::new(knob_x, cy - knob_d / 2.0, knob_d, knob_d);
        let knob_color = if active {
            self.theme.accent
        } else {
            self.theme.content
        };
        self.painter
            .fill_rounded_rect(knob, knob_d / 2.0, knob_color);
    }

    /// Рисует ссылку; при наведении или фокусе подчёркивает текст.
    pub fn link(&mut self, rect: Rect, style: Style, state: NodeState, label: &[u16]) {
        let color = style.text.unwrap_or(self.theme.accent);
        let format = self.formats.format(style, 1, false, 20.0);
        self.painter.draw_text(label, &format, rect, color);
        if state.hovered || state.focused {
            let w = self.text.width(label, &format);
            let uy = rect.y + rect.height / 2.0 + 10.0;
            self.painter.fill_rounded_rect(
                Rect::new(rect.x, uy, w.min(rect.width), 1.5),
                0.75,
                color,
            );
        }
    }

    /// Рисует флажок с подписью справа.
    pub fn checkbox(
        &mut self,
        rect: Rect,
        style: Style,
        state: NodeState,
        label: &[u16],
        checked: bool,
    ) {
        let box_d = 22.0;
        let bx = rect.x;
        let by = rect.y + (rect.height - box_d) / 2.0;
        let box_rect = Rect::new(bx, by, box_d, box_d);
        if state.focused {
            let ring = Rect::new(bx - 3.0, by - 3.0, box_d + 6.0, box_d + 6.0);
            self.painter
                .fill_rounded_rect(ring, 8.0, self.theme.content);
        }
        if checked {
            let fill = style.fill.unwrap_or(self.theme.accent);
            self.painter.fill_rounded_rect(box_rect, 5.0, fill);
            let check: Vec<u16> = "\u{2713}".encode_utf16().collect();
            let format = self.formats.format(style, 0, true, 24.0);
            self.painter
                .draw_text(&check, &format, box_rect, self.theme.on_accent);
        } else {
            self.painter
                .fill_rounded_rect(box_rect, 5.0, self.theme.track);
            let inner = Rect::new(bx + 2.0, by + 2.0, box_d - 4.0, box_d - 4.0);
            self.painter
                .fill_rounded_rect(inner, 4.0, self.theme.surface);
        }
        let label_rect = Rect::new(
            bx + box_d + 10.0,
            rect.y,
            (rect.width - box_d - 10.0).max(0.0),
            rect.height,
        );
        let color = style.text.unwrap_or(self.theme.content);
        let format = self.formats.format(style, 1, false, 20.0);
        self.painter.draw_text(label, &format, label_rect, color);
    }

    /// Рисует тумблер с подписью справа.
    pub fn switch(
        &mut self,
        rect: Rect,
        style: Style,
        state: NodeState,
        label: &[u16],
        on: bool,
    ) {
        let tw = 44.0;
        let th = 24.0;
        let tx = rect.x;
        let ty = rect.y + (rect.height - th) / 2.0;
        let track = Rect::new(tx, ty, tw, th);
        if state.focused {
            let ring = Rect::new(tx - 3.0, ty - 3.0, tw + 6.0, th + 6.0);
            self.painter
                .fill_rounded_rect(ring, th / 2.0 + 3.0, self.theme.content);
        }
        let track_col = if on {
            style.fill.unwrap_or(self.theme.accent)
        } else {
            self.theme.track
        };
        self.painter.fill_rounded_rect(track, th / 2.0, track_col);
        let kd = th - 6.0;
        let kx = if on { tx + tw - kd - 3.0 } else { tx + 3.0 };
        let ky = ty + 3.0;
        self.painter
            .fill_rounded_rect(Rect::new(kx, ky, kd, kd), kd / 2.0, self.theme.on_accent);
        let label_rect = Rect::new(
            tx + tw + 10.0,
            rect.y,
            (rect.width - tw - 10.0).max(0.0),
            rect.height,
        );
        let color = style.text.unwrap_or(self.theme.content);
        let format = self.formats.format(style, 1, false, 20.0);
        self.painter.draw_text(label, &format, label_rect, color);
    }

    /// Рисует радиокнопку с подписью справа.
    pub fn radio(&mut self, rect: Rect, style: Style, state: NodeState, label: &[u16], on: bool) {
        let d = 22.0;
        let bx = rect.x;
        let by = rect.y + (rect.height - d) / 2.0;
        let outer = Rect::new(bx, by, d, d);
        if state.focused {
            let ring = Rect::new(bx - 3.0, by - 3.0, d + 6.0, d + 6.0);
            self.painter
                .fill_rounded_rect(ring, d / 2.0 + 3.0, self.theme.content);
        }
        let border = if on {
            style.fill.unwrap_or(self.theme.accent)
        } else {
            self.theme.track
        };
        self.painter.fill_rounded_rect(outer, d / 2.0, border);
        let inner = Rect::new(bx + 2.0, by + 2.0, d - 4.0, d - 4.0);
        self.painter
            .fill_rounded_rect(inner, (d - 4.0) / 2.0, self.theme.surface);
        if on {
            let dot = 10.0;
            let dx = bx + (d - dot) / 2.0;
            let dy = by + (d - dot) / 2.0;
            let fill = style.fill.unwrap_or(self.theme.accent);
            self.painter
                .fill_rounded_rect(Rect::new(dx, dy, dot, dot), dot / 2.0, fill);
        }
        let label_rect = Rect::new(
            bx + d + 10.0,
            rect.y,
            (rect.width - d - 10.0).max(0.0),
            rect.height,
        );
        let color = style.text.unwrap_or(self.theme.content);
        let format = self.formats.format(style, 1, false, 20.0);
        self.painter.draw_text(label, &format, label_rect, color);
    }

    /// Рисует полосу прогресса.
    pub fn progress(&mut self, rect: Rect, style: Style, value: f32) {
        let v = value.clamp(0.0, 1.0);
        let bar_h = 10.0;
        let cy = rect.y + rect.height / 2.0;
        let track = Rect::new(rect.x, cy - bar_h / 2.0, rect.width, bar_h);
        self.painter
            .fill_rounded_rect(track, bar_h / 2.0, self.theme.track);
        let fill = style.fill.unwrap_or(self.theme.accent);
        let filled = Rect::new(rect.x, cy - bar_h / 2.0, rect.width * v, bar_h);
        self.painter.fill_rounded_rect(filled, bar_h / 2.0, fill);
    }

    /// Рисует разделительную линию.
    pub fn separator(&mut self, rect: Rect, style: Style, vertical: bool) {
        let col = style.fill.unwrap_or(self.theme.track);
        let th = 2.0;
        let line = if vertical {
            Rect::new(rect.x + (rect.width - th) / 2.0, rect.y, th, rect.height)
        } else {
            Rect::new(rect.x, rect.y + (rect.height - th) / 2.0, rect.width, th)
        };
        self.painter.fill_rounded_rect(line, th / 2.0, col);
    }

    /// Рисует сегментную шкалу.
    pub fn meter(&mut self, rect: Rect, style: Style, value: f32, segments: usize) {
        let n = segments.max(1);
        let bh = rect.height.min(24.0);
        let by = rect.y + (rect.height - bh) / 2.0;
        let gap = 3.0;
        let sw = ((rect.width - gap * (n - 1) as f32) / n as f32).max(1.0);
        let filled = (value.clamp(0.0, 1.0) * n as f32).round() as usize;
        let col = style.fill.unwrap_or(self.theme.accent);
        for i in 0..n {
            let x = rect.x + i as f32 * (sw + gap);
            let c = if i < filled { col } else { self.theme.track };
            self.painter
                .fill_rounded_rect(Rect::new(x, by, sw, bh), 3.0, c);
        }
    }

    /// Рисует разделитель панелей с зацепкой; `bar_w` — ширина полосы.
    pub fn splitter(&mut self, rect: Rect, style: Style, ratio: f32, vertical: bool, bar_w: f32) {
        let bar = if vertical {
            let w1 = (rect.width - bar_w) * ratio;
            Rect::new(rect.x + w1, rect.y, bar_w, rect.height)
        } else {
            let h1 = (rect.height - bar_w) * ratio;
            Rect::new(rect.x, rect.y + h1, rect.width, bar_w)
        };
        let col = style.fill.unwrap_or(self.theme.track);
        self.painter.fill_rounded_rect(bar, bar_w / 2.0, col);
        let g = 3.0;
        let grip = if vertical {
            Rect::new(
                bar.x + (bar_w - g) / 2.0,
                bar.y + bar.height / 2.0 - 16.0,
                g,
                32.0,
            )
        } else {
            Rect::new(
                bar.x + bar.width / 2.0 - 16.0,
                bar.y + (bar_w - g) / 2.0,
                32.0,
                g,
            )
        };
        self.painter
            .fill_rounded_rect(grip, g / 2.0, self.theme.content);
    }

    /// Рисует строку состояния.
    pub fn status(&mut self, rect: Rect, style: Style, text: &[u16]) {
        let fill = style.fill.unwrap_or(self.theme.surface);
        self.painter
            .fill_rounded_rect(rect, style.radius.unwrap_or(6.0), fill);
        let tr = Rect::new(rect.x + 12.0, rect.y, (rect.width - 24.0).max(0.0), rect.height);
        let col = style.text.unwrap_or(self.theme.content);
        let format = self.formats.format(style, 1, false, 20.0);
        self.painter.draw_text(text, &format, tr, col);
    }

    /// Рисует строку меню; `item_w` — ширина пункта.
    pub fn menubar(&mut self, rect: Rect, style: Style, titles: &[Vec<u16>], item_w: f32) {
        let fill = style.fill.unwrap_or(self.theme.surface);
        self.painter
            .fill_rounded_rect(rect, style.radius.unwrap_or(6.0), fill);
        let col = style.text.unwrap_or(self.theme.content);
        let format = self.formats.format(style, 0, true, 24.0);
        for (i, t) in titles.iter().enumerate() {
            let tx = rect.x + i as f32 * item_w;
            self.painter
                .draw_text(t, &format, Rect::new(tx, rect.y, item_w, rect.height), col);
        }
    }

    /// Рисует столбчатую диаграмму.
    pub fn chart(&mut self, rect: Rect, style: Style, values: &[f32]) {
        let n = values.len();
        if n == 0 {
            return;
        }
        let base = Rect::new(rect.x, rect.y + rect.height - 2.0, rect.width, 2.0);
        self.painter.fill_rounded_rect(base, 1.0, self.theme.track);
        let gap = 8.0;
        let bw = ((rect.width - gap * (n - 1) as f32) / n as f32).max(1.0);
        let max = values.iter().cloned().fold(0.0f32, f32::max).max(0.0001);
        let area = (rect.height - 6.0).max(0.0);
        let col = style.fill.unwrap_or(self.theme.accent);
        for (i, v) in values.iter().enumerate() {
            let hh = (v.max(0.0) / max * area).max(2.0);
            let x = rect.x + i as f32 * (bw + gap);
            let y = rect.y + rect.height - 2.0 - hh;
            self.painter
                .fill_rounded_rect(Rect::new(x, y, bw, hh), 4.0, col);
        }
    }

    /// Рисует диапазонный ползунок с двумя ручками.
    pub fn range(&mut self, rect: Rect, style: Style, lo: f32, hi: f32) {
        let a = lo.clamp(0.0, 1.0);
        let b = hi.clamp(a, 1.0);
        let track_h = 6.0;
        let cy = rect.y + rect.height / 2.0;
        let track = Rect::new(rect.x, cy - track_h / 2.0, rect.width, track_h);
        self.painter
            .fill_rounded_rect(track, track_h / 2.0, self.theme.track);
        let fill = style.fill.unwrap_or(self.theme.accent);
        let sel = Rect::new(
            rect.x + rect.width * a,
            cy - track_h / 2.0,
            (rect.width * (b - a)).max(0.0),
            track_h,
        );
        self.painter.fill_rounded_rect(sel, track_h / 2.0, fill);
        let kd = 16.0;
        let cap = (rect.x + rect.width - kd).max(rect.x);
        for t in [a, b] {
            let kx = (rect.x + rect.width * t - kd / 2.0).clamp(rect.x, cap);
            let knob = Rect::new(kx, cy - kd / 2.0, kd, kd);
            self.painter
                .fill_rounded_rect(knob, kd / 2.0, self.theme.content);
        }
    }

    /// Рисует вращающийся индикатор загрузки по фазе 0..1.
    pub fn spinner(&mut self, rect: Rect, style: Style, phase: f32) {
        let d = rect.width.min(rect.height).min(48.0);
        let cx = rect.x + rect.width / 2.0;
        let cy = rect.y + rect.height / 2.0;
        let radius = d / 2.0 - 4.0;
        let dot = (d / 10.0).max(2.5);
        let col = style.fill.unwrap_or(self.theme.accent);
        for i in 0..8 {
            let t = i as f32 / 8.0;
            let a = (t + phase) * std::f32::consts::TAU;
            let alpha = 0.15 + 0.85 * t;
            let px = cx + a.cos() * radius - dot / 2.0;
            let py = cy + a.sin() * radius - dot / 2.0;
            let c = Color::rgba(col.r, col.g, col.b, col.a * alpha);
            self.painter
                .fill_rounded_rect(Rect::new(px, py, dot, dot), dot / 2.0, c);
        }
    }

    /// Рисует рамку группы с заголовком; `header` — высота шапки.
    pub fn group(&mut self, rect: Rect, style: Style, title: &[u16], radius: f32, header: f32) {
        let rad = style.radius.unwrap_or(radius);
        if let Some(f) = style.fill {
            self.painter.fill_rounded_rect(rect, rad, f);
        }
        self.painter.stroke_rect(rect, 1.0, self.theme.track);
        let tr = Rect::new(
            rect.x + 14.0,
            rect.y + 2.0,
            (rect.width - 28.0).max(0.0),
            header,
        );
        let color = style.text.unwrap_or(self.theme.content);
        let format = self.formats.format(style, 1, false, 20.0);
        self.painter.draw_text(title, &format, tr, color);
    }

    /// Рисует шапку секции аккордеона со стрелкой состояния.
    #[allow(clippy::too_many_arguments)]
    pub fn accordion(
        &mut self,
        rect: Rect,
        style: Style,
        state: NodeState,
        title: &[u16],
        open: bool,
        radius: f32,
        header: f32,
    ) {
        let rad = style.radius.unwrap_or(radius);
        let fill = if state.hovered {
            style.fill.unwrap_or(self.theme.track)
        } else {
            style.fill.unwrap_or(self.theme.surface)
        };
        let head = Rect::new(rect.x, rect.y, rect.width, header);
        self.painter.fill_rounded_rect(head, rad, fill);
        let arrow: Vec<u16> = if open {
            "\u{25BC}".encode_utf16().collect()
        } else {
            "\u{25B6}".encode_utf16().collect()
        };
        let color = style.text.unwrap_or(self.theme.content);
        let format = self.formats.format(style, 1, false, 20.0);
        self.painter.draw_text(
            &arrow,
            &format,
            Rect::new(rect.x + 12.0, rect.y, 24.0, header),
            color,
        );
        self.painter.draw_text(
            title,
            &format,
            Rect::new(rect.x + 40.0, rect.y, (rect.width - 52.0).max(0.0), header),
            color,
        );
        if state.focused {
            self.painter.stroke_rect(head, 2.0, self.theme.accent);
        }
    }

    /// Рисует кнопку с меню; `arrow_w` — ширина зоны стрелки.
    pub fn split_button(
        &mut self,
        rect: Rect,
        style: Style,
        label: &[u16],
        radius: f32,
        arrow_w: f32,
    ) {
        let rad = style.radius.unwrap_or(radius);
        let fill = style.fill.unwrap_or(self.theme.accent);
        self.painter.fill_rounded_rect(rect, rad, fill);
        let text_color = style.text.unwrap_or(self.theme.on_accent);
        let format = self.formats.format(style, 0, true, 24.0);
        let main = Rect::new(rect.x, rect.y, (rect.width - arrow_w).max(0.0), rect.height);
        self.painter.draw_text(label, &format, main, text_color);
        let line = Rect::new(
            rect.x + rect.width - arrow_w,
            rect.y + 6.0,
            1.0,
            (rect.height - 12.0).max(0.0),
        );
        self.painter.fill_rounded_rect(line, 0.5, text_color);
        let arrow: Vec<u16> = "\u{25BC}".encode_utf16().collect();
        let ar = Rect::new(rect.x + rect.width - arrow_w, rect.y, arrow_w, rect.height);
        self.painter.draw_text(&arrow, &format, ar, text_color);
    }

    /// Рисует панель дока: фон и шапку со стрелкой.
    pub fn dock(&mut self, rect: Rect, style: Style, title: &[u16], open: bool, header: f32) {
        let rad = style.radius.unwrap_or(10.0);
        self.painter
            .fill_rounded_rect(rect, rad, style.fill.unwrap_or(self.theme.surface));
        let head = Rect::new(rect.x, rect.y, rect.width, header);
        self.painter.fill_rounded_rect(head, rad, self.theme.track);
        let color = style.text.unwrap_or(self.theme.content);
        let arrow: Vec<u16> = if open {
            "\u{25BE}".encode_utf16().collect()
        } else {
            "\u{25B8}".encode_utf16().collect()
        };
        let center = self.formats.format(style, 0, true, 24.0);
        self.painter.draw_text(
            &arrow,
            &center,
            Rect::new(rect.x + 4.0, rect.y, 24.0, header),
            color,
        );
        if open {
            let left = self.formats.format(style, 1, false, 20.0);
            self.painter.draw_text(
                title,
                &left,
                Rect::new(rect.x + 30.0, rect.y, (rect.width - 40.0).max(0.0), header),
                color,
            );
        }
    }

    /// Рисует зону приёма файлов: пунктирная рамка и подпись.
    pub fn drop_area(&mut self, rect: Rect, style: Style, label: &[u16]) {
        self.painter.fill_rounded_rect(
            rect,
            style.radius.unwrap_or(10.0),
            style.fill.unwrap_or(self.theme.surface),
        );
        let col = self.theme.accent;
        let dash: f32 = 10.0;
        let step = dash * 2.0;
        let mut x0 = rect.x + 6.0;
        while x0 < rect.x + rect.width - 8.0 {
            let w = dash.min(rect.x + rect.width - 8.0 - x0);
            self.painter
                .fill_rounded_rect(Rect::new(x0, rect.y + 6.0, w, 2.0), 1.0, col);
            self.painter.fill_rounded_rect(
                Rect::new(x0, rect.y + rect.height - 8.0, w, 2.0),
                1.0,
                col,
            );
            x0 += step;
        }
        let mut y0 = rect.y + 6.0;
        while y0 < rect.y + rect.height - 8.0 {
            let hh = dash.min(rect.y + rect.height - 8.0 - y0);
            self.painter
                .fill_rounded_rect(Rect::new(rect.x + 6.0, y0, 2.0, hh), 1.0, col);
            self.painter.fill_rounded_rect(
                Rect::new(rect.x + rect.width - 8.0, y0, 2.0, hh),
                1.0,
                col,
            );
            y0 += step;
        }
        let format = self.formats.format(style, 0, true, 24.0);
        self.painter.draw_text(
            label,
            &format,
            rect,
            style.text.unwrap_or(self.theme.content),
        );
    }

    /// Вертикальная полоса прокрутки внутри области; `bar_w` — толщина.
    #[allow(clippy::too_many_arguments)]
    pub fn scrollbar(
        &mut self,
        track: Rect,
        content: f32,
        visible: f32,
        scroll: f32,
        track_col: Color,
        thumb_col: Color,
        bar_w: f32,
    ) {
        if content <= visible || visible <= 0.0 {
            return;
        }
        let bar = Rect::new(
            track.x + track.width - bar_w - 2.0,
            track.y + 2.0,
            bar_w,
            (track.height - 4.0).max(0.0),
        );
        self.painter.fill_rounded_rect(bar, bar_w / 2.0, track_col);
        let ratio = (visible / content).clamp(0.05, 1.0);
        let th = (bar.height * ratio).max(24.0);
        let max_scroll = (content - visible).max(1.0);
        let t = (scroll / max_scroll).clamp(0.0, 1.0);
        let ty = bar.y + (bar.height - th) * t;
        self.painter
            .fill_rounded_rect(Rect::new(bar.x, ty, bar_w, th), bar_w / 2.0, thumb_col);
    }

    /// Две полосы прокрутки для области с содержимым `rw`×`rh`.
    #[allow(clippy::too_many_arguments)]
    pub fn scrollbars(
        &mut self,
        view: Rect,
        rw: f32,
        rh: f32,
        ox: f32,
        oy: f32,
        track_col: Color,
        thumb_col: Color,
        bar_w: f32,
    ) {
        if rh > view.height && view.height > 0.0 {
            let bar = Rect::new(
                view.x + view.width - bar_w - 2.0,
                view.y + 2.0,
                bar_w,
                (view.height - 4.0).max(0.0),
            );
            self.painter.fill_rounded_rect(bar, bar_w / 2.0, track_col);
            let th = (bar.height * (view.height / rh)).max(24.0);
            let t = (oy / (rh - view.height)).clamp(0.0, 1.0);
            let ty = bar.y + (bar.height - th) * t;
            self.painter
                .fill_rounded_rect(Rect::new(bar.x, ty, bar_w, th), bar_w / 2.0, thumb_col);
        }
        if rw > view.width && view.width > 0.0 {
            let bar = Rect::new(
                view.x + 2.0,
                view.y + view.height - bar_w - 2.0,
                (view.width - 4.0).max(0.0),
                bar_w,
            );
            self.painter.fill_rounded_rect(bar, bar_w / 2.0, track_col);
            let tw = (bar.width * (view.width / rw)).max(24.0);
            let t = (ox / (rw - view.width)).clamp(0.0, 1.0);
            let tx = bar.x + (bar.width - tw) * t;
            self.painter
                .fill_rounded_rect(Rect::new(tx, bar.y, tw, bar_w), bar_w / 2.0, thumb_col);
        }
    }

    /// Рисует область прокрутки: фон и вертикальную полосу.
    pub fn scroll(&mut self, rect: Rect, style: Style, content: f32, offset: f32, bar_w: f32) {
        if let Some(f) = style.fill {
            self.painter
                .fill_rounded_rect(rect, style.radius.unwrap_or(8.0), f);
        }
        let (track_col, thumb_col) = (self.theme.track, self.theme.content);
        self.scrollbar(
            rect,
            content,
            rect.height,
            offset,
            track_col,
            thumb_col,
            bar_w,
        );
    }

    /// Рисует круговой индикатор из точек по дуге 270°.
    pub fn gauge(&mut self, rect: Rect, style: Style, value: f32, label: &[u16]) {
        let d = rect.width.min(rect.height);
        let cx = rect.x + rect.width / 2.0;
        let cy = rect.y + rect.height / 2.0;
        let radius = d / 2.0 - 8.0;
        let th = (d / 10.0).max(4.0);
        let steps = 48;
        let start = 0.75 * std::f32::consts::TAU;
        let sweep = 0.75 * std::f32::consts::TAU;
        let filled = (steps as f32 * value.clamp(0.0, 1.0)) as usize;
        let track_col = self.theme.track;
        let fill_col = style.fill.unwrap_or(self.theme.accent);
        for i in 0..steps {
            let t = i as f32 / (steps - 1) as f32;
            let a = start + sweep * t;
            let px = cx + a.cos() * radius - th / 2.0;
            let py = cy + a.sin() * radius - th / 2.0;
            let col = if i < filled { fill_col } else { track_col };
            self.painter
                .fill_rounded_rect(Rect::new(px, py, th, th), th / 2.0, col);
        }
        let tc = style.text.unwrap_or(self.theme.content);
        let format = self.formats.format(style, 0, true, 24.0);
        self.painter.draw_text(label, &format, rect, tc);
    }

    /// Ширина строки в текущем формате; нужна для авторазмеров.
    pub fn text_width(&mut self, text: &[u16], style: Style, slot: u8, bold: bool, size: f32) -> f32 {
        let format = self.formats.format(style, slot, bold, size);
        self.text.width(text, &format)
    }
}