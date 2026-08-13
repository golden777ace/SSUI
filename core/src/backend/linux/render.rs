use std::collections::HashMap;

use skia_safe::Image;

use super::painter::{SkiaFormats, SkiaPainter};
use super::text::SharedText;
use crate::render::paint::{ImageSource, NodeState, PaintCtx};
use crate::theme::Theme;
use crate::tree::{NodeId, NodeKind, Tree};

use crate::tree::{
    ACC_HEADER, BAR_ITEM, DOCK_HEADER, GROUP_HEADER, SCROLLBAR_W, SPLIT_ARROW, SPLIT_W,
};

/// Кэш изображений Skia по ключу источника.
#[derive(Default)]
pub struct Images {
    map: HashMap<String, Image>,
}

impl Images {
    /// Пустой кэш.
    pub fn new() -> Self {
        Self::default()
    }

    /// Кладёт готовое изображение под ключом.
    pub fn insert(&mut self, key: String, image: Image) {
        self.map.insert(key, image);
    }

    /// Загружено ли изображение по ключу.
    pub fn has(&self, key: &str) -> bool {
        self.map.contains_key(key)
    }

    /// Читает изображение с диска, если его ещё нет в кэше.
    pub fn ensure(&mut self, key: &str) {
        if self.map.contains_key(key) {
            return;
        }
        if let Some(img) = super::system::load_image(key) {
            self.map.insert(key.to_string(), img);
        }
    }

    /// Загружает все изображения, встречающиеся в дереве.
    pub fn preload(&mut self, tree: &Tree) {
        let mut stack = vec![tree.root()];
        while let Some(id) = stack.pop() {
            let node = tree.get(id);
            if let Some(icon) = &node.icon {
                self.ensure(icon);
            }
            if let NodeKind::Image { path, .. } = &node.kind {
                self.ensure(path);
            }
            stack.extend(node.children.iter().copied());
        }
    }
}

impl ImageSource for Images {
    type Image = Image;

    fn image(&self, key: &str) -> Option<&Image> {
        self.map.get(key)
    }
}

/// Состояния ввода на момент кадра.
#[derive(Clone, Copy, Default)]
pub struct Input {
    pub hovered: Option<NodeId>,
    pub pressed: Option<NodeId>,
    pub focused: Option<NodeId>,
}

impl Input {
    fn state(&self, id: NodeId, off: bool) -> NodeState {
        NodeState {
            hovered: self.hovered == Some(id),
            pressed: self.pressed == Some(id),
            focused: self.focused == Some(id),
            off,
        }
    }
}

/// Рисует всё дерево от корня; прямоугольники уже посчитаны раскладкой.
pub fn draw_tree(
    painter: &mut SkiaPainter<'_>,
    text: &mut SharedText,
    formats: &SkiaFormats,
    images: &Images,
    tree: &Tree,
    theme: Theme,
    input: Input,
) {
    let mut ctx = PaintCtx {
        painter,
        formats,
        text,
        images,
        theme,
    };
    draw_node(&mut ctx, tree, tree.root(), input);
}

fn draw_node(
    ctx: &mut PaintCtx<'_, SkiaPainter<'_>, SkiaFormats, SharedText, Images>,
    tree: &Tree,
    id: NodeId,
    input: Input,
) {
    let node = tree.get(id);
    let rect = node.rect;
    let style = node.style;
    let state = input.state(id, false);
    let icon = node.icon.as_deref();

    match &node.kind {
        NodeKind::Container | NodeKind::Stack { .. } => {}
        NodeKind::Frame { radius } => ctx.frame(rect, style, *radius),
        NodeKind::Image { path, fit } => ctx.image(rect, path, *fit),
        NodeKind::Label { text } => ctx.label(rect, style, state, text, icon),
        NodeKind::Button { label, radius } => ctx.button(
            rect,
            style,
            state,
            label,
            *radius,
            icon,
            node.style_hover.fill,
        ),
        NodeKind::Toggle { label, on } => ctx.toggle(rect, style, state, label, *on, icon),
        NodeKind::Checkbox { label, checked } => {
            ctx.checkbox(rect, style, state, label, *checked)
        }
        NodeKind::Switch { label, on } => ctx.switch(rect, style, state, label, *on),
        NodeKind::Radio { label, on, .. } => ctx.radio(rect, style, state, label, *on),
        NodeKind::Slider { value } => ctx.slider(rect, style, *value, state.focused),
        NodeKind::Range { lo, hi } => ctx.range(rect, style, *lo, *hi),
        NodeKind::Progress { value } => ctx.progress(rect, style, *value),
        NodeKind::Meter { value, segments } => ctx.meter(rect, style, *value, *segments),
        NodeKind::Chart { values } => ctx.chart(rect, style, values),
        NodeKind::Gauge { value, label } => ctx.gauge(rect, style, *value, label),
        NodeKind::Spinner { phase } => ctx.spinner(rect, style, *phase),
        NodeKind::Separator { vertical } => ctx.separator(rect, style, *vertical),
        NodeKind::Link { label } => ctx.link(rect, style, state, label),
        NodeKind::Status { text } => ctx.status(rect, style, text),
        NodeKind::MenuBar { titles, .. } => ctx.menubar(rect, style, titles, BAR_ITEM),
        NodeKind::Split { label, radius, .. } => {
            ctx.split_button(rect, style, label, *radius, SPLIT_ARROW)
        }
        NodeKind::Splitter { ratio, vertical } => {
            ctx.splitter(rect, style, *ratio, *vertical, SPLIT_W)
        }
        NodeKind::Group { title, radius } => {
            ctx.group(rect, style, title, *radius, GROUP_HEADER)
        }
        NodeKind::Accordion {
            title,
            open,
            radius,
            ..
        } => ctx.accordion(rect, style, state, title, *open, *radius, ACC_HEADER),
        NodeKind::Scroll { offset, content } => {
            ctx.scroll(rect, style, *content, *offset, SCROLLBAR_W)
        }
        NodeKind::Dock { title, open, .. } => {
            ctx.dock(rect, style, title, *open, DOCK_HEADER)
        }
        NodeKind::Drop { label } => ctx.drop_area(rect, style, label),
        _ => {}
    }

    for &child in &node.children {
        draw_node(ctx, tree, child, input);
    }
}