use std::cell::RefCell;
use std::rc::Rc;

use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::wrap_pyfunction;

use ssui_core::platform::{dpi, Window as CoreWindow};
use ssui_core::tree::{Axis, NodeId, NodeKind, Props, Tree};

#[pyclass(name = "N")]
#[derive(Clone, Copy)]
struct PyNode {
    id: NodeId,
}

type Bindings = Rc<RefCell<Vec<(NodeId, Py<PyAny>)>>>;

#[pyclass(unsendable, name = "W")]
struct PyWindow {
    tree: Option<Tree>,
    title: String,
    width: i32,
    height: i32,
    bindings: Bindings,
    value_bindings: Bindings,
}

#[pymethods]
impl PyWindow {
    #[new]
    #[pyo3(signature = (ttl="SSUI", w=1280, h=720, thm="drk"))]
    fn new(ttl: &str, w: i32, h: i32, thm: &str) -> Self {
        let mut tree = Tree::new();
        tree.set_theme(theme_index(thm));
        Self {
            tree: Some(tree),
            title: ttl.to_string(),
            width: w,
            height: h,
            bindings: Rc::new(RefCell::new(Vec::new())),
            value_bindings: Rc::new(RefCell::new(Vec::new())),
        }
    }

    /// Возвращает корневой узел окна.
    fn rt(&self) -> PyResult<PyNode> {
        let tree = self.tree.as_ref().ok_or_else(consumed)?;
        Ok(PyNode { id: tree.root() })
    }

    /// Добавляет панель со скруглением; возвращает её узел.
    #[pyo3(signature = (pr, rad=12.0, ax="v", pd=0.0, gp=0.0, w=None, h=None))]
    fn fr(
        &mut self,
        pr: PyNode,
        rad: f32,
        ax: &str,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<PyNode> {
        let props = make_props(ax, pd, gp, w, h);
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(pr.id, NodeKind::Frame { radius: rad }, props);
        Ok(PyNode { id })
    }

    /// Добавляет метку; `bind` — колбэк, возвращающий текст.
    #[pyo3(signature = (pr, txt="", bind=None, pd=0.0, gp=0.0, w=None, h=None))]
    fn lb(
        &mut self,
        py: Python,
        pr: PyNode,
        txt: &str,
        bind: Option<PyObject>,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<PyNode> {
        let props = make_props("v", pd, gp, w, h);
        let initial = match &bind {
            Some(f) => f.bind(py).call0()?.extract::<String>()?,
            None => txt.to_string(),
        };
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(
            pr.id,
            NodeKind::Label {
                text: utf16(&initial),
            },
            props,
        );
        if let Some(f) = bind {
            self.bindings.borrow_mut().push((id, f));
        }
        Ok(PyNode { id })
    }

    /// Добавляет кнопку; `clk` вызывается по нажатию.
    #[pyo3(signature = (pr, lb, rad=10.0, pd=0.0, gp=0.0, w=None, h=None, clk=None))]
    fn bt(
        &mut self,
        pr: PyNode,
        lb: &str,
        rad: f32,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
        clk: Option<PyObject>,
    ) -> PyResult<PyNode> {
        let props = make_props("v", pd, gp, w, h);
        let texts = self.bindings.clone();
        let values = self.value_bindings.clone();
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(
            pr.id,
            NodeKind::Button {
                label: utf16(lb),
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
        Ok(PyNode { id })
    }

    /// Добавляет ползунок 0..1; `ch(value)` при перетаскивании.
    #[pyo3(signature = (pr, vl=0.5, ch=None, pd=0.0, gp=0.0, w=None, h=None))]
    fn sl(
        &mut self,
        pr: PyNode,
        vl: f32,
        ch: Option<PyObject>,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<PyNode> {
        let props = make_props("v", pd, gp, w, h);
        let texts = self.bindings.clone();
        let values = self.value_bindings.clone();
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(pr.id, NodeKind::Slider { value: vl }, props);
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
    #[pyo3(signature = (pr, vl=0.0, bind=None, pd=0.0, gp=0.0, w=None, h=None))]
    fn pr(
        &mut self,
        py: Python,
        pr: PyNode,
        vl: f32,
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
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(pr.id, NodeKind::Progress { value: initial }, props);
        if let Some(f) = bind {
            self.value_bindings.borrow_mut().push((id, f));
        }
        Ok(PyNode { id })
    }

    /// Добавляет флажок; `clk` вызывается после переключения.
    #[pyo3(signature = (pr, lb, chk=false, clk=None, pd=0.0, gp=0.0, w=None, h=None))]
    fn ch(
        &mut self,
        pr: PyNode,
        lb: &str,
        chk: bool,
        clk: Option<PyObject>,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<PyNode> {
        let props = make_props("v", pd, gp, w, h);
        let texts = self.bindings.clone();
        let values = self.value_bindings.clone();
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(
            pr.id,
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

    /// Показывает окно и запускает цикл сообщений.
    fn go(&mut self) -> PyResult<()> {
        dpi::enable_dpi_awareness();
        let tree = self.tree.take().ok_or_else(consumed)?;
        let window = CoreWindow::new(&self.title, self.width, self.height, tree)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
        window.run();
        Ok(())
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
            }
            Err(e) => e.print(py),
        }
    }
}

#[pymodule]
fn ssui(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyWindow>()?;
    m.add_class::<PyNode>()?;
    m.add_class::<Signal>()?;
    m.add_function(wrap_pyfunction!(sgnl, m)?)?;
    Ok(())
}