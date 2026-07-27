"""Подсказки типов для SSUI. Реализация — в скомпилированном модуле."""

from typing import Callable, Optional, TypedDict

class TreeRow(TypedDict, total=False):
    """Строка дерева; все поля необязательны."""

    depth: int
    text: str
    leaf: bool
    open: bool
    values: list[str]
    bg: str
    fg: str
    icon: str
    cbg: list[str]
    cfg: list[str]

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

class Fl:
    """Файловые диалоги, возвращается `W.file()`."""

    def open(
        self,
        *,
        title: str = "",
        patterns: list[tuple[str, str]] = ...,
        on: Optional[Callable[[str], None]] = None,
    ) -> None:
        """Диалог выбора файла для открытия.

        `patterns` — пары `(описание, маска)`, например
        `("CSV", "*.csv")`. `on` получает путь; при отмене — пустую
        строку. Диалог модальный и показывается вне колбэка, поэтому
        `on` вызывается позже, а не мгновенно.
        """
    def save(
        self,
        *,
        title: str = "",
        name: str = "",
        patterns: list[tuple[str, str]] = ...,
        on: Optional[Callable[[str], None]] = None,
    ) -> None:
        """Диалог сохранения файла.

        `name` — имя файла по умолчанию. `on` получает путь либо
        пустую строку при отмене.
        """
    def dir(
        self,
        *,
        title: str = "",
        on: Optional[Callable[[str], None]] = None,
    ) -> None:
        """Диалог выбора папки; `on` получает путь либо ""."""

class Clip:
    """Буфер обмена, возвращается `W.clip()`."""

    def get(self) -> str:
        """Текст из буфера обмена; пустая строка, если он пуст.

        Переводы строк нормализуются в `\\n`. Безопасно из колбэков.
        """
    def set(self, text: str) -> None:
        """Кладёт текст в буфер обмена. Безопасно из колбэков."""

