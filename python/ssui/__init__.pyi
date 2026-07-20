"""Подсказки типов для SSUI. Реализация — в скомпилированном модуле."""

from typing import Callable, Optional

class S:
    """Реактивный сигнал."""

    def __init__(self, vl: object) -> None: ...
    def __call__(self) -> object:
        """Возвращает текущее значение (чтение отслеживается)."""
    def gt(self) -> object:
        """Возвращает текущее значение."""
    def st(self, vl: object) -> None:
        """Устанавливает новое значение."""

def sgnl(vl: object) -> S:
    """Создаёт сигнал с начальным значением."""

class Node:
    """Ссылка на узел дерева элементов."""

class Ctx:
    """Контекст `with` для контейнеров и составных виджетов."""

    def __enter__(self) -> Node: ...
    def __exit__(self, _t: object = None, _v: object = None, _tb: object = None) -> bool: ...

class Fx:
    """Контроллер анимаций, возвращается `W.fx()`."""

    def __call__(
        self,
        sig: S,
        to: float,
        *,
        frm: Optional[float] = None,
        dur: float = 0.3,
        ease: str = "out",
    ) -> None:
        """Анимирует сигнал `sig` к значению `to` за `dur` секунд."""

class Thm:
    """Смена темы, возвращается `W.thm()`."""

    def __call__(self, name: str) -> None:
        """Меняет тему окна: `wht`, `lit`, `drk`, `blk`."""

class Fnt:
    """Живая смена шрифта, возвращается `W.fnt()`."""

    def __call__(self, family: str, size: float) -> None:
        """Меняет базовый шрифт приложения: семейство и размер."""

class Rct:
    """Геометрия узлов после раскладки, возвращается `W.rects()`."""

    def __call__(self, node: Node) -> tuple[float, float, float, float]:
        """Прямоугольник узла в координатах окна: `(x, y, w, h)`."""

class Note:
    """Уведомления, возвращается `W.nt()`."""

    def __call__(
        self,
        title: str,
        text: str,
        *,
        secs: float = 4.0,
        action: str = "",
        on: Optional[Callable[[], None]] = None,
    ) -> None:
        """Уведомление в правом верхнем углу."""
    def snack(
        self,
        text: str,
        *,
        secs: float = 4.0,
        action: str = "",
        on: Optional[Callable[[], None]] = None,
    ) -> None:
        """Снэкбар внизу окна."""

class Dlg:
    """Диалоги, возвращается `W.dlg()`."""

    def __call__(
        self,
        title: str,
        message: str,
        buttons: list[str],
        *,
        on: Optional[Callable[[int], None]] = None,
    ) -> None:
        """Модальный диалог с произвольными кнопками."""
    def msg(
        self,
        title: str,
        message: str,
        *,
        ok: str = "Ок",
        on: Optional[Callable[[int], None]] = None,
    ) -> None:
        """Окно с одной кнопкой."""
    def alert(
        self,
        message: str,
        *,
        title: str = "Внимание",
        ok: str = "Ок",
        on: Optional[Callable[[int], None]] = None,
    ) -> None:
        """Предупреждение с одной кнопкой."""

