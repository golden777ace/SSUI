# SSUI

## Описание

GUI-библиотека для Windows на Rust (Direct3D 11 + Direct2D + DirectWrite)
с публичным API на Python (PyO3, maturin).

## Технические решения

| Вопрос | Решение |
|--------|---------|
| Язык ядра | Rust |
| Платформа | Windows only (10/11) |
| Рендеринг | D3D11 + Direct2D + DirectWrite |
| Прозрачность | DirectComposition + DWM blur |
| Изображения | WIC (PNG/JPEG) |
| Биндинги WinAPI | crate `windows` 0.62 |
| Публичный API | Python 3.11+ через PyO3 (maturin) |
| Парадигма | Retained-mode |
| Реактивность | Сигналы (fine-grained) |
| Раскладка | Линейная + constraint (grow/align/pin) |
| Стилизация | Токены/темы + CSS-подмножество |
| Темы | 4: white, light, dark, black |
| Лицензия | MIT, соло-разработка |

## Архитектура и файлы

- `core/` (crate `ssui-core`): `platform/` — окно, ввод, DPI; `render/` — D3D11, Canvas; `theme.rs` — темы; `tree/` — дерево, раскладка, CSS, анимации.
- `python/` (crate `ssui`): `src/lib.rs` — биндинги PyO3; `examples/` — примеры.

## Палитра

| Тема | Фон | Силуэты | Содержимое |
|------|-----|---------|------------|
| white | `#FFFFFF` | `#F5F5F5` | `#AAAAAA` |
| light | — | — | — |
| dark | — | — | — |
| black | `#000000` | `#1A1A1A` | `#424242` |

## Каталог виджетов

Легенда: ✅ есть · ⏳ план.

### Окна и диалоги
- ✅ Window (`W`) · Dialog (`dlg`) · Tooltip (`tip=`) · Toast (`toast=`)
- ✅ Message Box (`dlg.msg`) · Alert (`dlg.alert`) · Notification (`nt`) · Snackbar (`nt.snack`)

### Контейнеры
- ✅ Frame/Box (`fr`, `bx`) · GroupBox (`grp`) · Scroll Area (`scr`) · Splitter (`spl`)
- ✅ Tabs (`tab`) · Accordion (`acc`) · Stack (`stk`) · Canvas (`cv`) · Dock (`dock`) · Drop Area (`drop`)

### Отображение
- ✅ Label (`lb`) · Image (`img`) · Icon (`icon=`) · Separator (`sep`) · Progress Bar (`pr`)
- ✅ Spinner (`spn`) · Gauge (`gg`) · Chart (`cht`) · Meter (`mt`) · Status Bar (`stb`) · Badge (`bdg`)

### Кнопки и действия
- ✅ Button (`bt`) · Toggle Button (`tgl`) · Dropdown (`dd`) · Link (`lnk`) · Split Button (`sbt`)

### Ввод
- ✅ Text Entry (`tx`) · Text Area (`ta`) · CheckBox (`ch`) · Radio (`rd`) · Switch (`sw`)
- ✅ Slider (`sl`) · SpinBox (`spin`) · Range Slider (`rsl`) · Dial/Knob (`dl`)

### Списки и данные
- ✅ ListBox (`lst`) · Table (`tbl`) · Tree View (`tre`) · Property Grid (`pg`)

### Меню и навигация
- ✅ Menu (`menu`) · Menu Bar (`mb`) · Breadcrumbs (`crumb`) · Pagination (`pgn`) · Rating (`rat`)

### Выбор значений
- ✅ Calendar (`cal`) · Color Picker (`clr`) · Time Picker (`tm`)
- ⏳ Date Picker, DateTime Picker

### Мультимедиа
- ✅ Terminal (`term`)
- ⏳ WebView, Video Player, Audio Player, Map View

## Параметры виджетов

