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

pub enum NodeKind {
    Container,
    Frame {
        color: Color,
        radius: f32,
    },
    Label {
        text: Vec<u16>,
        color: Color,
    },
    Button {
        label: Vec<u16>,
        base: Color,
        hover: Color,
        pressed: Color,
        text: Color,
        radius: f32,
    },
}

pub struct Node {
    pub parent: Option<NodeId>,
    pub children: Vec<NodeId>,
    pub rect: Rect,
    pub kind: NodeKind,
    pub props: Props,
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

    /// Добавляет узел ребёнком к `parent` и возвращает его идентификатор.
    pub fn add_child(&mut self, parent: NodeId, kind: NodeKind, props: Props) -> NodeId {
        let id = NodeId(self.nodes.len());
        self.nodes.push(Node {
            parent: Some(parent),
            children: Vec::new(),
            rect: Rect::new(0.0, 0.0, 0.0, 0.0),
            kind,
            props,
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
        let children = &self.nodes[id.0].children;
        for i in 0..children.len() {
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
