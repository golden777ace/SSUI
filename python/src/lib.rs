use std::cell::RefCell;
use std::rc::Rc;

use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::wrap_pyfunction;

use ssui_core::platform::{dpi, Window as CoreWindow};
use ssui_core::tree::{Axis, NodeId, NodeKind, Props, Tree};

#[pyclass(name = "Node")]
#[derive(Clone, Copy)]
struct PyNode {
    id: NodeId,
}

type Bindings = Rc<RefCell<Vec<(NodeId, Py<PyAny>)>>>;

#[pyclass(unsendable, name = "Window")]
struct PyWindow {
    tree: Option<Tree>,
    title: String,
    width: i32,
    height: i32,
    bindings: Bindings,
}

#[pymethods]
impl PyWindow {
    #[new]
    #[pyo3(signature = (title="SSUI", width=1280, height=720))]
    fn new(title: &str, width: i32, height: i32) -> Self {
        Self {
            tree: Some(Tree::new()),
            title: title.to_string(),
            width,
            height,
            bindings: Rc::new(RefCell::new(Vec::new())),
        }
    }

    /// Возвращает корневой узел окна.
    fn root(&self) -> PyResult<PyNode> {
        let tree = self.tree.as_ref().ok_or_else(consumed)?;
        Ok(PyNode { id: tree.root() })
    }

    /// Добавляет панель со скруглением; возвращает её узел.
    #[pyo3(signature = (parent, radius=12.0, axis="vertical", padding=0.0, gap=0.0, width=None, height=None))]
    fn frame(
        &mut self,
        parent: PyNode,
        radius: f32,
        axis: &str,
        padding: f32,
        gap: f32,
        width: Option<f32>,
        height: Option<f32>,
    ) -> PyResult<PyNode> {
        let props = make_props(axis, padding, gap, width, height);
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(parent.id, NodeKind::Frame { radius }, props);
        Ok(PyNode { id })
    }

    /// Добавляет метку; `bind` — колбэк, возвращающий текст.
    #[pyo3(signature = (parent, text="", bind=None, padding=0.0, gap=0.0, width=None, height=None))]
    fn label(
        &mut self,
        py: Python,
        parent: PyNode,
        text: &str,
        bind: Option<PyObject>,
        padding: f32,
        gap: f32,
        width: Option<f32>,
        height: Option<f32>,
    ) -> PyResult<PyNode> {
        let props = make_props("vertical", padding, gap, width, height);
        let initial = match &bind {
            Some(f) => f.bind(py).call0()?.extract::<String>()?,
            None => text.to_string(),
        };
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(
            parent.id,
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

    /// Добавляет кнопку; `on_click` вызывается по нажатию.
    #[pyo3(signature = (parent, label, radius=10.0, padding=0.0, gap=0.0, width=None, height=None, on_click=None))]
    fn button(
        &mut self,
        parent: PyNode,
        label: &str,
        radius: f32,
        padding: f32,
        gap: f32,
        width: Option<f32>,
        height: Option<f32>,
        on_click: Option<PyObject>,
    ) -> PyResult<PyNode> {
        let props = make_props("vertical", padding, gap, width, height);
        let bindings = self.bindings.clone();
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(
            parent.id,
            NodeKind::Button {
                label: utf16(label),
                radius,
            },
            props,
        );
        tree.set_on_click(id, move |t| {
            Python::with_gil(|py| {
                if let Some(cb) = &on_click {
                    if let Err(e) = cb.bind(py).call0() {
                        e.print(py);
                    }
                }
                refresh_bindings(py, t, &bindings);
            });
        });
        Ok(PyNode { id })
    }

    /// Показывает окно и запускает цикл сообщений.
    fn run(&mut self) -> PyResult<()> {
        dpi::enable_dpi_awareness();
        let tree = self.tree.take().ok_or_else(consumed)?;
        let window = CoreWindow::new(&self.title, self.width, self.height, tree)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
        window.run();
        Ok(())
    }
}

fn consumed() -> PyErr {
    PyRuntimeError::new_err("окно уже запущено")
}

fn make_props(
    axis: &str,
    padding: f32,
    gap: f32,
    width: Option<f32>,
    height: Option<f32>,
) -> Props {
    Props {
        axis: parse_axis(axis),
        padding,
        gap,
        width,
        height,
    }
}

fn parse_axis(s: &str) -> Axis {
    match s {
        "horizontal" | "h" | "row" => Axis::Horizontal,
        _ => Axis::Vertical,
    }
}

fn utf16(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}

#[pyclass(name = "Signal")]
struct Signal {
    value: PyObject,
}

#[pymethods]
impl Signal {
    #[new]
    fn new(value: PyObject) -> Self {
        Self { value }
    }

    fn __call__(&self, py: Python) -> PyObject {
        self.value.clone_ref(py)
    }

    /// Возвращает текущее значение.
    fn get(&self, py: Python) -> PyObject {
        self.value.clone_ref(py)
    }

    /// Устанавливает новое значение.
    fn set(&mut self, value: PyObject) {
        self.value = value;
    }
}

/// Создаёт сигнал с начальным значением.
#[pyfunction]
fn signal(value: PyObject) -> Signal {
    Signal::new(value)
}

fn refresh_bindings(py: Python, t: &mut Tree, bindings: &Bindings) {
    for (id, f) in bindings.borrow().iter() {
        match f.bind(py).call0().and_then(|v| v.extract::<String>()) {
            Ok(s) => t.set_label_text(*id, utf16(&s)),
            Err(e) => e.print(py),
        }
    }
}

#[pymodule]
fn ssui(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyWindow>()?;
    m.add_class::<PyNode>()?;
    m.add_class::<Signal>()?;
    m.add_function(wrap_pyfunction!(signal, m)?)?;
    Ok(())
}