### Контейнеры
- `bx(rad=12.0, *, pr=None, ax="v", pd=0.0, gp=0.0, w=None, h=None, elev=0.0)`
- `fr(rad=12.0, *, pr=None, ax="v", pd=0.0, gp=0.0, w=None, h=None, elev=0.0)`
- `grp(title="", *, pr=None, rad=12.0, ax="v", pd=12.0, gp=8.0, w=None, h=None)`
- `scr(*, pr=None, pd=8.0, gp=8.0, w=None, h=None)`
- Содержимое `scr` обрезается по его рамке и не ловит мышь за её пределами; обрезка наследуется вложенными узлами
- `spl(*, pr=None, ratio=0.5, vertical=True, w=None, h=None)`
- `win.spl_ratio(node, ratio)` — положение разделителя из кода, 0.1..0.9
- `tab(labels, *, pr=None, sel=0, ch=None, pd=0.0, gp=0.0, w=None, h=None)`
- `acc(title="", *, pr=None, open=False, grp=0, ch=None, rad=10.0, pd=8.0, gp=8.0, w=None, h=None)`
- `win.acc_open(node, on=True)` — открыть или свернуть секцию из кода
- `stk(*, pr=None, page=0, bind=None, w=None, h=None)`
- `cv(shapes=[], *, pr=None, bind=None, scroll=False, down=None, move=None, up=None, dbl=None, pd=0.0, gp=0.0, w=None, h=None)`
- `win.cv_region(node, x1, y1, x2, y2)`, `win.cv_view(node, x, y)`
- `dock(ttl="", *, pr=None, side="l", size=260.0, open=True, ax="v", pd=10.0, gp=8.0)`
- `drop(txt="Перетащите файлы сюда", *, pr=None, on=None, pd=0.0, gp=0.0, w=None, h=None)`

### Отображение
- `lb(txt="", *, pr=None, bind=None, icon=None, pd=0.0, gp=0.0, w=None, h=None, wrap=False)`
- `img(src, *, pr=None, src_bind=None, data=None, fit="contain", fit_bind=None, pd=0.0, gp=0.0, w=None, h=None)`
- `sep(*, pr=None, vertical=False, pd=0.0, gp=0.0, w=None, h=None)`
- `pr(vl=0.0, *, pr=None, bind=None, pd=0.0, gp=0.0, w=None, h=None)`
- `spn(*, pr=None, pd=0.0, gp=0.0, w=None, h=None)`
- `gg(value=0.0, *, pr=None, lb="", bind=None, pd=0.0, gp=0.0, w=None, h=None)`
- `cht(data, *, pr=None, bind=None, pd=0.0, gp=0.0, w=None, h=None)`
- `mt(vl=0.0, *, pr=None, bind=None, seg=10, pd=0.0, gp=0.0, w=None, h=None)`
- `stb(txt="", *, pr=None, bind=None, pd=0.0, gp=0.0, w=None, h=None)`
- `bdg(txt="", *, pr=None, dot=False, bind=None, pd=0.0, gp=0.0, w=None, h=None)`

### Кнопки и действия
- `bt(lb="", *, pr=None, rad=10.0, icon=None, tip=None, toast=None, pd=0.0, gp=0.0, w=None, h=None, clk=None, elev=0.0)`
- `tgl(lb="", *, pr=None, on=False, clk=None, pd=0.0, gp=0.0, w=None, h=None)`
- `dd(options, *, pr=None, sel=0, ch=None, pd=0.0, gp=0.0, w=None, h=None)`
- `lnk(lb="", *, pr=None, clk=None, pd=0.0, gp=0.0, w=None, h=None)`
- `sbt(lb, opts, *, pr=None, clk=None, ch=None, rad=10.0, pd=0.0, gp=0.0, w=None, h=None)`