class W:
    """Главное окно SSUI."""

    def __init__(
        self,
        ttl: str = "SSUI",
        w: int = 1280,
        h: int = 720,
        thm: str = "drk",
        glass: bool = False,
        tint: float = 0.0,
        blur: bool = False,
        frameless: bool = False,
        topmost: bool = False,
        center: bool = False,
        resizable: bool = True,
        minbox: bool = True,
        maxbox: bool = True,
        closebox: bool = True,
        insp: bool = False,
    ) -> None:
        """Создаёт окно.

        `thm` — стартовая тема: wht/lit/drk/blk.
        `glass`/`tint`/`blur` — прозрачность и размытие фона.
        `frameless` — без рамки и заголовка ОС.
        `topmost` — поверх всех окон.
        `center` — по центру экрана или родителя.
        `resizable` — тянется ли за края.
        `minbox`/`maxbox`/`closebox` — кнопки заголовка.
        """

    # --- Окно и раскладка ---
    def rt(self) -> Node:
        """Возвращает корневой узел окна."""
    def thm(self) -> Thm:
        """Возвращает контроллер смены темы."""
    def fx(self) -> Fx:
        """Возвращает контроллер анимаций."""
    def dlg(self) -> Dlg:
        """Возвращает контроллер диалогов."""
    def nt(self) -> Note:
        """Возвращает контроллер уведомлений."""
    def tint(self, sig: S) -> None:
        """Привязывает прозрачность фона окна к сигналу 0..1."""
    def blur(self, sig: S) -> None:
        """Привязывает силу размытия фона к сигналу 0..1."""
    def blur_mode(self, sig: S) -> None:
        """Привязывает режим фона к сигналу: 0 — нет, иначе — размытие."""
    def drag_smooth(self, sig: S) -> None:
        """Привязывает гашение размытия при перемещении окна к сигналу."""
    def go(self) -> None:
        """Показывает окно и запускает цикл сообщений (блокирующий).

        Возвращается только при закрытии этого окна как главного.
        """
    def show(self) -> None:
        """Показывает окно, не блокируя цикл сообщений.

        Вызывается автоматически при выходе из `with`.
        """
    def close(self) -> None:
        """Закрывает окно программно; разрушение отложено до конца кадра."""
    def subwin(
        self,
        ttl: str = "",
        w: int = 520,
        h: int = 420,
        *,
        thm: str = "drk",
        modal: bool = False,
        center: bool = True,
        frameless: bool = False,
        topmost: bool = False,
        resizable: bool = True,
        minbox: bool = False,
        maxbox: bool = False,
        closebox: bool = True,
        glass: bool = False,
        tint: float = 0.0,
        blur: bool = False,
        insp: bool = False,
        on_close: Optional[Callable[[], None]] = None,
    ) -> "W":
        """Создаёт дочернее окно с собственным деревом виджетов.

        `modal` блокирует ввод в родителе до закрытия.
        `on_close` вызывается при закрытии любым способом.
        Содержимое строится внутри `with`, показ — по выходу из него.
        """
    def fnt(self) -> Fnt:
        """Возвращает контроллер смены шрифта."""
    def rects(self) -> Rct:
        """Возвращает доступ к геометрии узлов после раскладки."""
    def focus(
        self,
        node: Optional[Node] = None,
        *,
        txt: Optional[str] = None,
        sel: bool = True,
    ) -> None:
        """Ставит фокус на узел; `None` снимает фокус.

        `txt` подменяет текст поля ввода, `sel` выделяет его целиком.
        Безопасно вызывать из колбэков: запрос кладётся в очередь.
        """

    @staticmethod
    def measure_text(
            text: str, size: float = 15.0, family: str = "Segoe UI"
    ) -> tuple[float, float]:
        """Ширина и высота строки в пикселях: `(w, h)`."""

    @staticmethod
    def frames(path: str) -> int:
        """Число кадров в файле изображения; для GIF — длина анимации.

        Кадр выбирается суффиксом пути: `img(src_bind=lambda: f"{gif}|{i}")`.
        """

    def ghost(self, node: Node, on: bool = True) -> None:
        """Делает узел прозрачным для мыши."""
    def front(self, node: Node, on: bool = True) -> None:
        """Поднимает узел поверх соседей при нажатии внутри него."""
    def grow(self, n: Node, g: float) -> None:
        """Задаёт flex-вес узла вдоль главной оси."""
    def align(self, n: Node, *, justify: str = "st", cross: str = "str") -> None:
        """Выравнивает детей узла. justify: st/cnt/end/btw. cross: str/st/cnt/end."""
    def pin(
        self,
        n: Node,
        *,
        l: Optional[float] = None,
        t: Optional[float] = None,
        r: Optional[float] = None,
        b: Optional[float] = None,
    ) -> None:
        """Пинит узел к краям родителя; `None` — не привязан по этой стороне."""
    def pl(
        self,
        n: Node,
        *,
        x: Optional[float] = None,
        y: Optional[float] = None,
        r: Optional[float] = None,
        b: Optional[float] = None,
        w: Optional[float] = None,
        h: Optional[float] = None,
    ) -> None:
        """Размещает узел абсолютно внутри родителя."""
    def gr(self, n: Node, r: int = 0, c: int = 0, *, rs: int = 1, cs: int = 1) -> None:
        """Ставит узел в ячейку сетки: строка, столбец, растяжки по строкам/столбцам."""
    def pk(
        self, n: Node, side: str = "t", *, fill: Optional[str] = None, exp: bool = False
    ) -> None:
        """Прижимает узел к стороне контейнера: side — t/b/l/r; fill — x/y/both."""
    def dep(self, n: Node, z: int = 0) -> None:
        """Задаёт глубину узла по оси z; больше — ближе к зрителю."""

    def cls(self, n: Node, name: str) -> None:
        """Присваивает узлу CSS-класс."""
    def tip(self, n: Node, text: str) -> None:
        """Задаёт всплывающую подсказку узла."""
    def css(self, text: str) -> None:
        """Применяет CSS-подмножество из строки."""
    def css_file(self, path: str) -> None:
        """Применяет CSS из файла."""
    def css_hot(self, path: str) -> None:
        """Следит за CSS-файлом и перезагружает его на лету."""

    def bindv(self, node: Node, f: Callable[[], float]) -> None:
        """Привязывает числовой колбэк к узлу (значение / страница стопки)."""
    def bindb(self, node: Node, f: Callable[[], tuple[float, float]]) -> None:
        """Привязывает (padding, gap) контейнера к колбэку."""
    def bindl(self, node: Node, f: Callable[[], list[str]]) -> None:
        """Привязывает пункты списка к колбэку."""
    def bindt(self, node: Node, f: Callable[[], list[list[str]]]) -> None:
        """Привязывает строки таблицы к колбэку."""
    def bindz(self, node: Node, f: Callable[[], float]) -> None:
        """Привязывает глубину z узла к колбэку."""
    def bindp(
        self, node: Node, f: Callable[[], tuple[float, float, float, float]]
    ) -> None:
        """Привязывает абсолютную позицию узла к колбэку `(x, y, w, h)`."""
    def every(self, ms: float, f: Callable[[], None]) -> None:
        """Вызывает колбэк каждые `ms` миллисекунд, не блокируя окно.

        Регистрируется до показа окна. Пока есть хотя бы один таймер,
        цикл отрисовки не засыпает.
        """
    def frame(
        self,
        *,
        icon: Optional[str] = None,
        cap: Optional[str] = None,
        cap_txt: Optional[str] = None,
        brd: Optional[str] = None,
        dark: Optional[bool] = None,
    ) -> None:
        """Оформление рамки окна; вызывается до показа.

        `icon` — путь к .ico для заголовка и панели задач.
        `cap`/`cap_txt`/`brd` — цвета заголовка, его текста и рамки
        в формате `#RRGGBB`. Требуют Windows 11.
        `dark` — тёмный режим неклиентской области.
        """
    def keys(self, node: Node, f: Callable[[int], None]) -> None:
        """Реакция поля ввода на клавиши: `1` — Enter, `0` — Esc или клик мимо."""

    def menu(self, items: list[str], *, on_select: Optional[Callable[[int], None]] = None) -> None:
        """Задаёт контекстное меню окна (ПКМ)."""

    # --- Контейнеры (контексты, использовать через `with`) ---
    def bx(
        self,
        rad: float = 12.0,
        *,
        pr: Optional[Node] = None,
        ax: str = "v",
        pd: float = 0.0,
        gp: float = 0.0,
        w: Optional[float] = None,
        h: Optional[float] = None,
        elev: float = 0.0,
    ) -> Ctx:
        """Панель-контейнер как контекст."""
    def grp(
        self,
        title: str = "",
        *,
        pr: Optional[Node] = None,
        rad: float = 12.0,
        ax: str = "v",
        pd: float = 12.0,
        gp: float = 8.0,
        w: Optional[float] = None,
        h: Optional[float] = None,
    ) -> Ctx:
        """Группа с заголовком как контекст."""
    def scr(
        self,
        *,
        pr: Optional[Node] = None,
        pd: float = 8.0,
        gp: float = 8.0,
        w: Optional[float] = None,
        h: Optional[float] = None,
    ) -> Ctx:
        """Область прокрутки как контекст."""
    def spl(
        self,
        *,
        pr: Optional[Node] = None,
        ratio: float = 0.5,
        vertical: bool = True,
        w: Optional[float] = None,
        h: Optional[float] = None,
    ) -> Ctx:
        """Разделитель двух областей как контекст; тянется мышью."""
    def tab(
        self,
        labels: list[str],
        *,
        pr: Optional[Node] = None,
        sel: int = 0,
        ch: Optional[Callable[[int], None]] = None,
        pd: float = 0.0,
        gp: float = 0.0,
        w: Optional[float] = None,
        h: Optional[float] = None,
    ) -> Ctx:
        """Вкладки как контекст; вложенные контейнеры — их содержимое."""
    def acc(
        self,
        title: str = "",
        *,
        pr: Optional[Node] = None,
        open: bool = False,
        rad: float = 10.0,
        pd: float = 8.0,
        gp: float = 8.0,
        w: Optional[float] = None,
        h: Optional[float] = None,
    ) -> Ctx:
        """Секция аккордеона как контекст."""
    def stk(
        self,
        *,
        pr: Optional[Node] = None,
        page: int = 0,
        bind: Optional[Callable[[], float]] = None,
        w: Optional[float] = None,
        h: Optional[float] = None,
    ) -> Ctx:
        """Стопка страниц как контекст; видна одна по индексу."""
    def dock(
        self,
        ttl: str = "",
        *,
        pr: Optional[Node] = None,
        side: str = "l",
        size: float = 260.0,
        open: bool = True,
        ax: str = "v",
        pd: float = 10.0,
        gp: float = 8.0,
    ) -> Ctx:
        """Док-панель с заголовком как контекст; клик по шапке сворачивает."""

    # --- Панели и области (не контексты) ---
    def fr(
        self,
        rad: float = 12.0,
        *,
        pr: Optional[Node] = None,
        ax: str = "v",
        pd: float = 0.0,
        gp: float = 0.0,
        w: Optional[float] = None,
        h: Optional[float] = None,
        elev: float = 0.0,
    ) -> Node:
        """Панель; возвращает узел (не контекст)."""
    def cv(
        self,
        shapes: list[tuple] = ...,
        *,
        pr: Optional[Node] = None,
        bind: Optional[Callable[[], list[tuple]]] = None,
        ch: Optional[Callable[[int], None]] = None,
        pd: float = 0.0,
        gp: float = 0.0,
        w: Optional[float] = None,
        h: Optional[float] = None,
    ) -> Node:
        """Область рисования. Фигуры: `(вид, [args], цвет, текст)`.

        Виды и аргументы:
        `rect` — [x, y, w, h, radius, stroke]
        `circle` — [cx, cy, r, stroke]
        `line` — [x1, y1, x2, y2, width]
        `text` — [x, y, w, h]
        `arrow` — [x1, y1, x2, y2, width, head]
        `arc` — [cx, cy, r, start_deg, sweep_deg, width]
        `sector` — [cx, cy, r, start_deg, sweep_deg, 0]

        `stroke` > 0 рисует контур, 0 — заливку.
        `ch` получает индекс фигуры под курсором или -1 при промахе.
        """
    def drop(
        self,
        txt: str = "Перетащите файлы сюда",
        *,
        pr: Optional[Node] = None,
        on: Optional[Callable[[list[str]], None]] = None,
        pd: float = 0.0,
        gp: float = 0.0,
        w: Optional[float] = None,
        h: Optional[float] = None,
    ) -> Node:
        """Зона приёма файлов из проводника."""

    # --- Отображение ---
    def lb(
        self,
        txt: str = "",
        *,
        pr: Optional[Node] = None,
        bind: Optional[Callable[[], str]] = None,
        icon: Optional[str] = None,
        pd: float = 0.0,
        gp: float = 0.0,
        w: Optional[float] = None,
        h: Optional[float] = None,
        wrap: bool = False,
    ) -> Node:
        """Текстовая метка."""

    def img(
            self,
            src: str = "",
            *,
            pr: Optional[Node] = None,
            src_bind: Optional[Callable[[], str]] = None,
            fit: str = "contain",
            fit_bind: Optional[Callable[[], float]] = None,
            pd: float = 0.0,
            gp: float = 0.0,
            w: Optional[float] = None,
            h: Optional[float] = None,
    ) -> Node:
        """Изображение из файла.

        `src` — путь к файлу; `src_bind` — путь из колбэка,
        пересчитывается каждый кадр.
        Кадр анимированного GIF выбирается суффиксом: `"logo.gif|3"`.
        `fit`/`fit_bind` — режим вписывания:
        `contain`, `cover`, `fill`, `none`.
        """
    def sep(
        self,
        *,
        pr: Optional[Node] = None,
        vertical: bool = False,
        pd: float = 0.0,
        gp: float = 0.0,
        w: Optional[float] = None,
        h: Optional[float] = None,
    ) -> Node:
        """Разделитель."""
    def pr(
        self,
        vl: float = 0.0,
        *,
        pr: Optional[Node] = None,
        bind: Optional[Callable[[], float]] = None,
        pd: float = 0.0,
        gp: float = 0.0,
        w: Optional[float] = None,
        h: Optional[float] = None,
    ) -> Node:
        """Индикатор прогресса 0..1."""
    def spn(
        self,
        *,
        pr: Optional[Node] = None,
        pd: float = 0.0,
        gp: float = 0.0,
        w: Optional[float] = None,
        h: Optional[float] = None,
    ) -> Node:
        """Вращающийся индикатор загрузки."""
    def gg(
        self,
        value: float = 0.0,
        *,
        pr: Optional[Node] = None,
        lb: str = "",
        bind: Optional[Callable[[], float]] = None,
        pd: float = 0.0,
        gp: float = 0.0,
        w: Optional[float] = None,
        h: Optional[float] = None,
    ) -> Node:
        """Круговой индикатор 0..1."""
    def cht(
        self,
        data: list[float],
        *,
        pr: Optional[Node] = None,
        bind: Optional[Callable[[], list[float]]] = None,
        pd: float = 0.0,
        gp: float = 0.0,
        w: Optional[float] = None,
        h: Optional[float] = None,
    ) -> Node:
        """Столбчатая диаграмма."""
    def mt(
        self,
        vl: float = 0.0,
        *,
        pr: Optional[Node] = None,
        bind: Optional[Callable[[], float]] = None,
        seg: int = 10,
        pd: float = 0.0,
        gp: float = 0.0,
        w: Optional[float] = None,
        h: Optional[float] = None,
    ) -> Node:
        """Сегментная шкала 0..1."""
    def stb(
        self,
        txt: str = "",
        *,
        pr: Optional[Node] = None,
        bind: Optional[Callable[[], str]] = None,
        pd: float = 0.0,
        gp: float = 0.0,
        w: Optional[float] = None,
        h: Optional[float] = None,
    ) -> Node:
        """Строка состояния."""
    def bdg(
        self,
        txt: str = "",
        *,
        pr: Optional[Node] = None,
        dot: bool = False,
        bind: Optional[Callable[[], str]] = None,
        pd: float = 0.0,
        gp: float = 0.0,
        w: Optional[float] = None,
        h: Optional[float] = None,
    ) -> Node:
        """Значок-счётчик; `dot=True` — точка без текста."""

    # --- Кнопки и действия ---
    def bt(
        self,
        lb: str = "",
        *,
        pr: Optional[Node] = None,
        rad: float = 10.0,
        icon: Optional[str] = None,
        tip: Optional[str] = None,
        toast: Optional[str] = None,
        pd: float = 0.0,
        gp: float = 0.0,
        w: Optional[float] = None,
        h: Optional[float] = None,
        clk: Optional[Callable[[], None]] = None,
        elev: float = 0.0,
    ) -> Node:
        """Кнопка."""
    def tgl(
        self,
        lb: str = "",
        *,
        pr: Optional[Node] = None,
        on: bool = False,
        clk: Optional[Callable[[bool], None]] = None,
        pd: float = 0.0,
        gp: float = 0.0,
        w: Optional[float] = None,
        h: Optional[float] = None,
    ) -> Node:
        """Кнопка-переключатель (двухпозиционная)."""
    def dd(
        self,
        options: list[str],
        *,
        pr: Optional[Node] = None,
        sel: int = 0,
        ch: Optional[Callable[[int], None]] = None,
        pd: float = 0.0,
        gp: float = 0.0,
        w: Optional[float] = None,
        h: Optional[float] = None,
    ) -> Node:
        """Выпадающий список (ComboBox)."""
    def lnk(
        self,
        lb: str = "",
        *,
        pr: Optional[Node] = None,
        clk: Optional[Callable[[], None]] = None,
        pd: float = 0.0,
        gp: float = 0.0,
        w: Optional[float] = None,
        h: Optional[float] = None,
    ) -> Node:
        """Кликабельная ссылка."""
    def sbt(
        self,
        lb: str,
        opts: list[str],
        *,
        pr: Optional[Node] = None,
        clk: Optional[Callable[[], None]] = None,
        ch: Optional[Callable[[int], None]] = None,
        rad: float = 10.0,
        pd: float = 0.0,
        gp: float = 0.0,
        w: Optional[float] = None,
        h: Optional[float] = None,
    ) -> Node:
        """Кнопка с меню: основное действие + список пунктов."""

    # --- Ввод ---
    def tx(
        self,
        txt: str = "",
        *,
        pr: Optional[Node] = None,
        sig: Optional[S] = None,
        ph: str = "",
        pd: float = 0.0,
        gp: float = 0.0,
        w: Optional[float] = None,
        h: Optional[float] = None,
    ) -> Node:
        """Однострочное поле ввода. `sig` — сигнал текста, `ph` — плейсхолдер."""
    def ta(
        self,
        txt: str = "",
        *,
        pr: Optional[Node] = None,
        sig: Optional[S] = None,
        ph: str = "",
        pd: float = 0.0,
        gp: float = 0.0,
        w: Optional[float] = None,
        h: Optional[float] = None,
    ) -> Node:
        """Многострочное поле ввода."""
    def ch(
        self,
        lb: str = "",
        *,
        pr: Optional[Node] = None,
        chk: bool = False,
        clk: Optional[Callable[[], None]] = None,
        pd: float = 0.0,
        gp: float = 0.0,
        w: Optional[float] = None,
        h: Optional[float] = None,
    ) -> Node:
        """Флажок (CheckBox)."""
    def rd(
        self,
        lb: str = "",
        *,
        pr: Optional[Node] = None,
        grp: int = 0,
        on: bool = False,
        clk: Optional[Callable[[], None]] = None,
        pd: float = 0.0,
        gp: float = 0.0,
        w: Optional[float] = None,
        h: Optional[float] = None,
    ) -> Node:
        """Радиокнопка; `grp` — идентификатор группы выбора."""
    def sw(
        self,
        lb: str = "",
        *,
        pr: Optional[Node] = None,
        on: bool = False,
        clk: Optional[Callable[[bool], None]] = None,
        pd: float = 0.0,
        gp: float = 0.0,
        w: Optional[float] = None,
        h: Optional[float] = None,
    ) -> Node:
        """Переключатель (Switch)."""
    def sl(
        self,
        vl: float = 0.5,
        *,
        pr: Optional[Node] = None,
        ch: Optional[Callable[[float], None]] = None,
        pd: float = 0.0,
        gp: float = 0.0,
        w: Optional[float] = None,
        h: Optional[float] = None,
    ) -> Node:
        """Ползунок 0..1."""
    def spin(
        self,
        value: float = 0.0,
        *,
        pr: Optional[Node] = None,
        min: float = 0.0,
        max: float = 100.0,
        step: float = 1.0,
        ch: Optional[Callable[[float], None]] = None,
        pd: float = 0.0,
        gp: float = 6.0,
        w: Optional[float] = None,
        h: Optional[float] = None,
    ) -> Node:
        """Числовое поле с кнопками −/+."""
    def rsl(
        self,
        lo: float = 0.25,
        hi: float = 0.75,
        *,
        pr: Optional[Node] = None,
        ch: Optional[Callable[[float, float], None]] = None,
        pd: float = 0.0,
        gp: float = 0.0,
        w: Optional[float] = None,
        h: Optional[float] = None,
    ) -> Node:
        """Диапазонный ползунок двумя ручками."""
    def dl(
        self,
        vl: float = 0.5,
        *,
        pr: Optional[Node] = None,
        lb: str = "",
        ch: Optional[Callable[[float], None]] = None,
        bind: Optional[Callable[[], float]] = None,
        pd: float = 0.0,
        gp: float = 0.0,
        w: Optional[float] = None,
        h: Optional[float] = None,
    ) -> Node:
        """Круговой регулятор (тянуть вверх/вниз)."""

    # --- Списки и данные ---
    def lst(
        self,
        items: list[str],
        *,
        pr: Optional[Node] = None,
        sel: Optional[int] = None,
        ch: Optional[Callable[[int], None]] = None,
        pd: float = 0.0,
        gp: float = 0.0,
        w: Optional[float] = None,
        h: Optional[float] = None,
    ) -> Node:
        """Список с выбором пункта."""
    def tbl(
        self,
        columns: list[str],
        rows: list[list[str]],
        *,
        pr: Optional[Node] = None,
        ch: Optional[Callable[[int], None]] = None,
        hl: float = 0.0,
        vl: float = 0.0,
        pd: float = 0.0,
        gp: float = 0.0,
        w: Optional[float] = None,
        h: Optional[float] = None,
    ) -> Node:
        """Таблица. hl/vl — толщина линий строк/столбцов."""
    def tre(
        self,
        items: list[tuple[int, str, bool]],
        *,
        pr: Optional[Node] = None,
        ch: Optional[Callable[[int], None]] = None,
        pd: float = 0.0,
        gp: float = 0.0,
        w: Optional[float] = None,
        h: Optional[float] = None,
    ) -> Node:
        """Дерево. items: (глубина, текст, лист)."""
    def pg(
        self,
        rows: list[tuple[str, str]],
        *,
        pr: Optional[Node] = None,
        bind: Optional[Callable[[], list[tuple[str, str]]]] = None,
        ch: Optional[Callable[[int], None]] = None,
        pd: float = 0.0,
        gp: float = 0.0,
        w: Optional[float] = None,
        h: Optional[float] = None,
    ) -> Node:
        """Таблица свойств «ключ — значение»."""

    # --- Меню и навигация ---
    def mb(
        self,
        menus: list[tuple[str, list[str]]],
        *,
        pr: Optional[Node] = None,
        on_select: Optional[Callable[[int, int], None]] = None,
        pd: float = 0.0,
        gp: float = 0.0,
        w: Optional[float] = None,
        h: Optional[float] = None,
    ) -> Node:
        """Строка меню."""
    def crumb(
        self,
        items: list[str],
        *,
        pr: Optional[Node] = None,
        ch: Optional[Callable[[int], None]] = None,
        pd: float = 0.0,
        gp: float = 0.0,
        w: Optional[float] = None,
        h: Optional[float] = None,
    ) -> Node:
        """Хлебные крошки."""
    def pgn(
        self,
        total: int,
        *,
        pr: Optional[Node] = None,
        page: int = 0,
        ch: Optional[Callable[[int], None]] = None,
        pd: float = 0.0,
        gp: float = 0.0,
        w: Optional[float] = None,
        h: Optional[float] = None,
    ) -> Node:
        """Постраничная навигация."""
    def rat(
        self,
        vl: int = 0,
        *,
        pr: Optional[Node] = None,
        max: int = 5,
        ch: Optional[Callable[[int], None]] = None,
        pd: float = 0.0,
        gp: float = 0.0,
        w: Optional[float] = None,
        h: Optional[float] = None,
    ) -> Node:
        """Оценка звёздами."""

    # --- Выбор значений ---
    def cal(
        self,
        year: int = 2026,
        month: int = 7,
        day: int = 1,
        *,
        pr: Optional[Node] = None,
        ch: Optional[Callable[[int, int, int], None]] = None,
        pd: float = 0.0,
        gp: float = 0.0,
        w: Optional[float] = None,
        h: Optional[float] = None,
    ) -> Node:
        """Выбор даты."""
    def clr(
        self,
        hue: float = 0.58,
        sat: float = 0.75,
        val: float = 0.96,
        *,
        pr: Optional[Node] = None,
        ch: Optional[Callable[[str], None]] = None,
        pd: float = 0.0,
        gp: float = 0.0,
        w: Optional[float] = None,
        h: Optional[float] = None,
    ) -> Node:
        """Палитра цвета HSV; `ch` возвращает `#RRGGBB`."""
    def tm(
        self,
        hour: int = 12,
        minute: int = 0,
        *,
        pr: Optional[Node] = None,
        ch: Optional[Callable[[int, int], None]] = None,
        pd: float = 0.0,
        gp: float = 0.0,
        w: Optional[float] = None,
        h: Optional[float] = None,
    ) -> Node:
        """Выбор времени."""

    # --- Терминал ---
    def term(
        self,
        lines: list[str] = ...,
        *,
        pr: Optional[Node] = None,
        prompt: str = "$",
        on: Optional[Callable[[str], str]] = None,
        pd: float = 0.0,
        gp: float = 0.0,
        w: Optional[float] = None,
        h: Optional[float] = None,
    ) -> Node:
        """Консоль с вводом команд; `on(cmd)` возвращает строку вывода."""
    def term_clear(self, node: Node) -> None:
        """Очищает вывод терминала."""