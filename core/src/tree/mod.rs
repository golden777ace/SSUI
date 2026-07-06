use crate::render::types::{Color, Rect};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct NodeId(usize);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Axis {
    Vertical,
    Horizontal,
}

#[derive(Clone, Copy)]
pub struct Props {
    pub axis: Axis,
    pub padding: f32,
    pub gap: f32,
    pub width: Option<f32>,
    pub height: Option<f32>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            axis: Axis::Vertical,
            padding: 0.0,
            gap: 0.0,
            width: None,
            height: None,
        }
    }
}

#[derive(Clone, Copy, Default)]
pub struct Style {
    pub fill: Option<Color>,
    pub text: Option<Color>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Action {
    #[default]
    None,
    Increment,
    Decrement,
    Toggle,
}

#[derive(Clone)]
pub struct TextState {
    pub text: Vec<u16>,
    pub caret: usize,
    pub anchor: usize,
    pub scroll: f32,
    undo: Vec<(Vec<u16>, usize)>,
    redo: Vec<(Vec<u16>, usize)>,
}

impl TextState {
    pub fn new() -> Self {
        Self {
            text: Vec::new(),
            caret: 0,
            anchor: 0,
            scroll: 0.0,
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }

    /// Возвращает границы выделения `(начало, конец)`.
    pub fn sel_range(&self) -> (usize, usize) {
        (self.caret.min(self.anchor), self.caret.max(self.anchor))
    }

    fn snapshot(&mut self) {
        self.undo.push((self.text.clone(), self.caret));
        if self.undo.len() > 100 {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    fn delete_range(&mut self) -> bool {
        let (a, b) = self.sel_range();
        if a != b {
            self.text.drain(a..b);
            self.caret = a;
            self.anchor = a;
            true
        } else {
            false
        }
    }

    /// Вставляет символы в позицию каретки, заменяя выделение.
    pub fn insert(&mut self, chars: &[u16]) {
        self.snapshot();
        self.delete_range();
        for (i, &c) in chars.iter().enumerate() {
            self.text.insert(self.caret + i, c);
        }
        self.caret += chars.len();
        self.anchor = self.caret;
    }

    /// Удаляет символ перед кареткой или выделение.
    pub fn backspace(&mut self) {
        self.snapshot();
        if !self.delete_range() && self.caret > 0 {
            self.text.remove(self.caret - 1);
            self.caret -= 1;
        }
        self.anchor = self.caret;
    }

    /// Удаляет символ после каретки или выделение.
    pub fn delete_forward(&mut self) {
        self.snapshot();
        if !self.delete_range() && self.caret < self.text.len() {
            self.text.remove(self.caret);
        }
        self.anchor = self.caret;
    }

    /// Двигает каретку влево (с расширением выделения при `extend`).
    pub fn move_left(&mut self, extend: bool) {
        if !extend && self.caret != self.anchor {
            self.caret = self.sel_range().0;
        } else if self.caret > 0 {
            self.caret -= 1;
        }
        if !extend {
            self.anchor = self.caret;
        }
    }

    /// Двигает каретку вправо (с расширением выделения при `extend`).
    pub fn move_right(&mut self, extend: bool) {
        if !extend && self.caret != self.anchor {
            self.caret = self.sel_range().1;
        } else if self.caret < self.text.len() {
            self.caret += 1;
        }
        if !extend {
            self.anchor = self.caret;
        }
    }

    /// Переносит каретку в начало.
    pub fn home(&mut self, extend: bool) {
        self.caret = 0;
        if !extend {
            self.anchor = 0;
        }
    }

    /// Переносит каретку в конец.
    pub fn end(&mut self, extend: bool) {
        self.caret = self.text.len();
        if !extend {
            self.anchor = self.caret;
        }
    }

    /// Выделяет весь текст.
    pub fn select_all(&mut self) {
        self.anchor = 0;
        self.caret = self.text.len();
    }

    /// Устанавливает каретку по индексу.
    pub fn set_caret(&mut self, index: usize, extend: bool) {
        self.caret = index.min(self.text.len());
        if !extend {
            self.anchor = self.caret;
        }
    }

    /// Отменяет последнее изменение.
    pub fn undo(&mut self) {
        if let Some((t, c)) = self.undo.pop() {
            self.redo.push((self.text.clone(), self.caret));
            self.text = t;
            self.caret = c.min(self.text.len());
            self.anchor = self.caret;
        }
    }

    /// Повторяет отменённое изменение.
    pub fn redo(&mut self) {
        if let Some((t, c)) = self.redo.pop() {
            self.undo.push((self.text.clone(), self.caret));
            self.text = t;
            self.caret = c.min(self.text.len());
            self.anchor = self.caret;
        }
    }
}

impl Default for TextState {
    fn default() -> Self {
        Self::new()
    }
}

pub enum NodeKind {
    Container,
    Frame { radius: f32 },
    Label { text: Vec<u16> },
    Button { label: Vec<u16>, radius: f32 },
    Slider { value: f32 },
    Checkbox { label: Vec<u16>, checked: bool },
    TextBox { state: TextState },
}

pub struct Node {
    pub parent: Option<NodeId>,
    pub children: Vec<NodeId>,
    pub rect: Rect,
    pub kind: NodeKind,
    pub props: Props,
    pub style: Style,
    pub action: Action,
}

pub struct Tree {
    nodes: Vec<Node>,
    root: NodeId,
}

impl Tree {
    /// Создаёт дерево с пустым корневым контейнером.
    pub fn new() -> Self {
        let root = Node {
            parent: None,
            children: Vec::new(),
            rect: Rect::new(0.0, 0.0, 0.0, 0.0),
            kind: NodeKind::Container,
            props: Props::default(),
            style: Style::default(),
            action: Action::default(),
        };
        Self {
            nodes: vec![root],
            root: NodeId(0),
        }
    }

    /// Идентификатор корневого узла.
    pub fn root(&self) -> NodeId {
        self.root
    }

    /// Задаёт свойства раскладки для узла.
    pub fn set_props(&mut self, id: NodeId, props: Props) {
        self.nodes[id.0].props = props;
    }

    /// Переопределяет цвета элемента поверх темы.
    pub fn set_style(&mut self, id: NodeId, style: Style) {
        self.nodes[id.0].style = style;
    }

    /// Меняет текст у узла-метки.
    pub fn set_label_text(&mut self, id: NodeId, text: Vec<u16>) {
        if let NodeKind::Label { text: t } = &mut self.nodes[id.0].kind {
            *t = text;
        }
    }

    /// Добавляет узел ребёнком к `parent` и возвращает его идентификатор.
    pub fn add_child(&mut self, parent: NodeId, kind: NodeKind, props: Props) -> NodeId {
        let id = NodeId(self.nodes.len());
        self.nodes.push(Node {
            parent: Some(parent),
            children: Vec::new(),
            rect: Rect::new(0.0, 0.0, 0.0, 0.0),
            kind,
            props,
            style: Style::default(),
            action: Action::default(),
        });
        self.nodes[parent.0].children.push(id);
        id
    }

    /// Возвращает узел по идентификатору.
    pub fn get(&self, id: NodeId) -> &Node {
        &self.nodes[id.0]
    }

    /// Является ли узел кнопкой.
    pub fn is_button(&self, id: NodeId) -> bool {
        matches!(self.nodes[id.0].kind, NodeKind::Button { .. })
    }

    /// Является ли узел чекбоксом.
    pub fn is_checkbox(&self, id: NodeId) -> bool {
        matches!(self.nodes[id.0].kind, NodeKind::Checkbox { .. })
    }

    /// Является ли узел полем ввода.
    pub fn is_textbox(&self, id: NodeId) -> bool {
        matches!(self.nodes[id.0].kind, NodeKind::TextBox { .. })
    }

    /// Реагирует ли узел на клик (кнопка или чекбокс).
    pub fn is_interactive(&self, id: NodeId) -> bool {
        self.is_button(id) || self.is_checkbox(id)
    }

    /// Является ли узел ползунком.
    pub fn is_slider(&self, id: NodeId) -> bool {
        matches!(self.nodes[id.0].kind, NodeKind::Slider { .. })
    }

    /// Задаёт значение ползунка в диапазоне 0..1.
    pub fn set_slider_value(&mut self, id: NodeId, value: f32) {
        if let NodeKind::Slider { value: v } = &mut self.nodes[id.0].kind {
            *v = value;
        }
    }

    /// Переключает состояние чекбокса.
    pub fn toggle_checkbox(&mut self, id: NodeId) {
        if let NodeKind::Checkbox { checked, .. } = &mut self.nodes[id.0].kind {
            *checked = !*checked;
        }
    }

    /// Возвращает состояние поля ввода.
    pub fn textbox_state(&self, id: NodeId) -> Option<&TextState> {
        if let NodeKind::TextBox { state } = &self.nodes[id.0].kind {
            Some(state)
        } else {
            None
        }
    }

    /// Возвращает изменяемое состояние поля ввода.
    pub fn textbox_state_mut(&mut self, id: NodeId) -> Option<&mut TextState> {
        if let NodeKind::TextBox { state } = &mut self.nodes[id.0].kind {
            Some(state)
        } else {
            None
        }
    }

    /// Возвращает действие узла.
    pub fn get_action(&self, id: NodeId) -> Action {
        self.nodes[id.0].action
    }

    /// Назначает действие узлу.
    pub fn set_action(&mut self, id: NodeId, action: Action) {
        self.nodes[id.0].action = action;
    }

    /// Возвращает верхний узел, содержащий точку `(x, y)`.
    pub fn hit_test(&self, x: f32, y: f32) -> Option<NodeId> {
        let mut hit = None;
        self.hit_walk(self.root, x, y, &mut hit);
        hit
    }

    fn hit_walk(&self, id: NodeId, x: f32, y: f32, hit: &mut Option<NodeId>) {
        if contains(self.nodes[id.0].rect, x, y) {
            *hit = Some(id);
        }
        let count = self.nodes[id.0].children.len();
        for i in 0..count {
            let child = self.nodes[id.0].children[i];
            self.hit_walk(child, x, y, hit);
        }
    }

    /// Вычисляет прямоугольники всех узлов линейной раскладкой.
    pub fn layout(&mut self, root_rect: Rect) {
        self.layout_node(self.root, root_rect);
    }

    fn layout_node(&mut self, id: NodeId, rect: Rect) {
        self.nodes[id.0].rect = rect;
        let props = self.nodes[id.0].props;
        let children = self.nodes[id.0].children.clone();
        if children.is_empty() {
            return;
        }

        let pad = props.padding;
        let inner = Rect::new(
            rect.x + pad,
            rect.y + pad,
            (rect.width - 2.0 * pad).max(0.0),
            (rect.height - 2.0 * pad).max(0.0),
        );
        let n = children.len();
        let total_gap = props.gap * n.saturating_sub(1) as f32;

        let main_available = match props.axis {
            Axis::Vertical => inner.height - total_gap,
            Axis::Horizontal => inner.width - total_gap,
        };
        let mut fixed_sum = 0.0;
        let mut flex_count = 0usize;
        for &c in &children {
            let cp = self.nodes[c.0].props;
            let main = match props.axis {
                Axis::Vertical => cp.height,
                Axis::Horizontal => cp.width,
            };
            match main {
                Some(v) => fixed_sum += v,
                None => flex_count += 1,
            }
        }
        let flex_size = if flex_count > 0 {
            (main_available - fixed_sum).max(0.0) / flex_count as f32
        } else {
            0.0
        };

        let mut cursor = match props.axis {
            Axis::Vertical => inner.y,
            Axis::Horizontal => inner.x,
        };
        for &c in &children {
            let cp = self.nodes[c.0].props;
            let child_rect = match props.axis {
                Axis::Vertical => {
                    let h = cp.height.unwrap_or(flex_size);
                    let w = cp.width.unwrap_or(inner.width);
                    let r = Rect::new(inner.x, cursor, w, h);
                    cursor += h + props.gap;
                    r
                }
                Axis::Horizontal => {
                    let w = cp.width.unwrap_or(flex_size);
                    let h = cp.height.unwrap_or(inner.height);
                    let r = Rect::new(cursor, inner.y, w, h);
                    cursor += w + props.gap;
                    r
                }
            };
            self.layout_node(c, child_rect);
        }
    }

    /// Обходит дерево в глубину, вызывая `visit(id, node)` для каждого узла.
    pub fn for_each<F: FnMut(NodeId, &Node)>(&self, mut visit: F) {
        self.walk(self.root, &mut visit);
    }

    fn walk<F: FnMut(NodeId, &Node)>(&self, id: NodeId, visit: &mut F) {
        visit(id, &self.nodes[id.0]);
        let count = self.nodes[id.0].children.len();
        for i in 0..count {
            let child = self.nodes[id.0].children[i];
            self.walk(child, visit);
        }
    }
}

impl Default for Tree {
    fn default() -> Self {
        Self::new()
    }
}

fn contains(rect: Rect, x: f32, y: f32) -> bool {
    x >= rect.x && x <= rect.x + rect.width && y >= rect.y && y <= rect.y + rect.height
}