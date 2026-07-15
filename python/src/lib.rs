use std::cell::RefCell;
use std::rc::Rc;

use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::wrap_pyfunction;

use ssui_core::platform::{dpi, Window as CoreWindow};
use ssui_core::tree::{
    Anim, AnimQueue, Axis, DialogData, DialogQueue, Ease, NodeId, NodeKind, NoteData,
    NoteQueue, Props, Shape, TextState, Tree, TreeItem,
};

type ShapeSpec = (String, Vec<f32>, String, String);

fn hexa(s: &str) -> u32 {
    ssui_core::render::parse_hex(s)
        .unwrap_or(ssui_core::render::Color::rgba(1.0, 1.0, 1.0, 1.0))
        .pack()
}

fn make_shapes(items: Vec<ShapeSpec>) -> Vec<Shape> {
    items
        .iter()
        .map(|(k, a, c, t)| {
            let kind = match k.as_str() {
                "rect" => 0u8,
                "circle" => 1,
                "line" => 2,
                _ => 3,
            };
            let mut args = [0.0f32; 6];
            for (i, v) in a.iter().take(6).enumerate() {
                args[i] = *v;
            }
            Shape {
                kind,
                args,
                color: hexa(c),
                text: utf16(t),
            }
        })
        .collect()
}

#[pyclass(name = "N")]
#[derive(Clone, Copy)]
struct PyNode {
    id: NodeId,
}

type Bindings = Rc<RefCell<Vec<(NodeId, Py<PyAny>)>>>;
type Stack = Rc<RefCell<Vec<NodeId>>>;

thread_local! {
    static WIN_SETTINGS: RefCell<Vec<(u8, Py<PyAny>)>> = RefCell::new(Vec::new());
    static CHART_BINDINGS: RefCell<Vec<(NodeId, Py<PyAny>)>> = RefCell::new(Vec::new());
    static PROP_BINDINGS: RefCell<Vec<(NodeId, Py<PyAny>)>> = RefCell::new(Vec::new());
    static CANVAS_BINDINGS: RefCell<Vec<(NodeId, Py<PyAny>)>> = RefCell::new(Vec::new());
    static BOX_BINDINGS: RefCell<Vec<(NodeId, Py<PyAny>)>> = RefCell::new(Vec::new());
    static LIST_BINDINGS: RefCell<Vec<(NodeId, Py<PyAny>)>> = RefCell::new(Vec::new());
    static TABLE_BINDINGS: RefCell<Vec<(NodeId, Py<PyAny>)>> = RefCell::new(Vec::new());
}

#[pyclass(unsendable, name = "Ctx")]
struct Ctx {
    stack: Stack,
    node: NodeId,
}

#[pymethods]
impl Ctx {
    fn __enter__(&self) -> PyNode {
        self.stack.borrow_mut().push(self.node);
        PyNode { id: self.node }
    }

    #[pyo3(signature = (_t=None, _v=None, _tb=None))]
    fn __exit__(
        &self,
        _t: Option<PyObject>,
        _v: Option<PyObject>,
        _tb: Option<PyObject>,
    ) -> bool {
        self.stack.borrow_mut().pop();
        false
    }
}

#[pyclass(unsendable, name = "Fx")]
struct Fx {
    queue: AnimQueue,
    texts: Bindings,
    values: Bindings,
}

#[pymethods]
impl Fx {
    /// Анимирует сигнал `sig` к значению `to` за `dur` секунд.
    #[pyo3(signature = (sig, to, *, frm=None, dur=0.3, ease="out"))]
    fn __call__(
        &self,
        py: Python,
        sig: PyObject,
        to: f32,
        frm: Option<f32>,
        dur: f32,
        ease: &str,
    ) -> PyResult<()> {
        let from = match frm {
            Some(v) => v,
            None => sig.bind(py).call0()?.extract::<f32>()?,
        };
        let e = parse_ease(ease);
        let texts = self.texts.clone();
        let values = self.values.clone();
        self.queue
            .borrow_mut()
            .push(Anim::new(from, to, dur, e, move |t, v| {
                Python::with_gil(|py| {
                    if let Err(err) = sig.bind(py).call_method1("st", (v,)) {
                        err.print(py);
                    }
                    refresh_all(py, t, &texts, &values);
                });
            }));
        Ok(())
    }
}

#[pyclass(unsendable, name = "Thm")]
struct Thm {
    queue: Rc<RefCell<Option<usize>>>,
}

#[pymethods]
impl Thm {
    /// Меняет тему окна: `wht`, `lit`, `drk`, `blk`.
    fn __call__(&self, name: &str) {
        *self.queue.borrow_mut() = Some(theme_index(name));
    }
}

#[pyclass(unsendable, name = "Note")]
struct Note {
    queue: NoteQueue,
    texts: Bindings,
    values: Bindings,
}

impl Note {
    fn push(
        &self,
        kind: u8,
        title: Vec<u16>,
        text: Vec<u16>,
        action: Vec<u16>,
        secs: f32,
        on: Option<PyObject>,
    ) {
        let texts = self.texts.clone();
        let values = self.values.clone();
        let cb: Option<Box<dyn FnMut(&mut Tree)>> = match on {
            Some(f) => Some(Box::new(move |t: &mut Tree| {
                Python::with_gil(|py| {
                    if let Err(e) = f.bind(py).call0() {
                        e.print(py);
                    }
                    refresh_all(py, t, &texts, &values);
                });
            })),
            None => None,
        };
        self.queue.borrow_mut().push(NoteData {
            title,
            text,
            action,
            secs,
            kind,
            cb,
        });
    }
}

#[pymethods]
impl Note {
    /// Уведомление в правом верхнем углу; `on()` по кнопке действия.
    #[pyo3(signature = (title, text, *, secs=4.0, action="", on=None))]
    fn __call__(
        &self,
        title: &str,
        text: &str,
        secs: f32,
        action: &str,
        on: Option<PyObject>,
    ) -> PyResult<()> {
        self.push(0, utf16(title), utf16(text), utf16(action), secs, on);
        Ok(())
    }

    /// Снэкбар внизу окна; `on()` по кнопке действия.
    #[pyo3(signature = (text, *, secs=4.0, action="", on=None))]
    fn snack(
        &self,
        text: &str,
        secs: f32,
        action: &str,
        on: Option<PyObject>,
    ) -> PyResult<()> {
        self.push(1, Vec::new(), utf16(text), utf16(action), secs, on);
        Ok(())
    }
}

#[pyclass(unsendable, name = "Dlg")]
struct Dlg {
    queue: DialogQueue,
    texts: Bindings,
    values: Bindings,
}

#[pymethods]
impl Dlg {
    /// Показывает модальный диалог; `on(index)` по нажатию кнопки.
    #[pyo3(signature = (title, message, buttons, *, on=None))]
    fn __call__(
        &self,
        title: &str,
        message: &str,
        buttons: Vec<String>,
        on: Option<PyObject>,
    ) -> PyResult<()> {
        let btns: Vec<Vec<u16>> = buttons.iter().map(|s| utf16(s)).collect();
        let texts = self.texts.clone();
        let values = self.values.clone();
        let data = DialogData {
            title: utf16(title),
            message: utf16(message),
            buttons: btns,
            cb: Box::new(move |t, i| {
                Python::with_gil(|py| {
                    if let Some(cb) = &on {
                        if let Err(e) = cb.bind(py).call1((i as i64,)) {
                            e.print(py);
                        }
                    }
                    refresh_all(py, t, &texts, &values);
                });
            }),
        };
        *self.queue.borrow_mut() = Some(data);
        Ok(())
    }

    /// Информационное окно с одной кнопкой.
    #[pyo3(signature = (title, message, *, ok="Ок", on=None))]
    fn msg(
        &self,
        title: &str,
        message: &str,
        ok: &str,
        on: Option<PyObject>,
    ) -> PyResult<()> {
        self.__call__(title, message, vec![ok.to_string()], on)
    }

    /// Предупреждение с одной кнопкой.
    #[pyo3(signature = (message, *, title="Внимание", ok="Ок", on=None))]
    fn alert(
        &self,
        message: &str,
        title: &str,
        ok: &str,
        on: Option<PyObject>,
    ) -> PyResult<()> {
        let head = format!("\u{26A0} {}", title);
        self.__call__(&head, message, vec![ok.to_string()], on)
    }
}

#[pyclass(unsendable, name = "W")]
struct PyWindow {
    tree: Option<Tree>,
    title: String,
    width: i32,
    height: i32,
    root: NodeId,
    stack: Stack,
    bindings: Bindings,
    value_bindings: Bindings,
    anim_queue: AnimQueue,
    dialog_queue: DialogQueue,
    note_queue: NoteQueue,
    theme_queue: Rc<RefCell<Option<usize>>>,
    glass: bool,
    tint: f32,
    blur: bool,
}

#[pymethods]
impl PyWindow {
    #[new]
    #[pyo3(signature = (ttl="SSUI", w=1280, h=720, thm="drk", glass=false, tint=0.0, blur=false))]
    fn new(ttl: &str, w: i32, h: i32, thm: &str, glass: bool, tint: f32, blur: bool) -> Self {
        let mut tree = Tree::new();
        tree.set_theme(theme_index(thm));
        let root = tree.root();
        let anim_queue = tree.anim_queue();
        let dialog_queue = tree.dialog_queue();
        let note_queue = tree.note_queue();
        let theme_queue = tree.theme_queue();
        Self {
            tree: Some(tree),
            title: ttl.to_string(),
            width: w,
            height: h,
            root,
            stack: Rc::new(RefCell::new(Vec::new())),
            bindings: Rc::new(RefCell::new(Vec::new())),
            value_bindings: Rc::new(RefCell::new(Vec::new())),
            anim_queue,
            dialog_queue,
            note_queue,
            theme_queue,
            glass,
            tint,
            blur,
        }
    }