### Ввод
- `tx(txt="", *, pr=None, sig=None, ph="", pd=0.0, gp=0.0, w=None, h=None)`
- `ta(txt="", *, pr=None, sig=None, ph="", pd=0.0, gp=0.0, w=None, h=None)`
- `ch(lb="", *, pr=None, chk=False, clk=None, pd=0.0, gp=0.0, w=None, h=None)`
- `rd(lb="", *, pr=None, grp=0, on=False, clk=None, pd=0.0, gp=0.0, w=None, h=None)`
- `sw(lb="", *, pr=None, on=False, clk=None, pd=0.0, gp=0.0, w=None, h=None)`
- `sl(vl=0.5, *, pr=None, ch=None, pd=0.0, gp=0.0, w=None, h=None)`
- `spin(value=0.0, *, pr=None, min=0.0, max=100.0, step=1.0, ch=None, pd=0.0, gp=6.0, w=None, h=None)`
- `rsl(lo=0.25, hi=0.75, *, pr=None, ch=None, pd=0.0, gp=0.0, w=None, h=None)`
- `dl(vl=0.5, *, pr=None, lb="", ch=None, bind=None, pd=0.0, gp=0.0, w=None, h=None)`

### Списки и данные
- `lst(items, *, pr=None, sel=None, multi=False, ch=None, pd=0.0, gp=0.0, w=None, h=None)`
- `win.lst_sel(node, indexes)`
- `tbl(columns, rows, *, pr=None, ch=None, bg=None, hl=0.0, vl=0.0, pd=0.0, gp=0.0, w=None, h=None)`
- `win.tbl_see(node, row)`
- `win.cols(node, *, widths=None, mins=None, head=None, resize=None, on_cols=None)` — колонки и шапка `tbl` и `tre`
- `head` — высота шапки; выше умолчания в 34 px заголовок переносится по словам, работает и `\n`
- Границы колонок тянутся мышью в шапке: захват ±4 px, ход ограничен минимумами соседей, общая ширина пары сохраняется
- Первое перетаскивание фиксирует все колонки в текущих пикселях; двойной клик по границе — автоширина по содержимому
- `resize=False` запрещает перетаскивание; `on_cols(widths)` приходит после отпускания и после автоширины, задаётся до показа окна

### Раскрытие групп в `tre`
- `tre(..., on_open=None)` — `on_open(index, open)` по клику на треугольник и из `tre_open`; при пересборке `bind` не вызывается
- `win.tre_opened(node)` — индексы раскрытых строк; листья не попадают
- `win.tre_open(node, indexes=[], *, on=True)` — раскрыть или свернуть поддеревья; пустой список — всё дерево
- Ключ `open` в `TreeRow` необязателен: без него строка забирает прежнее состояние узла с той же глубиной и подписью, поэтому раскрытие переживает пересборку `bind`
- Сверяется сперва строка на том же месте, затем первая подходящая по дереву — добавление, удаление и перестановка соседей раскрытие не ломают
- Заданный `open` остаётся принудительной установкой и перекрывает сохранённое

### Управление состоянием ввода
- `win.val(node, value)` — числовое состояние: `sl`, `pb`, `gau`, `mtr`, `dial`, `ch`, `sw`, `tgl`, `rd`, `dd`, `tab`, `stk`, `lst`, `pgr`, `rat`, `spin`
- `win.chk(node, on=True)` — состояние флажка, переключателя, тумблера, радиокнопки
- `win.dd_sel(node, index)` — выбранный пункт `dd`, `tab`, `stk`, `lst`
- `win.txt(node, text)` — текст `tx` и `ta`; каретка в конец, выделение снимается
- `win.items(node, items)` — пункты `dd` и `lst`; выбор сбрасывается
- `win.cal_set(node, y, m, d)`, `win.tm_set(node, h, m)`, `win.clr_set(node, hue, sat, val)`, `win.rsl_set(node, lo, hi)`
- `win.enable(node, on=True)` — доступность узла и всего поддерева; отключённый не ловит мышь, не берёт фокус, рисуется приглушённым
- `win.pwd(node, on=True)` — содержимое `tx` и `ta` точками; маскируется только отрисовка, значение остаётся настоящим

