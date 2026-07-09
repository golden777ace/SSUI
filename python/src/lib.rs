use std::cell::RefCell;
use std::rc::Rc;

use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::wrap_pyfunction;

use ssui_core::platform::{dpi, Window as CoreWindow};
use ssui_core::tree::{
    Anim, AnimQueue, Axis, DialogData, DialogQueue, Ease, NodeId, NodeKind, Props, TextState, Tree,
};

#[pyclass(name = "N")]
#[derive(Clone, Copy)]
struct PyNode {
    id: NodeId,
}

type Bindings = Rc<RefCell<Vec<(NodeId, Py<PyAny>)>>>;
type Stack = Rc<RefCell<Vec<NodeId>>>;

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
    glass: bool,
    tint: f32,
}

#[pymethods]
impl PyWindow {
    #[new]
    #[pyo3(signature = (ttl="SSUI", w=1280, h=720, thm="drk", glass=false, tint=0.0))]
    fn new(ttl: &str, w: i32, h: i32, thm: &str, glass: bool, tint: f32) -> Self {
        let mut tree = Tree::new();
        tree.set_theme(theme_index(thm));
        let root = tree.root();
        let anim_queue = tree.anim_queue();
        let dialog_queue = tree.dialog_queue();
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
            glass,
            tint,
        }
    }

    /// Возвращает корневой узел окна.
    fn rt(&self) -> PyNode {
        PyNode { id: self.root }
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
        let root = self.tree.as_ref().ok_or_else(consumed)?.root();
        self.value_bindings.borrow_mut().push((root, sig));
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

    /// Добавляет метку; `bind` — колбэк, возвращающий текст.
    #[pyo3(signature = (txt="", *, pr=None, bind=None, pd=0.0, gp=0.0, w=None, h=None, wrap=false))]
    fn lb(
        &mut self,
        py: Python,
        txt: &str,
        pr: Option<PyNode>,
        bind: Option<PyObject>,
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
        if let Some(f) = bind {
            self.bindings.borrow_mut().push((id, f));
        }
        Ok(PyNode { id })
    }

    /// Добавляет кнопку; `clk` вызывается по нажатию.
    #[pyo3(signature = (lb="", *, pr=None, rad=10.0, pd=0.0, gp=0.0, w=None, h=None, clk=None, elev=0.0))]
    fn bt(
        &mut self,
        lb: &str,
        pr: Option<PyNode>,
        rad: f32,
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

    /// Добавляет индикатор; `bind` — колбэк, возвращающий 0..1.
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

    /// Добавляет поле ввода; `sig` — сигнал, куда пишется текст.
    #[pyo3(signature = (txt="", *, pr=None, sig=None, pd=0.0, gp=0.0, w=None, h=None))]
    fn tx(
        &mut self,
        txt: &str,
        pr: Option<PyNode>,
        sig: Option<PyObject>,
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

    /// Задаёт CSS-класс узла.
    fn cls(&mut self, n: PyNode, name: &str) -> PyResult<()> {
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        tree.set_class(n.id, Some(name.to_string()));
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

    /// Добавляет таблицу; `ch(index)` при выборе строки.
    #[pyo3(signature = (columns, rows, *, pr=None, ch=None, pd=0.0, gp=0.0, w=None, h=None))]
    fn tbl(
        &mut self,
        columns: Vec<String>,
        rows: Vec<Vec<String>>,
        pr: Option<PyNode>,
        ch: Option<PyObject>,
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
        let window = CoreWindow::new(&self.title, self.width, self.height, tree, self.glass, self.tint)
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

fn make_props(ax: &str, pd: f32, gp: f32, w: Option<f32>, h: Option<f32>) -> Props {
    Props {
        axis: parse_axis(ax),
        padding: pd,
        gap: gp,
        width: w,
        height: h,
    }
}

fn parse_axis(s: &str) -> Axis {
    match s {
        "h" => Axis::Horizontal,
        _ => Axis::Vertical,
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
    let root = t.root();
    for (id, f) in values.borrow().iter() {
        match f.bind(py).call0().and_then(|v| v.extract::<f32>()) {
            Ok(v) => {
                if *id == root {
                    t.set_tint(v);
                } else {
                    t.set_slider_value(*id, v);
                    t.set_progress_value(*id, v);
                }
            }
            Err(e) => e.print(py),
        }
    }
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