    /// Возвращает корневой узел окна.
    fn rt(&self) -> PyNode {
        PyNode { id: self.root }
    }

    /// Смена темы из кода: `thm("drk")`.
    fn thm(&self) -> Thm {
        Thm {
            queue: self.theme_queue.clone(),
        }
    }

    /// Делает узел прозрачным для мыши; клики проходят насквозь.
    #[pyo3(signature = (node, on=true))]
    fn ghost(&mut self, node: PyNode, on: bool) -> PyResult<()> {
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        tree.set_ghost(node.id, on);
        Ok(())
    }

    /// Привязывает числовой колбэк к узлу (значение или страница стопки).
    fn bindv(&mut self, node: PyNode, f: PyObject) -> PyResult<()> {
        self.value_bindings.borrow_mut().push((node.id, f));
        Ok(())
    }

    /// Привязывает `(padding, gap)` контейнера к колбэку.
    fn bindb(&mut self, node: PyNode, f: PyObject) -> PyResult<()> {
        BOX_BINDINGS.with(|c| c.borrow_mut().push((node.id, f)));
        Ok(())
    }

    /// Привязывает пункты списка к колбэку, возвращающему список строк.
    fn bindl(&mut self, node: PyNode, f: PyObject) -> PyResult<()> {
        LIST_BINDINGS.with(|c| c.borrow_mut().push((node.id, f)));
        Ok(())
    }

    /// Привязывает строки таблицы к колбэку, возвращающему список рядов.
    fn bindt(&mut self, node: PyNode, f: PyObject) -> PyResult<()> {
        TABLE_BINDINGS.with(|c| c.borrow_mut().push((node.id, f)));
        Ok(())
    }

    /// Уведомления: `nt(title, text)` и `nt.snack(text)`.
    fn nt(&self) -> Note {
        Note {
            queue: self.note_queue.clone(),
            texts: self.bindings.clone(),
            values: self.value_bindings.clone(),
        }
    }

    /// Задаёт flex-вес узла вдоль главной оси.
    fn grow(&mut self, n: PyNode, g: f32) -> PyResult<()> {
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        tree.set_grow(n.id, g);
        Ok(())
    }

    /// Выравнивает детей узла: `justify` (st/cnt/end/btw), `cross` (str/st/cnt/end).
    #[pyo3(signature = (n, *, justify="st", cross="str"))]
    fn align(&mut self, n: PyNode, justify: &str, cross: &str) -> PyResult<()> {
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        tree.set_align(n.id, justify_code(justify), cross_code(cross));
        Ok(())
    }

    /// Пинит узел к краям родителя; `l/t/r/b` — отступы, `None` — не привязан.
    #[pyo3(signature = (n, *, l=None, t=None, r=None, b=None))]
    fn pin(
        &mut self,
        n: PyNode,
        l: Option<f32>,
        t: Option<f32>,
        r: Option<f32>,
        b: Option<f32>,
    ) -> PyResult<()> {
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        tree.set_pin(n.id, l, t, r, b);
        Ok(())
    }

    /// Возвращает контроллер анимаций.
    fn fx(&self) -> Fx {
        Fx {
            queue: self.anim_queue.clone(),
            texts: self.bindings.clone(),
            values: self.value_bindings.clone(),
        }
    }

    /// Привязывает прозрачность фона окна к сигналу 0..1.
    fn tint(&mut self, sig: PyObject) -> PyResult<()> {
        set_win_setting(0, sig);
        Ok(())
    }

    /// Привязывает силу размытия к сигналу 0..1 (0 — выключено).
    fn blur(&mut self, sig: PyObject) -> PyResult<()> {
        set_win_setting(1, sig);
        Ok(())
    }

    /// Привязывает режим фона к сигналу: 0 — нет, иначе — размытие.
    fn blur_mode(&mut self, sig: PyObject) -> PyResult<()> {
        set_win_setting(2, sig);
        Ok(())
    }

    /// Привязывает гашение размытия при перемещении к сигналу (0/1).
    fn drag_smooth(&mut self, sig: PyObject) -> PyResult<()> {
        set_win_setting(3, sig);
        Ok(())
    }

    /// Возвращает контроллер диалогов.
    fn dlg(&self) -> Dlg {
        Dlg {
            queue: self.dialog_queue.clone(),
            texts: self.bindings.clone(),
            values: self.value_bindings.clone(),
        }
    }

    /// Добавляет панель; возвращает её узел.
    #[pyo3(signature = (rad=12.0, *, pr=None, ax="v", pd=0.0, gp=0.0, w=None, h=None, elev=0.0))]
    fn fr(
        &mut self,
        rad: f32,
        pr: Option<PyNode>,
        ax: &str,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
        elev: f32,
    ) -> PyResult<PyNode> {
        let props = make_props(ax, pd, gp, w, h);
        let parent = self.parent_of(pr);
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(parent, NodeKind::Frame { radius: rad }, props);
        if elev > 0.0 {
            tree.set_elev(id, elev);
        }
        Ok(PyNode { id })
    }

    /// Панель-контейнер как контекст: `with win.bx(...) as p:`.
    #[pyo3(signature = (rad=12.0, *, pr=None, ax="v", pd=0.0, gp=0.0, w=None, h=None, elev=0.0))]
    fn bx(
        &mut self,
        rad: f32,
        pr: Option<PyNode>,
        ax: &str,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
        elev: f32,
    ) -> PyResult<Ctx> {
        let props = make_props(ax, pd, gp, w, h);
        let parent = self.parent_of(pr);
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(parent, NodeKind::Frame { radius: rad }, props);
        if elev > 0.0 {
            tree.set_elev(id, elev);
        }
        Ok(Ctx {
            stack: self.stack.clone(),
            node: id,
        })
    }