### Контекстные меню
- `win.menu(items, *, on_select=None)` — меню на всё окно; пустой список отключает его
- `win.popup(items, x, y, *, on_select=None)` — открыть меню в точке; пустой список закрывает
- Связка для меню узла: `win.menu([], on_select=f)` при сборке, `win.rmb(node, g)`, а в `g` — `win.popup(items, x, y)`
- `on_select` ставится на корень, поэтому хотя бы раз должен быть задан до показа окна; дальше меняются только пункты
- Пункты `popup` живут до закрытия меню и оконный набор из `menu` не затирают
- ПКМ по `tre` сначала выделяет строку под курсором, как ЛКМ, и лишь затем вызывает `rmb`; строка внутри готового выделения его не сбрасывает
- Клик по пустой области `tre` снимает выделение: одиночное дерево получает `ch(-1)`, дерево с `multi=True` — `ch([])`
- Колбэки при программной установке не вызываются; все методы безопасны после показа окна и из колбэков
- `sig=` двусторонний у `tx`, `ta`, `dd`, `ch`, `sw`, `tgl`, `sl`, `rat`, `rsl`, `cal`, `tm`, `clr`: ввод пишет в сигнал, запись в сигнал меняет виджет
- Совпадающее значение не применяется, поэтому обратная запись не сбивает набор с клавиатуры
- У `rd` параметра `sig` нет: сигнал одной кнопки не узнаёт, что её погасил выбор соседа — держите один сигнал на группу и `win.bindv(btn, lambda k=k: float(sel() == k))`
- `tre(items, *, pr=None, cols=None, widths=None, multi=False, bind=None, ch=None, clk=None, dbl=None, pd=0.0, gp=0.0, w=None, h=None)`
- `win.tre_see(node, index)`, `win.tre_sel(node, indexes)`
- `win.tre_open(node, index, open)`, `win.tre_cell(node, index, col)`
- `pg(rows, *, pr=None, bind=None, ch=None, pd=0.0, gp=0.0, w=None, h=None)`

### Меню и навигация
- `menu(items, *, on_select=None)`
- `mb(menus, *, pr=None, on_select=None, pd=0.0, gp=0.0, w=None, h=None)`
- `crumb(items, *, pr=None, ch=None, pd=0.0, gp=0.0, w=None, h=None)`
- `pgn(total, *, pr=None, page=0, ch=None, pd=0.0, gp=0.0, w=None, h=None)`
- `rat(vl=0, *, pr=None, max=5, ch=None, pd=0.0, gp=0.0, w=None, h=None)`

### Выбор значений
- `cal(year=2026, month=7, day=1, *, pr=None, ch=None, pd=0.0, gp=0.0, w=None, h=None)`
- `clr(hue=0.58, sat=0.75, val=0.96, *, pr=None, ch=None, pd=0.0, gp=0.0, w=None, h=None)`
- `tm(hour=12, minute=0, *, pr=None, ch=None, pd=0.0, gp=0.0, w=None, h=None)`

### Терминал
- `term(lines=[], *, pr=None, prompt="$", on=None, pd=0.0, gp=0.0, w=None, h=None)`

### Диалоги и уведомления
- `win.dlg()(title, message, buttons, *, on=None)` — `on` вызывается ровно один раз: индекс кнопки либо `-1` при отмене (Esc); длинное сообщение прокручивается колесом
- `dlg.msg(title, message, *, ok="Ок", on=None)`
- `dlg.alert(message, *, title="Внимание", ok="Ок", on=None)`
- `win.nt()(title, text, *, secs=4.0, action="", corner="tr", on=None)`, `win.nt().snack(text, *, corner="", ...)`
- `corner` ∈ `tl`, `tr`, `bl`, `br` — отсчёт от углов окна, не экрана; пустой у `snack` оставляет центр внизу
- `nt.snack(text, *, secs=4.0, action="", on=None)`