class Post:
    """Очередь вызовов в UI-потоке, возвращается `W.post()`.

    Единственный объект SSUI, который можно использовать
    из других потоков.
    """

    def __call__(self, f: Callable[[], None]) -> None:
        """Ставит вызов в очередь UI-потока. Безопасно из любого потока.

        Вызовы выполняются в порядке поступления, в начале ближайшего
        кадра, до раскладки и до опроса привязок. Если окно закрыто,
        вызов отбрасывается без ошибки.
        """

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
        """Модальный диалог с произвольными кнопками.

        Вызовы копятся в очереди: если запросить несколько диалогов
        подряд, они покажутся по очереди, ни один не потеряется.
        `on` вызывается ровно один раз: с индексом нажатой кнопки
        (0 слева направо) при клике, Enter или пробеле, либо с -1
        при отмене клавишей Esc. Правая кнопка считается основной —
        Enter без явного фокуса нажимает именно её. Клик мимо панели
        диалог не закрывает: он модальный.

        `message` переносится по словам; если текст не помещается
        в область, она становится прокручиваемой колесом мыши
        с индикатором справа. Для очень объёмного содержимого
        всё же лучше обычные виджеты окна или `scr`.
        """
    def msg(
        self,
        title: str,
        message: str,
        *,
        ok: str = "Ок",
        on: Optional[Callable[[int], None]] = None,
    ) -> None:
        """Окно с одной кнопкой; `on` получает 0 либо -1 при отмене."""
    def alert(
        self,
        message: str,
        *,
        title: str = "Внимание",
        ok: str = "Ок",
        on: Optional[Callable[[int], None]] = None,
    ) -> None:
        """Предупреждение с одной кнопкой; `on` — 0 либо -1 (отмена)."""

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
    def clip(self) -> Clip:
        """Возвращает контроллер буфера обмена."""
    def on_error(self, f: Callable[[str], None]) -> None:
        """Хук необработанных исключений в колбэках.

        `f` получает готовый traceback строкой. Без хука исключение
        печатается в `stderr` и теряется в сборке без консоли.
        Ставится в любой момент, в том числе после показа окна;
        повторный вызов заменяет прежний хук. Исключение внутри
        самого хука печатается в `stderr` и дальше не идёт.
        """
    def file(self) -> Fl:
        """Возвращает контроллер файловых диалогов."""
    def post(self) -> Post:
        """Возвращает очередь вызовов в UI-потоке.

        Вызывается до показа окна, из UI-потока. Полученный объект
        можно передавать в рабочие потоки.

        Прямая запись сигнала из рабочего потока не обновит окно:
        подписки биндингов хранятся отдельно в каждом потоке.
        Из рабочего потока — только `p(lambda: sig.st(v))`.
        """
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
    def show(self, node: Optional[Node] = None, on: bool = True) -> None:
        """Показывает окно, не блокируя цикл сообщений.

        Вызывается автоматически при выходе из `with`.

        С узлом работает иначе: `show(node, False)` убирает узел
        из раскладки целиком — вместе с потомками, — и его место
        достаётся соседям. Это отличается от `ghost`, который лишь
        отключает мышь, оставляя узел на экране и в потоке.

        Скрытие действует в любой раскладке: поток, сетка, упаковка,
        `scr` и тело `acc`. У `tab`, `stk` и `spl` место позиционное,
        поэтому там узел просто перестаёт рисоваться.

        Безопасно вызывать из колбэков и после показа окна.
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
    def screen() -> tuple[float, float]:
        """Размер основного экрана в пикселях: `(ширина, высота)`."""

    def size(self) -> tuple[float, float]:
        """Размер клиентской области окна: `(ширина, высота)`.

        Достоверен после показа окна; до показа — `(0, 0)`.
        """

    def move(self, x: float, y: float) -> None:
        """Перемещает окно; `(x, y)` — левый верхний угол на экране."""
    def on_resize(self, f: Callable[[float, float], None]) -> None:
        """Колбэк изменения размера окна; `f(width, height)`.

        Размер — клиентская область в пикселях. Событие приходит
        часто во время перетаскивания края; для тяжёлого пересчёта
        оборачивай в дебаунс через `after`. Задаётся до показа окна.
        """

    @staticmethod
    def frames(path: str) -> int:
        """Число кадров в файле изображения; для GIF — длина анимации.

        Кадр выбирается суффиксом пути: `img(src_bind=lambda: f"{gif}|{i}")`.
        """

    def ghost(self, node: Node, on: bool = True) -> None:
        """Делает узел прозрачным для мыши.

        Узел остаётся на экране и в раскладке — уходит только
        реакция на курсор. Чтобы убрать и место, нужен `show`.
        Безопасно вызывать из колбэков и после показа окна.
        """
    def front(self, node: Node, on: bool = True) -> None:
        """Поднимает узел поверх соседей при нажатии внутри него.

        Безопасно вызывать из колбэков и после показа окна.
        """
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
    def css(self, text: str, *, replace: bool = False) -> None:
        """Применяет CSS-подмножество из строки.

        Вызывается в любой момент, в том числе из колбэков и после
        показа окна: заявка применяется в начале следующего кадра.

        По умолчанию правила добавляются к накопленным, и при равной
        специфичности побеждает более позднее. `replace=True` целиком
        заменяет источник — это рабочий режим живой смены палитры,
        иначе строки копятся, а каскад пересчитывается всё дольше.

        Каскад ложится на узлы, существующие в момент применения,
        поэтому после `build` палитру надо применить заново.
        """
    def css_file(self, path: str, *, replace: bool = False) -> None:
        """Применяет CSS из файла; `replace` — как у `css`."""
    def build(self, f: Callable[[], None]) -> None:
        """Достраивает дерево виджетов; работает и после показа окна.

        Внутри `f` доступны все строители, включая `pop`, `bx` и
        `tre` — окно на время вызова забирает дерево себе. До показа
        `f` выполняется сразу, после — в начале следующего кадра.

        Родителем по умолчанию служит корень окна, поэтому для
        вставки в конкретный контейнер нужен явный `pr=`. Узлы
        стилей не наследуют: после достройки вызывайте `css`.
        """
    def css_hot(self, path: str) -> None:
        """Следит за CSS-файлом и перезагружает его на лету."""
    def tip_style(
        self,
        *,
        radius: float = 6.0,
        pad_x: float = 12.0,
        pad_y: float = 5.0,
    ) -> None:
        """Оформление тултипа: радиус и отступы вокруг текста."""

    def bindv(self, node: Node, f: Callable[[], float]) -> None:
        """Привязывает числовой колбэк к узлу.

        Значение применяется по типу узла: положение слайдера,
        прогресса, шкалы или циферблата; страница стопки `stk`;
        активная вкладка `tab`. Так вкладку переключают из кода —
        свяжите `tab` с сигналом и меняйте сигнал; клик по вкладке
        по-прежнему шлёт `ch`.
        """
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
    def every(self, ms: float, f: Callable[[], None]) -> int:
        """Периодический вызов каждые `ms` мс; возвращает идентификатор.

        Регистрируется в любой момент, в том числе из колбэка и после
        показа окна. Пока есть хотя бы один таймер, цикл отрисовки
        не засыпает.
        """
    def after(self, ms: float, f: Callable[[], None]) -> int:
        """Одноразовый отложенный вызов; возвращает идентификатор.

        Снимается сразу после срабатывания. Если это был последний
        таймер, цикл отрисовки снова засыпает.
        """
    def cancel(self, tid: int) -> None:
        """Отменяет таймер по идентификатору; повторный вызов безвреден.

        Отмена вступает в силу в начале следующего кадра. Отменить
        уже сработавший или чужой идентификатор безопасно.
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
    def hotkey(self, spec: str, f: Callable[[], None]) -> None:
        """Горячая клавиша уровня окна; задаётся до показа.

        `spec` — клавиши через `+`: модификаторы `ctrl`, `shift`,
        `alt` и одна основная клавиша. Основная — буква (`a`), цифра
        (`5`), функциональная (`f2`) или имя: `enter`, `space`, `tab`,
        `escape`, `delete`, `home`, `end`, `pageup`, `pagedown`,
        стрелки, `plus`, `minus`. Примеры: `"ctrl+c"`,
        `"ctrl+shift+a"`, `"f2"`, `"delete"`.

        Не срабатывает, когда фокус в поле ввода. `plus`/`minus`
        ловят и цифровой блок. При нескольких совпадениях вызывается
        зарегистрированный первым.
        """
    def rmb(self, node: Node, f: Callable[[float, float], None]) -> None:
        """Правый клик по узлу; `f(x, y)` в координатах окна.

        Обработчик ищется от узла под курсором вверх по родителям,
        поэтому его можно повесить на контейнер. Если найден, меню
        окна из `menu()` для этого клика не открывается.
        """
    def wheel(self, node: Node, f: Callable[[float, float, float], None]) -> None:
        """Колесо над узлом: дельта в щелчках (±1), x, y.

        Задаётся до показа окна. Узел перехватывает событие целиком:
        ни собственная прокрутка узла, ни прокрутка родителя
        не срабатывают. Обработчик ищется от узла под курсором вверх
        по родителям, поэтому его можно повесить на контейнер.

        Координаты отсчитываются от левого верхнего угла узла;
        для прокручиваемой канвы — в системе координат содержимого.
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
        """Панель-контейнер как контекст.

        Без `h` высота берётся от детей: сумма их высот плюс зазоры
        и отступы по вертикальной оси, максимум высоты — по
        горизонтальной. Считается, только если высота каждого
        потомка известна; иначе работает прежнее правило.

        В `scr`, в теле `acc` и при упаковке `pk` эта высота теперь
        и определяет размер — раньше там стоял фиксированный
        умолчательный слот, и вложенные дети налезали друг на друга.
        В обычном потоке она служит нижней границей: пока места
        хватает, контейнер по-прежнему тянется на свободную долю.
        """
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
        grp: int = 0,
        ch: Optional[Callable[[bool], None]] = None,
        rad: float = 10.0,
        pd: float = 8.0,
        gp: float = 8.0,
        w: Optional[float] = None,
        h: Optional[float] = None,
    ) -> Ctx:
        """Секция аккордеона как контекст.

        `ch(open)` вызывается при каждой смене состояния — и по
        клику, и из `acc_open`, и при автозакрытии по группе.

        `grp` — номер группы взаимного исключения; ноль отключает
        её. Открытие секции закрывает остальные секции той же
        группы, каждой из них приходит свой `ch(False)`.
        """
    def acc_open(self, node: Node, on: bool = True) -> None:
        """Открывает или сворачивает секцию аккордеона из кода.

        Безопасно вызывать из колбэков и после показа окна:
        заявка применяется в начале следующего кадра.
        """
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
        down: Optional[Callable[[int, float, float], None]] = None,
        move: Optional[Callable[[int, float, float], None]] = None,
        up: Optional[Callable[[int, float, float], None]] = None,
        dbl: Optional[Callable[[int, float, float], None]] = None,
        scroll: bool = False,
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
    `poly` — [x1, y1, x2, y2, ..., xn, yn, stroke]

    Контур `poly` замыкается автоматически; при чётной длине
    списка обводки нет и многоугольник заливается.
    Попадание в `poly` считается по площади, а не по контуру.

    `stroke` > 0 рисует контур, 0 — заливку.
    `ch` получает индекс фигуры под курсором или -1 при промахе.

    `down`, `move`, `up`, `dbl` получают `(индекс, x, y)`, где
    индекс — верхняя фигура под курсором или -1, а координаты
    отсчитываются от левого верхнего угла содержимого.
    `move` вызывается только при зажатой левой кнопке.
    `up` приходит даже если курсор ушёл за пределы области.
    `dbl` приходит дополнительно к `down`, а не вместо него.

    `scroll` включает прокрутку колесом и панорамирование
    перетаскиванием. Панорамирование отключается, если задан
    `move`: тогда левая кнопка целиком принадлежит приложению,
    а прокрутка остаётся за колесом и `cv_view`.
    Границы задаются `cv_region`; без него прокрутки не будет.
    """
    def cv_region(self, node: Node, x1: float, y1: float,
                  x2: float, y2: float) -> None:
        """Виртуальная область прокрутки в координатах содержимого.

        Безопасно вызывать из колбэков и после показа окна:
        запрос кладётся в очередь и применяется в начале кадра.
        """
    def cv_view(self, node: Node, x: float, y: float) -> None:
        """Прокручивает область к точке; значение зажимается областью.

        Безопасно вызывать из колбэков и после показа окна.
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
            data: Optional[bytes] = None,
            fit: str = "contain",
            fit_bind: Optional[Callable[[], float]] = None,
            pd: float = 0.0,
            gp: float = 0.0,
            w: Optional[float] = None,
            h: Optional[float] = None,
    ) -> Node:
        """Изображение из файла или из памяти.

        `src` — путь к файлу; `src_bind` — путь из колбэка,
        пересчитывается каждый кадр.
        Кадр анимированного GIF выбирается суффиксом: `"logo.gif|3"`.
        `fit`/`fit_bind` — режим вписывания:
        `contain`, `cover`, `fill`, `none`.

        `data` — байты закодированного изображения (PNG/JPEG),
        например график matplotlib из `BytesIO`. Задаётся при
        создании и имеет приоритет над `src`; с `src_bind` не
        сочетается.
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
            sel: Optional[int] | Optional[list[int]] = None,
            multi: bool = False,
            ch: Optional[Callable[[int], None]]
                | Optional[Callable[[list[int]], None]] = None,
            pd: float = 0.0,
            gp: float = 0.0,
            w: Optional[float] = None,
            h: Optional[float] = None,
    ) -> Node:
        """Список с выбором пункта.

        `multi` включает множественный выбор: Ctrl добавляет и
        убирает пункт, Shift выделяет диапазон от текущего.
        Форма `ch` определяется этим флагом: при `multi=False`
        приходит индекс, при `multi=True` — список индексов.
        `sel` задаёт начальный выбор: число или список номеров.
        """

    def lst_sel(self, node: Node, indexes: list[int]) -> None:
        """Задаёт выделенные пункты; первый становится текущим.

        Безопасно вызывать из колбэков и после показа окна.
        Индексы вне длины списка отбрасываются.
        """
    def tbl(
        self,
        columns: list[str],
        rows: list[list[str]],
        *,
        pr: Optional[Node] = None,
        ch: Optional[Callable[[int], None]]
            | Optional[Callable[[int, int], None]] = None,
        bg: Optional[Callable[[], list[tuple[tuple[int, int], str]]]] = None,
        hl: float = 0.0,
        vl: float = 0.0,
        pd: float = 0.0,
        gp: float = 0.0,
        w: Optional[float] = None,
        h: Optional[float] = None,
    ) -> Node:
        """Таблица. hl/vl — толщина линий строк/столбцов.

        Форма `ch` выбирается по числу параметров колбэка: один —
        приходит строка, два — строка и колонка. Старый однопара-
        метрический колбэк работает без изменений.

        `bg` — колбэков цветов ячеек: список `((строка, колонка), цвет)`,
        пересобирается при изменении зависимостей. Цвет ячейки
        рисуется поверх подсветки строки, но под текстом.
        """

    def tbl_see(self, node: Node, row: int) -> None:
        """Прокручивает таблицу к строке.

        Безопасно вызывать из колбэков и после показа окна.
        """
    def tre(
        self,
        items: list[TreeRow] | list[tuple[int, str, bool]],
        *,
        pr: Optional[Node] = None,
        cols: Optional[list[str]] = None,
        widths: Optional[list[float]] = None,
        multi: bool = False,
        bind: Optional[Callable[[], list[TreeRow]]] = None,
        ch: Optional[Callable[[int], None]]
            | Optional[Callable[[list[int]], None]] = None,
        clk: Optional[Callable[[int, int], None]] = None,
        dbl: Optional[Callable[[int, int], None]] = None,
        pd: float = 0.0,
        gp: float = 0.0,
        w: Optional[float] = None,
        h: Optional[float] = None,
    ) -> Node:
        """Дерево из плоского списка строк с полем `depth`.

        Строка — словарь `TreeRow` либо старый кортеж
        `(глубина, текст, лист)`; обе формы можно смешивать.

        `cols` — заголовки колонок; первая колонка всегда содержит
        дерево, остальные берутся из `values` по порядку.
        `widths` — ширины в пикселях; ноль или отсутствие означает
        «поделить остаток поровну».
        `bg` и `fg` — цвета фона строки и её текста в виде `#RRGGBB`.
        `cbg` и `cfg` — цвета отдельных ячеек по номерам колонок;
        пустая строка означает «без переопределения», список может
        быть короче числа колонок. Ячейка перекрывает и фон строки,
        и подсветку выбора — так видно окраску конкретного значения.
        `icon` — путь к файлу изображения слева от текста.

        `bind` пересобирает все строки при изменении зависимостей;
        выбор сохраняется, если индекс остался в пределах длины.

        `multi` включает множественный выбор: Ctrl добавляет и
        убирает строку, Shift выделяет диапазон от текущей.
        Форма `ch` определяется этим флагом: при `multi=False`
        приходит индекс строки, при `multi=True` — список индексов.

        `clk` и `dbl` получают `(строка, колонка)`; колонка ноль —
        та, в которой нарисовано дерево. Клик по треугольнику
        раскрытия их не вызывает.
        """
    def tre_sel(self, node: Node, indexes: list[int]) -> None:
        """Задаёт выделенные строки; первая становится текущей.

        Безопасно вызывать из колбэков и после показа окна.
        Индексы вне длины списка отбрасываются.
        """
    def tre_see(self, node: Node, index: int) -> None:
        """Прокручивает дерево к строке, раскрывая её предков.

        Если строка уже видна, прокрутка не меняется. Безопасно
        вызывать из колбэков и после показа окна.
        """
    def tre_open(self, node: Node, indexes: list[int] = ...,
                 *, on: bool = True) -> None:
        """Раскрывает или сворачивает поддеревья.

        Пустой список означает всё дерево. Для каждого индекса
        операция применяется к строке и всем её потомкам.
        Безопасно вызывать из колбэков и после показа окна.
        """
    def pop(self, *, w: float = 240.0, h: float = 40.0,
            on_close: Optional[Callable[[], None]] = None) -> Ctx:
        """Слой поверх всего окна; содержимое строится внутри `with`.

        Узел цепляется к корню окна, а не к текущему родителю,
        поэтому не обрезается границами `scr` и рисуется поверх
        любых соседей.

        Создаётся скрытым. Показывается `pop_at`, прячется `pop_off`
        либо сам — по клику мимо и по Escape; в этих двух случаях
        вызывается `on_close`.

        Открытым может быть только один слой на окно: показ второго
        просто перехватывает управление на себя.
        """
    def pop_at(self, node: Node, x: float, y: float,
               w: float = 0.0, h: float = 0.0) -> None:
        """Показывает слой в прямоугольнике окна.

        Нулевые `w` и `h` означают «оставить прежний размер».
        Безопасно вызывать из колбэков и после показа окна.
        """
    def pop_off(self, node: Node) -> None:
        """Прячет слой; `on_close` при этом не вызывается.

        Безопасно вызывать из колбэков и после показа окна.
        """
    def tre_cell(self, node: Node, index: int,
                 col: int) -> tuple[float, float, float, float]:
        """Прямоугольник ячейки в координатах окна: `(x, y, w, h)`.

        Возвращает нули, если строка скрыта свёрнутым предком либо
        уползла за границы видимой части. Геометрия берётся из
        последнего отрисованного кадра, поэтому вызывать следует
        после того, как раскладка устоялась — например из `clk`.
        """
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