    /// Стопка страниц как контекст; `bind` — колбэк, возвращающий индекс.
    #[pyo3(signature = (*, pr=None, page=0, bind=None, w=None, h=None))]
    fn stk(
        &mut self,
        py: Python,
        pr: Option<PyNode>,
        page: usize,
        bind: Option<PyObject>,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<Ctx> {
        let props = make_props("v", 0.0, 0.0, w, h);
        let initial = match &bind {
            Some(f) => f.bind(py).call0()?.extract::<f32>()? as usize,
            None => page,
        };
        let parent = self.parent_of(pr);
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(parent, NodeKind::Stack { page: initial }, props);
        if let Some(f) = bind {
            self.value_bindings.borrow_mut().push((id, f));
        }
        Ok(Ctx {
            stack: self.stack.clone(),
            node: id,
        })
    }

    /// Разделитель двух областей как контекст; тянется мышью.
    #[pyo3(signature = (*, pr=None, ratio=0.5, vertical=true, w=None, h=None))]
    fn spl(
        &mut self,
        pr: Option<PyNode>,
        ratio: f32,
        vertical: bool,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<Ctx> {
        let props = make_props("v", 0.0, 0.0, w, h);
        let parent = self.parent_of(pr);
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(
            parent,
            NodeKind::Splitter {
                ratio: ratio.clamp(0.1, 0.9),
                vertical,
            },
            props,
        );
        Ok(Ctx {
            stack: self.stack.clone(),
            node: id,
        })
    }

    /// Секция аккордеона как контекст: `with win.acc("Имя"):`.
    #[pyo3(signature = (title="", *, pr=None, open=false, rad=10.0, pd=8.0, gp=8.0, w=None, h=None))]
    fn acc(
        &mut self,
        title: &str,
        pr: Option<PyNode>,
        open: bool,
        rad: f32,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<Ctx> {
        let props = make_props("v", pd, gp, w, h);
        let parent = self.parent_of(pr);
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(
            parent,
            NodeKind::Accordion {
                title: utf16(title),
                open,
                radius: rad,
            },
            props,
        );
        Ok(Ctx {
            stack: self.stack.clone(),
            node: id,
        })
    }

    /// Область прокрутки как контекст: `with win.scr():`.
    #[pyo3(signature = (*, pr=None, pd=8.0, gp=8.0, w=None, h=None))]
    fn scr(
        &mut self,
        pr: Option<PyNode>,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<Ctx> {
        let props = make_props("v", pd, gp, w, h);
        let parent = self.parent_of(pr);
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(
            parent,
            NodeKind::Scroll {
                offset: 0.0,
                content: 0.0,
            },
            props,
        );
        Ok(Ctx {
            stack: self.stack.clone(),
            node: id,
        })
    }

    /// Группа с заголовком как контекст: `with win.grp("Имя"):`.
    #[pyo3(signature = (title="", *, pr=None, rad=12.0, ax="v", pd=12.0, gp=8.0, w=None, h=None))]
    fn grp(
        &mut self,
        title: &str,
        pr: Option<PyNode>,
        rad: f32,
        ax: &str,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<Ctx> {
        let props = make_props(ax, pd, gp, w, h);
        let parent = self.parent_of(pr);
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(
            parent,
            NodeKind::Group {
                title: utf16(title),
                radius: rad,
            },
            props,
        );
        Ok(Ctx {
            stack: self.stack.clone(),
            node: id,
        })
    }

    /// Добавляет ссылку; `clk` вызывается по нажатию.
    #[pyo3(signature = (lb="", *, pr=None, clk=None, pd=0.0, gp=0.0, w=None, h=None))]
    fn lnk(
        &mut self,
        lb: &str,
        pr: Option<PyNode>,
        clk: Option<PyObject>,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<PyNode> {
        let h = h.or(Some(28.0));
        let props = make_props("v", pd, gp, w, h);
        let parent = self.parent_of(pr);
        let texts = self.bindings.clone();
        let values = self.value_bindings.clone();
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(parent, NodeKind::Link { label: utf16(lb) }, props);
        tree.set_on_click(id, move |t| {
            Python::with_gil(|py| {
                if let Some(cb) = &clk {
                    if let Err(e) = cb.bind(py).call0() {
                        e.print(py);
                    }
                }
                refresh_all(py, t, &texts, &values);
            });
        });
        Ok(PyNode { id })
    }

    /// Добавляет метку; `bind` — колбэк, возвращающий текст.
    #[pyo3(signature = (txt="", *, pr=None, bind=None, icon=None, pd=0.0, gp=0.0, w=None, h=None, wrap=false))]
    fn lb(
        &mut self,
        py: Python,
        txt: &str,
        pr: Option<PyNode>,
        bind: Option<PyObject>,
        icon: Option<String>,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
        wrap: bool,
    ) -> PyResult<PyNode> {
        let props = make_props("v", pd, gp, w, h);
        let initial = match &bind {
            Some(f) => f.bind(py).call0()?.extract::<String>()?,
            None => txt.to_string(),
        };
        let parent = self.parent_of(pr);
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(
            parent,
            NodeKind::Label {
                text: utf16(&initial),
            },
            props,
        );
        if wrap {
            tree.set_wrap(id, true);
        }
        if let Some(ic) = &icon {
            tree.set_icon(id, ic);
        }
        if let Some(f) = bind {
            self.bindings.borrow_mut().push((id, f));
        }
        Ok(PyNode { id })
    }

    /// Добавляет изображение; `fit`/`fit_bind` — режим вписывания.
    #[pyo3(signature = (src, *, pr=None, fit="contain", fit_bind=None, pd=0.0, gp=0.0, w=None, h=None))]
    fn img(
        &mut self,
        py: Python,
        src: &str,
        pr: Option<PyNode>,
        fit: &str,
        fit_bind: Option<PyObject>,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<PyNode> {
        let props = make_props("v", pd, gp, w, h);
        let code = match &fit_bind {
            Some(f) => f.bind(py).call0()?.extract::<f32>()? as u8,
            None => fit_code(fit),
        };
        let parent = self.parent_of(pr);
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(
            parent,
            NodeKind::Image {
                path: src.to_string(),
                fit: code,
            },
            props,
        );
        if let Some(f) = fit_bind {
            self.value_bindings.borrow_mut().push((id, f));
        }
        Ok(PyNode { id })
    }

    /// Добавляет кнопку; `clk` вызывается по нажатию.
    #[pyo3(signature = (lb="", *, pr=None, rad=10.0, icon=None, tip=None, toast=None, pd=0.0, gp=0.0, w=None, h=None, clk=None, elev=0.0))]
    fn bt(
        &mut self,
        lb: &str,
        pr: Option<PyNode>,
        rad: f32,
        icon: Option<String>,
        tip: Option<String>,
        toast: Option<String>,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
        clk: Option<PyObject>,
        elev: f32,
    ) -> PyResult<PyNode> {
        let props = make_props("v", pd, gp, w, h);
        let parent = self.parent_of(pr);
        let texts = self.bindings.clone();
        let values = self.value_bindings.clone();
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(
            parent,
            NodeKind::Button {
                label: utf16(lb),
                radius: rad,
            },
            props,
        );
        if elev > 0.0 {
            tree.set_elev(id, elev);
        }
        if let Some(ic) = &icon {
            tree.set_icon(id, ic);
        }
        if let Some(tp) = &tip {
            tree.set_tip(id, utf16(tp));
        }
        let toast_msg = toast.map(|s| utf16(&s));
        tree.set_on_click(id, move |t| {
            if let Some(m) = &toast_msg {
                t.push_toast(m.clone(), 2.5);
            }
            Python::with_gil(|py| {
                if let Some(cb) = &clk {
                    if let Err(e) = cb.bind(py).call0() {
                        e.print(py);
                    }
                }
                refresh_all(py, t, &texts, &values);
            });
        });
        Ok(PyNode { id })
    }

    /// Добавляет ползунок 0..1; `ch(value)` при перетаскивании.
    #[pyo3(signature = (vl=0.5, *, pr=None, ch=None, pd=0.0, gp=0.0, w=None, h=None))]
    fn sl(
        &mut self,
        vl: f32,
        pr: Option<PyNode>,
        ch: Option<PyObject>,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<PyNode> {
        let props = make_props("v", pd, gp, w, h);
        let parent = self.parent_of(pr);
        let texts = self.bindings.clone();
        let values = self.value_bindings.clone();
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(parent, NodeKind::Slider { value: vl }, props);
        tree.set_on_change(id, move |t, v| {
            Python::with_gil(|py| {
                if let Some(cb) = &ch {
                    if let Err(e) = cb.bind(py).call1((v,)) {
                        e.print(py);
                    }
                }
                refresh_all(py, t, &texts, &values);
            });
        });
        Ok(PyNode { id })
    }

    /// Док-панель с заголовком; клик по шапке сворачивает.
    #[pyo3(signature = (ttl="", *, pr=None, side="l", size=260.0, open=true, ax="v", pd=10.0, gp=8.0))]
    fn dock(
        &mut self,
        ttl: &str,
        pr: Option<PyNode>,
        side: &str,
        size: f32,
        open: bool,
        ax: &str,
        pd: f32,
        gp: f32,
    ) -> PyResult<Ctx> {
        let code = match side {
            "r" => 1u8,
            "t" => 2,
            "b" => 3,
            _ => 0,
        };
        let main = if open { size } else { 32.0 };
        let (w, h) = if code == 0 || code == 1 {
            (Some(main), None)
        } else {
            (None, Some(main))
        };
        let props = make_props(ax, pd, gp, w, h);
        let parent = self.parent_of(pr);
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(
            parent,
            NodeKind::Dock {
                title: utf16(ttl),
                side: code,
                size,
                open,
            },
            props,
        );
        Ok(Ctx {
            stack: self.stack.clone(),
            node: id,
        })
    }

    /// Зона приёма файлов; `on(paths)` при перетаскивании из проводника.
    #[pyo3(signature = (txt="Перетащите файлы сюда", *, pr=None, on=None, pd=0.0, gp=0.0, w=None, h=None))]
    fn drop(
        &mut self,
        txt: &str,
        pr: Option<PyNode>,
        on: Option<PyObject>,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<PyNode> {
        let h = h.or(Some(160.0));
        let props = make_props("v", pd, gp, w, h);
        let parent = self.parent_of(pr);
        let texts = self.bindings.clone();
        let values = self.value_bindings.clone();
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(parent, NodeKind::Drop { label: utf16(txt) }, props);
        tree.set_on_input(id, move |t, text| {
            let list: Vec<&str> = text.split('\n').filter(|s| !s.is_empty()).collect();
            Python::with_gil(|py| {
                if let Some(cb) = &on {
                    if let Err(e) = cb.bind(py).call1((list,)) {
                        e.print(py);
                    }
                }
                refresh_all(py, t, &texts, &values);
            });
        });
        Ok(PyNode { id })
    }

    /// Терминал; `on(cmd)` при Enter, вывод — возврат строки из `on`.
    #[pyo3(signature = (lines=Vec::new(), *, pr=None, prompt="$", on=None, pd=0.0, gp=0.0, w=None, h=None))]
    fn term(
        &mut self,
        lines: Vec<String>,
        pr: Option<PyNode>,
        prompt: &str,
        on: Option<PyObject>,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<PyNode> {
        let h = h.or(Some(320.0));
        let props = make_props("v", pd, gp, w, h);
        let parent = self.parent_of(pr);
        let texts = self.bindings.clone();
        let values = self.value_bindings.clone();
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(
            parent,
            NodeKind::Term {
                lines: lines.iter().map(|s| utf16(s)).collect(),
                input: TextState::new(),
                prompt: utf16(prompt),
                scroll: 0.0,
            },
            props,
        );
        tree.set_on_input(id, move |t, cmd| {
            Python::with_gil(|py| {
                if let Some(cb) = &on {
                    match cb.bind(py).call1((cmd,)) {
                        Ok(out) => {
                            if let Ok(s) = out.extract::<String>() {
                                for line in s.lines() {
                                    t.term_push(id, utf16(line));
                                }
                            }
                        }
                        Err(e) => e.print(py),
                    }
                }
                refresh_all(py, t, &texts, &values);
            });
        });
        Ok(PyNode { id })
    }

    /// Очищает вывод терминала.
    fn term_clear(&mut self, node: PyNode) -> PyResult<()> {
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        tree.term_clear(node.id);
        Ok(())
    }

    /// Область рисования; фигуры — `(вид, [args], цвет, текст)`.
    /// Виды: `rect` `[x,y,w,h,rad,stroke]`, `circle` `[cx,cy,r,stroke]`,
    /// `line` `[x1,y1,x2,y2,w]`, `text` `[x,y,w,h]`.
    #[pyo3(signature = (shapes=Vec::new(), *, pr=None, bind=None, pd=0.0, gp=0.0, w=None, h=None))]
    fn cv(
        &mut self,
        py: Python,
        shapes: Vec<ShapeSpec>,
        pr: Option<PyNode>,
        bind: Option<PyObject>,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<PyNode> {
        let h = h.or(Some(280.0));
        let props = make_props("v", pd, gp, w, h);
        let initial = match &bind {
            Some(f) => f.bind(py).call0()?.extract::<Vec<ShapeSpec>>()?,
            None => shapes,
        };
        let parent = self.parent_of(pr);
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(
            parent,
            NodeKind::Canvas {
                shapes: make_shapes(initial),
            },
            props,
        );
        if let Some(f) = bind {
            CANVAS_BINDINGS.with(|c| c.borrow_mut().push((id, f)));
        }
        Ok(PyNode { id })
    }

    /// Постраничная навигация; `ch(page)` при смене страницы.
    #[pyo3(signature = (total, *, pr=None, page=0, ch=None, pd=0.0, gp=0.0, w=None, h=None))]
    fn pgn(
        &mut self,
        total: usize,
        pr: Option<PyNode>,
        page: usize,
        ch: Option<PyObject>,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<PyNode> {
        let h = h.or(Some(44.0));
        let props = make_props("v", pd, gp, w, h);
        let parent = self.parent_of(pr);
        let texts = self.bindings.clone();
        let values = self.value_bindings.clone();
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let total = total.max(1);
        let id = tree.add_child(
            parent,
            NodeKind::Pager {
                page: page.min(total - 1),
                total,
            },
            props,
        );
        tree.set_on_change(id, move |t, v| {
            let i = v.max(0.0) as i64;
            Python::with_gil(|py| {
                if let Some(cb) = &ch {
                    if let Err(e) = cb.bind(py).call1((i,)) {
                        e.print(py);
                    }
                }
                refresh_all(py, t, &texts, &values);
            });
        });
        Ok(PyNode { id })
    }

    /// Оценка звёздами; `ch(value)` при клике.
    #[pyo3(signature = (vl=0, *, pr=None, max=5, ch=None, pd=0.0, gp=0.0, w=None, h=None))]
    fn rat(
        &mut self,
        vl: usize,
        pr: Option<PyNode>,
        max: usize,
        ch: Option<PyObject>,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<PyNode> {
        let h = h.or(Some(40.0));
        let props = make_props("v", pd, gp, w, h);
        let parent = self.parent_of(pr);
        let texts = self.bindings.clone();
        let values = self.value_bindings.clone();
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let max = max.max(1);
        let id = tree.add_child(
            parent,
            NodeKind::Rating {
                value: vl.min(max),
                max,
            },
            props,
        );
        tree.set_on_change(id, move |t, v| {
            let i = v.max(0.0) as i64;
            Python::with_gil(|py| {
                if let Some(cb) = &ch {
                    if let Err(e) = cb.bind(py).call1((i,)) {
                        e.print(py);
                    }
                }
                refresh_all(py, t, &texts, &values);
            });
        });
        Ok(PyNode { id })
    }

    /// Значок-счётчик; `dot=True` — точка без текста, `bind` — текст.
    #[pyo3(signature = (txt="", *, pr=None, dot=false, bind=None, pd=0.0, gp=0.0, w=None, h=None))]
    fn bdg(
        &mut self,
        py: Python,
        txt: &str,
        pr: Option<PyNode>,
        dot: bool,
        bind: Option<PyObject>,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<PyNode> {
        let h = h.or(Some(26.0));
        let w = w.or(Some(if dot { 16.0 } else { 46.0 }));
        let props = make_props("v", pd, gp, w, h);
        let initial = match &bind {
            Some(f) => f.bind(py).call0()?.extract::<String>()?,
            None => txt.to_string(),
        };
        let parent = self.parent_of(pr);
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(
            parent,
            NodeKind::Badge {
                text: utf16(&initial),
                dot,
            },
            props,
        );
        if let Some(f) = bind {
            self.bindings.borrow_mut().push((id, f));
        }
        Ok(PyNode { id })
    }

    /// Хлебные крошки; клик обрезает путь, `ch(i)`.
    #[pyo3(signature = (items, *, pr=None, ch=None, pd=0.0, gp=0.0, w=None, h=None))]
    fn crumb(
        &mut self,
        items: Vec<String>,
        pr: Option<PyNode>,
        ch: Option<PyObject>,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<PyNode> {
        let h = h.or(Some(34.0));
        let props = make_props("h", pd, gp, w, h);
        let parent = self.parent_of(pr);
        let texts = self.bindings.clone();
        let values = self.value_bindings.clone();
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(
            parent,
            NodeKind::Crumbs {
                items: items.iter().map(|s| utf16(s)).collect(),
            },
            props,
        );
        tree.set_on_change(id, move |t, v| {
            let i = v.max(0.0) as i64;
            Python::with_gil(|py| {
                if let Some(cb) = &ch {
                    if let Err(e) = cb.bind(py).call1((i,)) {
                        e.print(py);
                    }
                }
                refresh_all(py, t, &texts, &values);
            });
        });
        Ok(PyNode { id })
    }

    /// Возвращает элементы хлебных крошек узла.
    fn crumb_get(&mut self, node: PyNode) -> PyResult<Vec<String>> {
        let tree = self.tree.as_ref().ok_or_else(consumed)?;
        Ok(tree
            .crumb_items(node.id)
            .iter()
            .map(|s| String::from_utf16_lossy(s))
            .collect())
    }

    /// Задаёт элементы хлебных крошек узла.
    fn crumb_set(&mut self, node: PyNode, items: Vec<String>) -> PyResult<()> {
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        tree.set_crumb_items(node.id, items.iter().map(|s| utf16(s)).collect());
        Ok(())
    }

    /// Выбор времени; клики по половинам, `ch(часы, минуты)`.
    #[pyo3(signature = (hour=12, minute=0, *, pr=None, ch=None, pd=0.0, gp=0.0, w=None, h=None))]
    fn tm(
        &mut self,
        hour: u32,
        minute: u32,
        pr: Option<PyNode>,
        ch: Option<PyObject>,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<PyNode> {
        let h = h.or(Some(120.0));
        let props = make_props("v", pd, gp, w, h);
        let parent = self.parent_of(pr);
        let texts = self.bindings.clone();
        let values = self.value_bindings.clone();
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(
            parent,
            NodeKind::Time {
                hour: hour % 24,
                minute: minute % 60,
            },
            props,
        );
        tree.set_on_change(id, move |t, v| {
            let code = v.max(0.0) as i64;
            let (hh, mm) = (code / 100, code % 100);
            Python::with_gil(|py| {
                if let Some(cb) = &ch {
                    if let Err(e) = cb.bind(py).call1((hh, mm)) {
                        e.print(py);
                    }
                }
                refresh_all(py, t, &texts, &values);
            });
        });
        Ok(PyNode { id })
    }

    /// Таблица свойств «ключ — значение»; `bind` — колбэк списка пар.
    #[pyo3(signature = (rows, *, pr=None, bind=None, ch=None, pd=0.0, gp=0.0, w=None, h=None))]
    fn pg(
        &mut self,
        py: Python,
        rows: Vec<(String, String)>,
        pr: Option<PyNode>,
        bind: Option<PyObject>,
        ch: Option<PyObject>,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<PyNode> {
        let h = h.or(Some(260.0));
        let props = make_props("v", pd, gp, w, h);
        let initial = match &bind {
            Some(f) => f.bind(py).call0()?.extract::<Vec<(String, String)>>()?,
            None => rows,
        };
        let data = initial
            .iter()
            .map(|(k, v)| (utf16(k), utf16(v)))
            .collect::<Vec<_>>();
        let parent = self.parent_of(pr);
        let texts = self.bindings.clone();
        let values = self.value_bindings.clone();
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(
            parent,
            NodeKind::PropGrid {
                rows: data,
                selected: None,
                scroll: 0.0,
            },
            props,
        );
        tree.set_on_change(id, move |t, v| {
            let i = v.max(0.0) as i64;
            Python::with_gil(|py| {
                if let Some(cb) = &ch {
                    if let Err(e) = cb.bind(py).call1((i,)) {
                        e.print(py);
                    }
                }
                refresh_all(py, t, &texts, &values);
            });
        });
        if let Some(f) = bind {
            PROP_BINDINGS.with(|c| c.borrow_mut().push((id, f)));
        }
        Ok(PyNode { id })
    }

    /// Календарь; `ch(год, месяц, день)` при выборе даты.
    #[pyo3(signature = (year=2026, month=7, day=1, *, pr=None, ch=None, pd=0.0, gp=0.0, w=None, h=None))]
    fn cal(
        &mut self,
        year: i32,
        month: u32,
        day: u32,
        pr: Option<PyNode>,
        ch: Option<PyObject>,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<PyNode> {
        let h = h.or(Some(300.0));
        let props = make_props("v", pd, gp, w, h);
        let parent = self.parent_of(pr);
        let texts = self.bindings.clone();
        let values = self.value_bindings.clone();
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(
            parent,
            NodeKind::Calendar {
                year,
                month: month.clamp(1, 12),
                day: day.max(1),
            },
            props,
        );
        tree.set_on_change(id, move |t, v| {
            let code = v.max(0.0) as i64;
            let y = 2000 + code / 10000;
            let m = (code / 100) % 100;
            let d = code % 100;
            Python::with_gil(|py| {
                if let Some(cb) = &ch {
                    if let Err(e) = cb.bind(py).call1((y, m, d)) {
                        e.print(py);
                    }
                }
                refresh_all(py, t, &texts, &values);
            });
        });
        Ok(PyNode { id })
    }

    /// Палитра цвета (HSV); `ch("#RRGGBB")` при выборе.
    #[pyo3(signature = (hue=0.58, sat=0.75, val=0.96, *, pr=None, ch=None, pd=0.0, gp=0.0, w=None, h=None))]
    fn clr(
        &mut self,
        hue: f32,
        sat: f32,
        val: f32,
        pr: Option<PyNode>,
        ch: Option<PyObject>,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<PyNode> {
        let h = h.or(Some(220.0));
        let props = make_props("v", pd, gp, w, h);
        let parent = self.parent_of(pr);
        let texts = self.bindings.clone();
        let values = self.value_bindings.clone();
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(
            parent,
            NodeKind::Color {
                hue: hue.clamp(0.0, 1.0),
                sat: sat.clamp(0.0, 1.0),
                val: val.clamp(0.0, 1.0),
            },
            props,
        );
        tree.set_on_change(id, move |t, v| {
            let code = v.max(0.0) as u32;
            let hex = format!("#{:06X}", code);
            Python::with_gil(|py| {
                if let Some(cb) = &ch {
                    if let Err(e) = cb.bind(py).call1((hex.as_str(),)) {
                        e.print(py);
                    }
                }
                refresh_all(py, t, &texts, &values);
            });
        });
        Ok(PyNode { id })
    }

    /// Круговой регулятор 0..1; тянуть вверх/вниз, `ch(value)`.
    #[pyo3(signature = (vl=0.5, *, pr=None, lb="", ch=None, bind=None, pd=0.0, gp=0.0, w=None, h=None))]
    fn dl(
        &mut self,
        py: Python,
        vl: f32,
        pr: Option<PyNode>,
        lb: &str,
        ch: Option<PyObject>,
        bind: Option<PyObject>,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<PyNode> {
        let h = h.or(Some(140.0));
        let props = make_props("v", pd, gp, w, h);
        let initial = match &bind {
            Some(f) => f.bind(py).call0()?.extract::<f32>()?,
            None => vl,
        };
        let parent = self.parent_of(pr);
        let texts = self.bindings.clone();
        let values = self.value_bindings.clone();
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(
            parent,
            NodeKind::Dial {
                value: initial.clamp(0.0, 1.0),
                label: utf16(lb),
            },
            props,
        );
        tree.set_on_change(id, move |t, v| {
            Python::with_gil(|py| {
                if let Some(cb) = &ch {
                    if let Err(e) = cb.bind(py).call1((v,)) {
                        e.print(py);
                    }
                }
                refresh_all(py, t, &texts, &values);
            });
        });
        if let Some(f) = bind {
            self.value_bindings.borrow_mut().push((id, f));
        }
        Ok(PyNode { id })
    }

    /// Дерево; `items` — список `(глубина, текст, лист)`, `ch(i)` при выборе.
    #[pyo3(signature = (items, *, pr=None, ch=None, pd=0.0, gp=0.0, w=None, h=None))]
    fn tre(
        &mut self,
        items: Vec<(usize, String, bool)>,
        pr: Option<PyNode>,
        ch: Option<PyObject>,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<PyNode> {
        let h = h.or(Some(300.0));
        let props = make_props("v", pd, gp, w, h);
        let nodes: Vec<TreeItem> = items
            .iter()
            .map(|(d, s, leaf)| TreeItem {
                depth: *d,
                label: utf16(s),
                open: true,
                leaf: *leaf,
            })
            .collect();
        let parent = self.parent_of(pr);
        let texts = self.bindings.clone();
        let values = self.value_bindings.clone();
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(
            parent,
            NodeKind::TreeView {
                items: nodes,
                selected: None,
                scroll: 0.0,
            },
            props,
        );
        tree.set_on_change(id, move |t, v| {
            let i = v.max(0.0) as i64;
            Python::with_gil(|py| {
                if let Some(cb) = &ch {
                    if let Err(e) = cb.bind(py).call1((i,)) {
                        e.print(py);
                    }
                }
                refresh_all(py, t, &texts, &values);
            });
        });
        Ok(PyNode { id })
    }

    /// Кнопка с меню; `clk` — основное действие, `ch(i)` — пункт меню.
    #[pyo3(signature = (lb, opts, *, pr=None, clk=None, ch=None, rad=10.0, pd=0.0, gp=0.0, w=None, h=None))]
    fn sbt(
        &mut self,
        lb: &str,
        opts: Vec<String>,
        pr: Option<PyNode>,
        clk: Option<PyObject>,
        ch: Option<PyObject>,
        rad: f32,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<PyNode> {
        let h = h.or(Some(44.0));
        let props = make_props("v", pd, gp, w, h);
        let parent = self.parent_of(pr);
        let texts = self.bindings.clone();
        let values = self.value_bindings.clone();
        let texts2 = self.bindings.clone();
        let values2 = self.value_bindings.clone();
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(
            parent,
            NodeKind::Split {
                label: utf16(lb),
                options: opts.iter().map(|s| utf16(s)).collect(),
                radius: rad,
            },
            props,
        );
        tree.set_on_click(id, move |t| {
            Python::with_gil(|py| {
                if let Some(cb) = &clk {
                    if let Err(e) = cb.bind(py).call0() {
                        e.print(py);
                    }
                }
                refresh_all(py, t, &texts, &values);
            });
        });
        tree.set_on_change(id, move |t, v| {
            let i = v.max(0.0) as usize % 1000;
            Python::with_gil(|py| {
                if let Some(cb) = &ch {
                    if let Err(e) = cb.bind(py).call1((i as i64,)) {
                        e.print(py);
                    }
                }
                refresh_all(py, t, &texts2, &values2);
            });
        });
        Ok(PyNode { id })
    }

    /// Строка меню; `on_select(menu, item)` при выборе пункта.
    #[pyo3(signature = (menus, *, pr=None, on_select=None, pd=0.0, gp=0.0, w=None, h=None))]
    fn mb(
        &mut self,
        menus: Vec<(String, Vec<String>)>,
        pr: Option<PyNode>,
        on_select: Option<PyObject>,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<PyNode> {
        let h = h.or(Some(40.0));
        let props = make_props("h", pd, gp, w, h);
        let titles: Vec<Vec<u16>> = menus.iter().map(|(t, _)| utf16(t)).collect();
        let items: Vec<Vec<Vec<u16>>> = menus
            .iter()
            .map(|(_, its)| its.iter().map(|s| utf16(s)).collect())
            .collect();
        let parent = self.parent_of(pr);
        let texts = self.bindings.clone();
        let values = self.value_bindings.clone();
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(parent, NodeKind::MenuBar { titles, items }, props);
        tree.set_on_change(id, move |t, v| {
            let idx = v.max(0.0) as usize;
            let (m, i) = (idx / 1000, idx % 1000);
            Python::with_gil(|py| {
                if let Some(cb) = &on_select {
                    if let Err(e) = cb.bind(py).call1((m as i64, i as i64)) {
                        e.print(py);
                    }
                }
                refresh_all(py, t, &texts, &values);
            });
        });
        Ok(PyNode { id })
    }

    /// Диапазонный ползунок 0..1; `ch(lo, hi)` при перетаскивании.
    #[pyo3(signature = (lo=0.25, hi=0.75, *, pr=None, ch=None, pd=0.0, gp=0.0, w=None, h=None))]
    fn rsl(
        &mut self,
        lo: f32,
        hi: f32,
        pr: Option<PyNode>,
        ch: Option<PyObject>,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<PyNode> {
        let h = h.or(Some(36.0));
        let props = make_props("v", pd, gp, w, h);
        let parent = self.parent_of(pr);
        let texts = self.bindings.clone();
        let values = self.value_bindings.clone();
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(
            parent,
            NodeKind::Range {
                lo: lo.min(hi).clamp(0.0, 1.0),
                hi: lo.max(hi).clamp(0.0, 1.0),
            },
            props,
        );
        tree.set_on_change(id, move |t, _v| {
            let (a, b) = t.range_values(id);
            Python::with_gil(|py| {
                if let Some(cb) = &ch {
                    if let Err(e) = cb.bind(py).call1((a, b)) {
                        e.print(py);
                    }
                }
                refresh_all(py, t, &texts, &values);
            });
        });
        Ok(PyNode { id })
    }

    /// Строка состояния; `bind` — колбэк текста.
    #[pyo3(signature = (txt="", *, pr=None, bind=None, pd=0.0, gp=0.0, w=None, h=None))]
    fn stb(
        &mut self,
        py: Python,
        txt: &str,
        pr: Option<PyNode>,
        bind: Option<PyObject>,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<PyNode> {
        let h = h.or(Some(32.0));
        let props = make_props("h", pd, gp, w, h);
        let initial = match &bind {
            Some(f) => f.bind(py).call0()?.extract::<String>()?,
            None => txt.to_string(),
        };
        let parent = self.parent_of(pr);
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(
            parent,
            NodeKind::Status {
                text: utf16(&initial),
            },
            props,
        );
        if let Some(f) = bind {
            self.bindings.borrow_mut().push((id, f));
        }
        Ok(PyNode { id })
    }

    /// Сегментная шкала 0..1; `bind` — колбэк значения.
    #[pyo3(signature = (vl=0.0, *, pr=None, bind=None, seg=10, pd=0.0, gp=0.0, w=None, h=None))]
    fn mt(
        &mut self,
        py: Python,
        vl: f32,
        pr: Option<PyNode>,
        bind: Option<PyObject>,
        seg: usize,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<PyNode> {
        let h = h.or(Some(28.0));
        let props = make_props("v", pd, gp, w, h);
        let initial = match &bind {
            Some(f) => f.bind(py).call0()?.extract::<f32>()?,
            None => vl,
        };
        let parent = self.parent_of(pr);
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(
            parent,
            NodeKind::Meter {
                value: initial.clamp(0.0, 1.0),
                segments: seg.max(1),
            },
            props,
        );
        if let Some(f) = bind {
            self.value_bindings.borrow_mut().push((id, f));
        }
        Ok(PyNode { id })
    }

    /// Столбчатая диаграмма; `bind` — колбэк списка значений.
    #[pyo3(signature = (data, *, pr=None, bind=None, pd=0.0, gp=0.0, w=None, h=None))]
    fn cht(
        &mut self,
        py: Python,
        data: Vec<f32>,
        pr: Option<PyNode>,
        bind: Option<PyObject>,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<PyNode> {
        let h = h.or(Some(200.0));
        let props = make_props("v", pd, gp, w, h);
        let initial = match &bind {
            Some(f) => f.bind(py).call0()?.extract::<Vec<f32>>()?,
            None => data,
        };
        let parent = self.parent_of(pr);
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(parent, NodeKind::Chart { values: initial }, props);
        if let Some(f) = bind {
            CHART_BINDINGS.with(|c| c.borrow_mut().push((id, f)));
        }
        Ok(PyNode { id })
    }

    /// Добавляет вращающийся индикатор загрузки.
    #[pyo3(signature = (*, pr=None, pd=0.0, gp=0.0, w=None, h=None))]
    fn spn(
        &mut self,
        pr: Option<PyNode>,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<PyNode> {
        let h = h.or(Some(48.0));
        let props = make_props("v", pd, gp, w, h);
        let parent = self.parent_of(pr);
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(parent, NodeKind::Spinner { phase: 0.0 }, props);
        Ok(PyNode { id })
    }

    /// Круговой индикатор; `bind` — колбэк значения 0..1.
    #[pyo3(signature = (value=0.0, *, pr=None, lb="", bind=None, pd=0.0, gp=0.0, w=None, h=None))]
    fn gg(
        &mut self,
        py: Python,
        value: f32,
        pr: Option<PyNode>,
        lb: &str,
        bind: Option<PyObject>,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<PyNode> {
        let h = h.or(Some(140.0));
        let props = make_props("v", pd, gp, w, h);
        let initial = match &bind {
            Some(f) => f.bind(py).call0()?.extract::<f32>()?,
            None => value,
        };
        let parent = self.parent_of(pr);
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(
            parent,
            NodeKind::Gauge {
                value: initial.clamp(0.0, 1.0),
                label: utf16(lb),
            },
            props,
        );
        if let Some(f) = bind {
            self.value_bindings.borrow_mut().push((id, f));
        }
        Ok(PyNode { id })
    }

    /// Добавляет индикатор прогресса; `bind` — колбэк значения 0..1.
    #[pyo3(signature = (vl=0.0, *, pr=None, bind=None, pd=0.0, gp=0.0, w=None, h=None))]
    fn pr(
        &mut self,
        py: Python,
        vl: f32,
        pr: Option<PyNode>,
        bind: Option<PyObject>,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<PyNode> {
        let props = make_props("v", pd, gp, w, h);
        let initial = match &bind {
            Some(f) => f.bind(py).call0()?.extract::<f32>()?,
            None => vl,
        };
        let parent = self.parent_of(pr);
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(parent, NodeKind::Progress { value: initial }, props);
        if let Some(f) = bind {
            self.value_bindings.borrow_mut().push((id, f));
        }
        Ok(PyNode { id })
    }

    /// Добавляет флажок; `clk` вызывается после переключения.
    #[pyo3(signature = (lb="", *, pr=None, chk=false, clk=None, pd=0.0, gp=0.0, w=None, h=None))]
    fn ch(
        &mut self,
        lb: &str,
        pr: Option<PyNode>,
        chk: bool,
        clk: Option<PyObject>,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<PyNode> {
        let props = make_props("v", pd, gp, w, h);
        let parent = self.parent_of(pr);
        let texts = self.bindings.clone();
        let values = self.value_bindings.clone();
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(
            parent,
            NodeKind::Checkbox {
                label: utf16(lb),
                checked: chk,
            },
            props,
        );
        tree.set_on_click(id, move |t| {
            Python::with_gil(|py| {
                if let Some(cb) = &clk {
                    if let Err(e) = cb.bind(py).call0() {
                        e.print(py);
                    }
                }
                refresh_all(py, t, &texts, &values);
            });
        });
        Ok(PyNode { id })
    }

    /// Добавляет переключатель; `clk(on)` при смене состояния.
    #[pyo3(signature = (lb="", *, pr=None, on=false, clk=None, pd=0.0, gp=0.0, w=None, h=None))]
    fn sw(
        &mut self,
        lb: &str,
        pr: Option<PyNode>,
        on: bool,
        clk: Option<PyObject>,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<PyNode> {
        let props = make_props("v", pd, gp, w, h);
        let parent = self.parent_of(pr);
        let texts = self.bindings.clone();
        let values = self.value_bindings.clone();
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(parent, NodeKind::Switch { label: utf16(lb), on }, props);
        tree.set_on_change(id, move |t, v| {
            Python::with_gil(|py| {
                if let Some(cb) = &clk {
                    if let Err(e) = cb.bind(py).call1((v >= 0.5,)) {
                        e.print(py);
                    }
                }
                refresh_all(py, t, &texts, &values);
            });
        });
        Ok(PyNode { id })
    }

    /// Возвращает состояние переключателя.
    fn swv(&self, n: PyNode) -> PyResult<bool> {
        let tree = self.tree.as_ref().ok_or_else(consumed)?;
        Ok(tree.switch_on(n.id))
    }

    /// Добавляет радиокнопку группы `grp`; `clk()` при выборе.
    #[pyo3(signature = (lb="", *, pr=None, grp=0, on=false, clk=None, pd=0.0, gp=0.0, w=None, h=None))]
    fn rd(
        &mut self,
        lb: &str,
        pr: Option<PyNode>,
        grp: u32,
        on: bool,
        clk: Option<PyObject>,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<PyNode> {
        let props = make_props("v", pd, gp, w, h);
        let parent = self.parent_of(pr);
        let texts = self.bindings.clone();
        let values = self.value_bindings.clone();
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(
            parent,
            NodeKind::Radio { label: utf16(lb), on, group: grp },
            props,
        );
        tree.set_on_change(id, move |t, _v| {
            Python::with_gil(|py| {
                if let Some(cb) = &clk {
                    if let Err(e) = cb.bind(py).call0() {
                        e.print(py);
                    }
                }
                refresh_all(py, t, &texts, &values);
            });
        });
        Ok(PyNode { id })
    }

    /// Возвращает состояние радиокнопки.
    fn rdv(&self, n: PyNode) -> PyResult<bool> {
        let tree = self.tree.as_ref().ok_or_else(consumed)?;
        Ok(tree.radio_on(n.id))
    }

    /// Добавляет кнопку-переключатель; `clk(on)` при смене состояния.
    #[pyo3(signature = (lb="", *, pr=None, on=false, clk=None, pd=0.0, gp=0.0, w=None, h=None))]
    fn tgl(
        &mut self,
        lb: &str,
        pr: Option<PyNode>,
        on: bool,
        clk: Option<PyObject>,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<PyNode> {
        let props = make_props("v", pd, gp, w, h);
        let parent = self.parent_of(pr);
        let texts = self.bindings.clone();
        let values = self.value_bindings.clone();
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(parent, NodeKind::Toggle { label: utf16(lb), on }, props);
        tree.set_on_change(id, move |t, v| {
            Python::with_gil(|py| {
                if let Some(cb) = &clk {
                    if let Err(e) = cb.bind(py).call1((v >= 0.5,)) {
                        e.print(py);
                    }
                }
                refresh_all(py, t, &texts, &values);
            });
        });
        Ok(PyNode { id })
    }

    /// Возвращает состояние кнопки-переключателя.
    fn tglv(&self, n: PyNode) -> PyResult<bool> {
        let tree = self.tree.as_ref().ok_or_else(consumed)?;
        Ok(tree.toggle_on(n.id))
    }

    /// Добавляет разделитель; `vertical` — вертикальная линия.
    #[pyo3(signature = (*, pr=None, vertical=false, pd=0.0, gp=0.0, w=None, h=None))]
    fn sep(
        &mut self,
        pr: Option<PyNode>,
        vertical: bool,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<PyNode> {
        let (w2, h2) = if vertical {
            (w.or(Some(12.0)), h)
        } else {
            (w, h.or(Some(12.0)))
        };
        let props = make_props("v", pd, gp, w2, h2);
        let parent = self.parent_of(pr);
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(parent, NodeKind::Separator { vertical }, props);
        Ok(PyNode { id })
    }

    /// Добавляет поле ввода; `sig` — сигнал текста, `ph` — подсказка пустого поля.
    #[pyo3(signature = (txt="", *, pr=None, sig=None, ph="", pd=0.0, gp=0.0, w=None, h=None))]
    fn tx(
        &mut self,
        txt: &str,
        pr: Option<PyNode>,
        sig: Option<PyObject>,
        ph: &str,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<PyNode> {
        let props = make_props("v", pd, gp, w, h);
        let mut st = TextState::new();
        if !txt.is_empty() {
            st.text = utf16(txt);
            st.caret = st.text.len();
            st.anchor = st.caret;
        }
        let parent = self.parent_of(pr);
        let texts = self.bindings.clone();
        let values = self.value_bindings.clone();
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(parent, NodeKind::TextBox { state: st }, props);
        if !ph.is_empty() {
            tree.set_placeholder(id, utf16(ph));
        }
        if let Some(sig) = sig {
            tree.set_on_input(id, move |t, text| {
                Python::with_gil(|py| {
                    if let Err(e) = sig.bind(py).call_method1("st", (text,)) {
                        e.print(py);
                    }
                    refresh_all(py, t, &texts, &values);
                });
            });
        }
        Ok(PyNode { id })
    }

    /// Многострочное поле ввода; Enter — перенос, `ph` — подсказка пустого поля.
    #[pyo3(signature = (txt="", *, pr=None, sig=None, ph="", pd=0.0, gp=0.0, w=None, h=None))]
    fn ta(
        &mut self,
        txt: &str,
        pr: Option<PyNode>,
        sig: Option<PyObject>,
        ph: &str,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<PyNode> {
        let h = h.or(Some(120.0));
        let props = make_props("v", pd, gp, w, h);
        let mut st = TextState::new();
        if !txt.is_empty() {
            st.text = utf16(txt);
            st.caret = st.text.len();
            st.anchor = st.caret;
        }
        let parent = self.parent_of(pr);
        let texts = self.bindings.clone();
        let values = self.value_bindings.clone();
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(parent, NodeKind::TextBox { state: st }, props);
        tree.set_multiline(id);
        if !ph.is_empty() {
            tree.set_placeholder(id, utf16(ph));
        }
        if let Some(sig) = sig {
            tree.set_on_input(id, move |t, text| {
                Python::with_gil(|py| {
                    if let Err(e) = sig.bind(py).call_method1("st", (text,)) {
                        e.print(py);
                    }
                    refresh_all(py, t, &texts, &values);
                });
            });
        }
        Ok(PyNode { id })
    }

    /// Числовое поле с кнопками −/+; `ch(value)` при изменении.
    #[pyo3(signature = (value=0.0, *, pr=None, min=0.0, max=100.0, step=1.0, ch=None, pd=0.0, gp=6.0, w=None, h=None))]
    fn spin(
        &mut self,
        value: f32,
        pr: Option<PyNode>,
        min: f32,
        max: f32,
        step: f32,
        ch: Option<PyObject>,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<PyNode> {
        let bh = h.or(Some(44.0));
        let props = make_props("h", pd, gp, w, bh);
        let parent = self.parent_of(pr);
        let texts = self.bindings.clone();
        let values = self.value_bindings.clone();
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let box_id = tree.add_child(parent, NodeKind::Container, props);

        let minus = tree.add_child(
            box_id,
            NodeKind::Button { label: utf16("−"), radius: 8.0 },
            make_props("v", 0.0, 0.0, Some(44.0), bh),
        );
        let lbl = tree.add_child(
            box_id,
            NodeKind::Label {
                text: utf16(&fmt_num(value.clamp(min, max), step)),
            },
            make_props("v", 0.0, 0.0, None, bh),
        );
        tree.set_grow(lbl, 1.0);
        let plus = tree.add_child(
            box_id,
            NodeKind::Button { label: utf16("+"), radius: 8.0 },
            make_props("v", 0.0, 0.0, Some(44.0), bh),
        );

        let cur = std::rc::Rc::new(std::cell::Cell::new(value.clamp(min, max)));
        let cb_rc = std::rc::Rc::new(ch);

        let c1 = cur.clone();
        let ch1 = cb_rc.clone();
        let t1 = texts.clone();
        let v1 = values.clone();
        tree.set_on_click(minus, move |t| {
            let nv = (c1.get() - step).clamp(min, max);
            c1.set(nv);
            t.set_label_text(lbl, utf16(&fmt_num(nv, step)));
            Python::with_gil(|py| {
                if let Some(cb) = ch1.as_ref() {
                    if let Err(e) = cb.bind(py).call1((nv,)) {
                        e.print(py);
                    }
                }
                refresh_all(py, t, &t1, &v1);
            });
        });

        let c2 = cur.clone();
        let ch2 = cb_rc.clone();
        let t2 = texts.clone();
        let v2 = values.clone();
        tree.set_on_click(plus, move |t| {
            let nv = (c2.get() + step).clamp(min, max);
            c2.set(nv);
            t.set_label_text(lbl, utf16(&fmt_num(nv, step)));
            Python::with_gil(|py| {
                if let Some(cb) = ch2.as_ref() {
                    if let Err(e) = cb.bind(py).call1((nv,)) {
                        e.print(py);
                    }
                }
                refresh_all(py, t, &t2, &v2);
            });
        });

        Ok(PyNode { id: box_id })
    }

    /// Возвращает текущий текст поля ввода.
    fn tv(&self, n: PyNode) -> PyResult<String> {
        let tree = self.tree.as_ref().ok_or_else(consumed)?;
        Ok(tree.textbox_text(n.id).unwrap_or_default())
    }

    /// Добавляет выпадающий список; `ch(index)` при выборе.
    #[pyo3(signature = (options, *, pr=None, sel=0, ch=None, pd=0.0, gp=0.0, w=None, h=None))]
    fn dd(
        &mut self,
        options: Vec<String>,
        pr: Option<PyNode>,
        sel: usize,
        ch: Option<PyObject>,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<PyNode> {
        let opts: Vec<Vec<u16>> = options.iter().map(|s| utf16(s)).collect();
        let props = make_props("v", pd, gp, w, h);
        let parent = self.parent_of(pr);
        let texts = self.bindings.clone();
        let values = self.value_bindings.clone();
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(
            parent,
            NodeKind::Dropdown {
                options: opts,
                selected: sel,
                open: false,
            },
            props,
        );
        tree.set_on_change(id, move |t, v| {
            Python::with_gil(|py| {
                if let Some(cb) = &ch {
                    if let Err(e) = cb.bind(py).call1((v as i64,)) {
                        e.print(py);
                    }
                }
                refresh_all(py, t, &texts, &values);
            });
        });
        Ok(PyNode { id })
    }

    /// Добавляет вкладки как контекст; вложенные контейнеры — их содержимое.
    #[pyo3(signature = (labels, *, pr=None, sel=0, ch=None, pd=0.0, gp=0.0, w=None, h=None))]
    fn tab(
        &mut self,
        labels: Vec<String>,
        pr: Option<PyNode>,
        sel: usize,
        ch: Option<PyObject>,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<Ctx> {
        let labs: Vec<Vec<u16>> = labels.iter().map(|s| utf16(s)).collect();
        let props = make_props("v", pd, gp, w, h);
        let parent = self.parent_of(pr);
        let texts = self.bindings.clone();
        let values = self.value_bindings.clone();
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(
            parent,
            NodeKind::Tabs {
                labels: labs,
                selected: sel,
            },
            props,
        );
        tree.set_on_change(id, move |t, v| {
            Python::with_gil(|py| {
                if let Some(cb) = &ch {
                    if let Err(e) = cb.bind(py).call1((v as i64,)) {
                        e.print(py);
                    }
                }
                refresh_all(py, t, &texts, &values);
            });
        });
        Ok(Ctx {
            stack: self.stack.clone(),
            node: id,
        })
    }

    /// Задаёт всплывающую подсказку узла.
    fn tip(&mut self, n: PyNode, text: &str) -> PyResult<()> {
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        tree.set_tip(n.id, utf16(text));
        Ok(())
    }

    /// Присваивает узлу CSS-класс.
    fn cls(&mut self, n: PyNode, name: &str) -> PyResult<()> {
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        tree.set_class(n.id, Some(name.to_string()));
        Ok(())
    }

    /// Размещает узел абсолютно: `x`, `y`, `r`, `b`, `w`, `h`.
    #[pyo3(signature = (n, *, x=None, y=None, r=None, b=None, w=None, h=None))]
    fn pl(
        &mut self,
        n: PyNode,
        x: Option<f32>,
        y: Option<f32>,
        r: Option<f32>,
        b: Option<f32>,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<()> {
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        tree.set_place(n.id, x, y, r, b, w, h);
        Ok(())
    }

    /// Ставит узел в ячейку сетки: строка, столбец, растяжки.
    #[pyo3(signature = (n, r=0, c=0, *, rs=1, cs=1))]
    fn gr(&mut self, n: PyNode, r: u8, c: u8, rs: u8, cs: u8) -> PyResult<()> {
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        tree.set_grid(n.id, r, c, rs, cs);
        Ok(())
    }

    /// Прижимает узел к стороне: `t`/`b`/`l`/`r`, `fill`, `exp`.
    #[pyo3(signature = (n, side="t", *, fill=None, exp=false))]
    fn pk(&mut self, n: PyNode, side: &str, fill: Option<&str>, exp: bool) -> PyResult<()> {
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        tree.set_pack(n.id, side_code(side), fill_mask(fill), exp);
        Ok(())
    }

    /// Применяет CSS-подмножество из строки.
    fn css(&mut self, text: &str) -> PyResult<()> {
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        tree.apply_css(text);
        Ok(())
    }

    /// Применяет CSS из файла по пути.
    fn css_file(&mut self, path: &str) -> PyResult<()> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        tree.apply_css(&text);
        Ok(())
    }

    /// Следит за CSS-файлом и перезагружает его на лету.
    fn css_hot(&mut self, path: &str) -> PyResult<()> {
        let full = std::fs::canonicalize(path)
            .map_err(|e| PyRuntimeError::new_err(format!("{path}: {e}")))?;
        let full = full.to_string_lossy().to_string();
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        tree.css_watch(&full);
        Ok(())
    }

    /// Добавляет список; `ch(index)` при выборе пункта.
    #[pyo3(signature = (items, *, pr=None, sel=None, ch=None, pd=0.0, gp=0.0, w=None, h=None))]
    fn lst(
        &mut self,
        items: Vec<String>,
        pr: Option<PyNode>,
        sel: Option<usize>,
        ch: Option<PyObject>,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<PyNode> {
        let h = h.or(Some(240.0));
        let props = make_props("v", pd, gp, w, h);
        let its: Vec<Vec<u16>> = items.iter().map(|s| utf16(s)).collect();
        let parent = self.parent_of(pr);
        let texts = self.bindings.clone();
        let values = self.value_bindings.clone();
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(
            parent,
            NodeKind::List {
                items: its,
                selected: sel,
                scroll: 0.0,
            },
            props,
        );
        tree.set_on_change(id, move |t, v| {
            Python::with_gil(|py| {
                if let Some(cb) = &ch {
                    if let Err(e) = cb.bind(py).call1((v as i64,)) {
                        e.print(py);
                    }
                }
                refresh_all(py, t, &texts, &values);
            });
        });
        Ok(PyNode { id })
    }

    /// Возвращает выбранный пункт списка или -1.
    fn lstv(&self, n: PyNode) -> PyResult<i64> {
        let tree = self.tree.as_ref().ok_or_else(consumed)?;
        Ok(tree.list_selected(n.id).map_or(-1, |i| i as i64))
    }

    /// Добавляет таблицу; `hl`/`vl` — толщина разделителей строк и столбцов.
    #[pyo3(signature = (columns, rows, *, pr=None, ch=None, hl=0.0, vl=0.0, pd=0.0, gp=0.0, w=None, h=None))]
    fn tbl(
        &mut self,
        columns: Vec<String>,
        rows: Vec<Vec<String>>,
        pr: Option<PyNode>,
        ch: Option<PyObject>,
        hl: f32,
        vl: f32,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<PyNode> {
        let cols: Vec<Vec<u16>> = columns.iter().map(|s| utf16(s)).collect();
        let rws: Vec<Vec<Vec<u16>>> = rows
            .iter()
            .map(|row| row.iter().map(|c| utf16(c)).collect())
            .collect();
        let props = make_props("v", pd, gp, w, h);
        let parent = self.parent_of(pr);
        let texts = self.bindings.clone();
        let values = self.value_bindings.clone();
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(
            parent,
            NodeKind::Table {
                columns: cols,
                rows: rws,
                selected: None,
                scroll: 0.0,
                hline: hl,
                vline: vl,
            },
            props,
        );
        tree.set_on_change(id, move |t, v| {
            Python::with_gil(|py| {
                if let Some(cb) = &ch {
                    if let Err(e) = cb.bind(py).call1((v as i64,)) {
                        e.print(py);
                    }
                }
                refresh_all(py, t, &texts, &values);
            });
        });
        Ok(PyNode { id })
    }

    /// Задаёт контекстное меню окна; `on_select(index)` по ПКМ.
    #[pyo3(signature = (items, *, on_select=None))]
    fn menu(&mut self, items: Vec<String>, on_select: Option<PyObject>) -> PyResult<()> {
        let its: Vec<Vec<u16>> = items.iter().map(|s| utf16(s)).collect();
        let texts = self.bindings.clone();
        let values = self.value_bindings.clone();
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        tree.set_menu(its);
        let root = tree.root();
        tree.set_on_change(root, move |t, v| {
            Python::with_gil(|py| {
                if let Some(cb) = &on_select {
                    if let Err(e) = cb.bind(py).call1((v as i64,)) {
                        e.print(py);
                    }
                }
                refresh_all(py, t, &texts, &values);
            });
        });
        Ok(())
    }

    /// Показывает окно и запускает цикл сообщений.
    fn go(&mut self) -> PyResult<()> {
        dpi::enable_dpi_awareness();
        let tree = self.tree.take().ok_or_else(consumed)?;
        let window = CoreWindow::new(&self.title, self.width, self.height, tree, self.glass, self.tint, self.blur)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
        window.run();
        Ok(())
    }
}

impl PyWindow {
    fn parent_of(&self, pr: Option<PyNode>) -> NodeId {
        match pr {
            Some(p) => p.id,
            None => *self.stack.borrow().last().unwrap_or(&self.root),
        }
    }
}

#[pyclass(name = "S")]
struct Signal {
    value: PyObject,
}

#[pymethods]
impl Signal {
    #[new]
    fn new(vl: PyObject) -> Self {
        Self { value: vl }
    }

    fn __call__(&self, py: Python) -> PyObject {
        self.value.clone_ref(py)
    }

    /// Возвращает текущее значение.
    fn gt(&self, py: Python) -> PyObject {
        self.value.clone_ref(py)
    }

    /// Устанавливает новое значение.
    fn st(&mut self, vl: PyObject) {
        self.value = vl;
    }
}

/// Создаёт сигнал с начальным значением.
#[pyfunction]
fn sgnl(vl: PyObject) -> Signal {
    Signal::new(vl)
}

fn consumed() -> PyErr {
    PyRuntimeError::new_err("окно уже запущено")
}

fn set_win_setting(tag: u8, sig: PyObject) {
    WIN_SETTINGS.with(|s| {
        let mut v = s.borrow_mut();
        v.retain(|(t, _)| *t != tag);
        v.push((tag, sig));
    });
}

fn make_props(ax: &str, pd: f32, gp: f32, w: Option<f32>, h: Option<f32>) -> Props {
    Props {
        axis: parse_axis(ax),
        padding: pd,
        gap: gp,
        width: w,
        height: h,
        ..Default::default()
    }
}

fn justify_code(s: &str) -> u8 {
    match s {
        "cnt" => 1,
        "end" => 2,
        "btw" => 3,
        _ => 0,
    }
}

fn cross_code(s: &str) -> u8 {
    match s {
        "st" => 1,
        "cnt" => 2,
        "end" => 3,
        _ => 0,
    }
}

fn side_code(s: &str) -> u8 {
    match s {
        "b" => 1,
        "l" => 2,
        "r" => 3,
        _ => 0,
    }
}

fn fill_mask(s: Option<&str>) -> u8 {
    match s {
        Some("x") => 1,
        Some("y") => 2,
        Some("both") | Some("xy") => 3,
        _ => 0,
    }
}

fn parse_axis(s: &str) -> Axis {
    match s {
        "h" => Axis::Horizontal,
        _ => Axis::Vertical,
    }
}

fn fit_code(s: &str) -> u8 {
    match s {
        "cover" => 1,
        "fill" | "stretch" => 2,
        "center" | "none" => 3,
        _ => 0,
    }
}

fn fmt_num(v: f32, step: f32) -> String {
    if (step - step.round()).abs() < 1e-6 {
        format!("{}", v.round() as i64)
    } else {
        format!("{:.1}", v)
    }
}

fn parse_ease(s: &str) -> Ease {
    match s {
        "lin" => Ease::Linear,
        "in" => Ease::In,
        "io" => Ease::InOut,
        _ => Ease::Out,
    }
}

fn theme_index(name: &str) -> usize {
    match name {
        "wht" => 0,
        "lit" => 1,
        "blk" => 3,
        _ => 2,
    }
}

fn utf16(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}

fn refresh_all(py: Python, t: &mut Tree, texts: &Bindings, values: &Bindings) {
    for (id, f) in texts.borrow().iter() {
        match f.bind(py).call0().and_then(|v| v.extract::<String>()) {
            Ok(s) => t.set_label_text(*id, utf16(&s)),
            Err(e) => e.print(py),
        }
    }
    for (id, f) in values.borrow().iter() {
        match f.bind(py).call0().and_then(|v| v.extract::<f32>()) {
            Ok(v) => {
                t.set_slider_value(*id, v);
                t.set_progress_value(*id, v);
                t.set_gauge_value(*id, v);
                t.set_meter_value(*id, v);
                t.set_dial_value(*id, v);
                t.set_stack_page(*id, v.max(0.0) as usize);
                t.set_image_fit(*id, v as u8);
                if t.is_stack(*id) {
                    t.set_stack_page(*id, v.max(0.0) as usize);
                }
            }
            Err(e) => e.print(py),
        }
    }
    BOX_BINDINGS.with(|c| {
        for (id, f) in c.borrow().iter() {
            match f.bind(py).call0().and_then(|v| v.extract::<(f32, f32)>()) {
                Ok((pd, gp)) => t.set_box(*id, pd, gp),
                Err(e) => e.print(py),
            }
        }
    });
    LIST_BINDINGS.with(|c| {
        for (id, f) in c.borrow().iter() {
            match f.bind(py).call0().and_then(|v| v.extract::<Vec<String>>()) {
                Ok(rows) => {
                    let data = rows.iter().map(|s| utf16(s)).collect();
                    t.set_list_items(*id, data);
                }
                Err(e) => e.print(py),
            }
        }
    });
    TABLE_BINDINGS.with(|c| {
        for (id, f) in c.borrow().iter() {
            match f
                .bind(py)
                .call0()
                .and_then(|v| v.extract::<Vec<Vec<String>>>())
            {
                Ok(rows) => {
                    let data = rows
                        .iter()
                        .map(|r| r.iter().map(|c| utf16(c)).collect())
                        .collect();
                    t.set_table_rows(*id, data);
                }
                Err(e) => e.print(py),
            }
        }
    });
    CHART_BINDINGS.with(|c| {
        for (id, f) in c.borrow().iter() {
            match f.bind(py).call0().and_then(|v| v.extract::<Vec<f32>>()) {
                Ok(v) => t.set_chart_values(*id, v),
                Err(e) => e.print(py),
            }
        }
    });
    CANVAS_BINDINGS.with(|c| {
        for (id, f) in c.borrow().iter() {
            match f.bind(py).call0().and_then(|v| v.extract::<Vec<ShapeSpec>>()) {
                Ok(items) => t.set_canvas_shapes(*id, make_shapes(items)),
                Err(e) => e.print(py),
            }
        }
    });
    PROP_BINDINGS.with(|c| {
        for (id, f) in c.borrow().iter() {
            match f
                .bind(py)
                .call0()
                .and_then(|v| v.extract::<Vec<(String, String)>>())
            {
                Ok(rows) => {
                    let data = rows
                        .iter()
                        .map(|(k, v)| (utf16(k), utf16(v)))
                        .collect::<Vec<_>>();
                    t.set_prop_rows(*id, data);
                }
                Err(e) => e.print(py),
            }
        }
    });
    WIN_SETTINGS.with(|s| {
        for (tag, f) in s.borrow().iter() {
            let v = match f.bind(py).call0().and_then(|x| x.extract::<f32>()) {
                Ok(v) => v,
                Err(e) => {
                    e.print(py);
                    continue;
                }
            };
            match tag {
                0 => t.set_tint(v),
                1 => {
                    if v <= 0.001 {
                        t.set_blur_mode(0);
                    } else {
                        let a = (v.clamp(0.0, 1.0) * 255.0) as u32;
                        t.set_blur_mode(3);
                        t.set_blur_tint((a << 24) | 0x101418);
                    }
                }
                2 => {
                    let m = if v as i32 <= 0 { 0u32 } else { 3u32 };
                    t.set_blur_mode(m);
                }
                _ => t.set_drag_smooth(v >= 0.5),
            }
        }
    });
}

#[pymodule]
fn ssui(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyWindow>()?;
    m.add_class::<PyNode>()?;
    m.add_class::<Ctx>()?;
    m.add_class::<Fx>()?;
    m.add_class::<Dlg>()?;
    m.add_class::<Signal>()?;
    m.add_function(wrap_pyfunction!(sgnl, m)?)?;
    Ok(())
}