### Управление окном и раскладкой
- `win.thm()("wht"/"lit"/"drk"/"blk")`
- `win.fx()(sig, to, *, frm=None, dur=0.3, ease="out")`
- `win.tint(sig)`, `win.blur(sig)`, `win.blur_mode(sig)`, `win.drag_smooth(sig)`
- `win.css(text, *, replace=False)`, `win.css_file(path, *, replace=False)`, `win.css_hot(path)`
- `win.build(f)` — достройка дерева после показа окна; внутри работают все строители
- `win.grow(node, g)`, `win.align(node, *, justify="st", cross="str")`, `win.pin(node, *, l=None, t=None, r=None, b=None)`
- Контейнер без `h` берёт высоту по детям: в `scr`, теле `acc` и упаковке `pk` — как размер, в обычном потоке — как нижняя граница
- `win.lim(node, *, min_w=None, min_h=None, max_w=None, max_h=None)` — пределы размеров; ноль в максимуме снимает его
- Узел без явного `lim` берёт минимум из содержимого: вдоль своей оси суммой, поперёк максимумом; `tbl` и `tre` отдают сумму минимумов колонок
- Явный `lim` подъём отменяет; прокрутка при нехватке места не появляется — для неё нужен `scr`
- `win.ghost(node, on=True)`, `win.front(node, on=True)`, `win.show(node, on=True)`
- `win.bindv/bindb/bindl/bindt/bindz(node, callback)` — `bindv` задаёт также страницу `stk` и активную вкладку `tab`
- `win.screen()`, `win.size()`, `win.move(x, y)`, `win.on_resize(f)`

### Потоки, время и ввод
- `win.post()(f)` — вызов в UI-потоке из любого потока
- `win.after(ms, f)`, `win.every(ms, f)`, `win.cancel(tid)`
- `win.hotkey(spec, f)` — `"ctrl+c"`, `"ctrl+shift+a"`, `"f2"`, `"delete"`
- `win.wheel(node, f)`, `win.rmb(node, f)`
- `win.clip()` — `get()`, `set(text)`
- `win.file()` — `open(...)`, `save(...)`, `dir(...)`
- `win.pop(x=, y=, w=, h=, on_close=None)`, `win.pop_at(node)`, `win.pop_off()`

## Сокращения

### Виджеты и контейнеры
- W(Window); S(Signal); bx(box); fr(frame); grp(group); scr(scroll)
- spl(splitter); tab(tabs); acc(accordion); stk(stack); cv(canvas)
- dock(dock); drop(drop area); lb(label); img(image); sep(separator)
- pr(progress); spn(spinner); gg(gauge); cht(chart); mt(meter)
- stb(status bar); bdg(badge); bt(button); tgl(toggle); dd(dropdown)
- lnk(link); sbt(split button); tx(text box); ta(text area); ch(checkbox)
- rd(radio); sw(switch); sl(slider); spin(spinbox); rsl(range slider)
- dl(dial); lst(list box); tbl(table); tre(tree view); pg(property grid)
- mb(menu bar); crumb(breadcrumbs); pgn(pagination); rat(rating)
- cal(calendar); clr(color); tm(time); term(terminal)

### Управление узлом
- tip(tooltip); cls(class); pl(place); gr(grid); pk(pack); dep(depth)
- lim(limits — пределы размеров); cols(columns — колонки таблицы и дерева)
- css(css); rt(root); thm(theme); fx(effects); dlg(dialog); nt(notification)
- fnt(font); clip(clipboard); file(file dialogs); pop(popup layer)
- rmb(right mouse button); hotkey(hotkey); post(post to UI thread)

### Параметры (общие для многих виджетов)
- pr(parent) — не путать с методом `pr` (progress bar), различаются по контексту
- pd(padding); gp(gap); w(width); h(height); ax(axis); rad(radius)
- vl(value); lb(label — текст); clk(click callback)
- ch(change callback) — не путать с методом `ch` (checkbox)
- sel(selected); chk(checked); grp(group id) — не путать с методом `grp` (group box)
- lo/hi(low/high — границы диапазона); ttl(title); txt(text); elev(elevation)