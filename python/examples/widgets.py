import ssui

CSS = """
.clear { background: #00000000; }
.scrim { background: #00000099; }
.card  { background: #171A21F2; }
"""

TREE = [
    (0, "Проект", False),
    (1, "core", False),
    (2, "device.rs", True),
    (1, "python", True),
]

DATA = [0.4, 0.9, 0.6, 1.0, 0.7, 0.3, 0.85]

SHAPES = [
    ("rect", [16.0, 16.0, 300.0, 160.0, 12.0, 2.0], "#4B5563", ""),
    ("circle", [166.0, 96.0, 48.0, 0.0], "#22C55E", ""),
    ("line", [16.0, 190.0, 316.0, 190.0, 3.0], "#3B82F6", ""),
    ("text", [16.0, 200.0, 300.0, 26.0], "#EEF3FF", "Canvas"),
]


def main():
    win = ssui.W("SSUI · каталог виджетов", 1180, 820,
                 thm="drk", glass=True, tint=0.0, blur=False)
    thm = win.thm()
    nt = win.nt()
    dlg = win.dlg()
    fnt = win.fnt()

    page = ssui.sgnl(0)
    log = ssui.sgnl("—")

    FONTS = ["Segoe UI", "Consolas", "Georgia", "Times New Roman"]
    fam = ssui.sgnl("Segoe UI")
    sz = ssui.sgnl(20)

    def apply_font():
        fnt(fam(), float(sz()))

    def show(i):
        return lambda: page.st(i)

    names = [
        "Label", "Button", "Toggle", "Link", "Checkbox", "Switch",
        "Radio", "Separator", "SpinBox", "TextBox", "TextArea",
        "Dropdown", "ListBox", "Table", "Image", "Slider",
        "RangeSlider", "Dial", "ProgressBar", "Gauge", "Meter",
        "Chart", "Spinner", "StatusBar", "Badge", "Breadcrumbs",
        "Pagination", "Rating", "TreeView", "PropertyGrid", "Calendar",
        "ColorPicker", "TimePicker", "SplitButton", "MenuBar", "Canvas",
        "Terminal", "DropArea", "Dock", "GroupBox", "Accordion",
        "ScrollArea", "Stack", "Splitter", "Tabs", "Tooltip",
        "Toast", "Dialog", "MsgBox", "Alert", "Notify", "Snackbar",
    ]

    def body(name):
        if name == "Label":
            win.lb("Обычная метка", h=40.0)
        elif name == "Button":
            win.bt("Нажми меня", h=48.0, clk=lambda: log.st("клик"))
        elif name == "Toggle":
            win.tgl("Режим", h=48.0, clk=lambda v: log.st(f"toggle={v}"))
        elif name == "Link":
            win.lnk("Ссылка", clk=lambda: log.st("ссылка"), h=36.0)
        elif name == "Checkbox":
            win.ch("Флажок", chk=True, h=36.0)
        elif name == "Switch":
            win.sw("Wi-Fi", on=True, h=40.0)
        elif name == "Radio":
            win.rd("Первый", grp=7, on=True, h=32.0)
            win.rd("Второй", grp=7, h=32.0)
        elif name == "Separator":
            win.lb("Сверху", h=30.0)
            win.sep()
            win.lb("Снизу", h=30.0)
        elif name == "SpinBox":
            win.spin(3, min=0, max=10, step=1, h=48.0)
        elif name == "TextBox":
            win.tx("", h=48.0)
        elif name == "TextArea":
            win.ta("Выделение мышью работает.", h=220.0)
        elif name == "Dropdown":
            win.dd(["Один", "Два", "Три"], sel=0, h=48.0)
        elif name == "ListBox":
            win.lst([f"Пункт {i}" for i in range(12)], h=260.0)
        elif name == "Table":
            win.tbl(["Имя", "Тип", "Размер"],
                    [[f"file{i}.rs", "rust", f"{i * 3} КБ"] for i in range(12)],
                    hl=1.0, vl=1.0, h=300.0)
        elif name == "Image":
            win.img("test.png", h=200.0)
        elif name == "Slider":
            win.sl(0.5, ch=lambda v: log.st(f"{int(v * 100)}%"), h=44.0)
        elif name == "RangeSlider":
            win.rsl(0.25, 0.75,
                    ch=lambda a, b: log.st(f"{int(a * 100)}–{int(b * 100)}%"),
                    h=44.0)
        elif name == "Dial":
            win.dl(0.4, lb="MIX", ch=lambda v: log.st(f"{int(v * 100)}%"),
                   h=160.0)
        elif name == "ProgressBar":
            win.pr(0.6, h=20.0)
        elif name == "Gauge":
            win.gg(0.7, lb="VOL", h=160.0)
        elif name == "Meter":
            win.mt(0.55, seg=12, h=30.0)
        elif name == "Chart":
            win.cht(DATA, h=220.0)
        elif name == "Spinner":
            win.spn(h=60.0)
        elif name == "StatusBar":
            win.stb("Готово · всё в порядке", h=34.0)
        elif name == "Badge":
            with win.bx(ax="h", gp=10.0, h=36.0) as row:
                win.cls(row, "clear")
                win.lb("Входящие", h=36.0)
                win.bdg("7", h=26.0)
                win.bdg(dot=True, h=26.0)
        elif name == "Breadcrumbs":
            win.crumb(["Проект", "core", "render"],
                      ch=lambda i: log.st(f"крошка {i}"), h=36.0)
        elif name == "Pagination":
            win.pgn(5, page=0, ch=lambda i: log.st(f"стр. {i + 1}"), h=46.0)
        elif name == "Rating":
            win.rat(4, max=5, ch=lambda v: log.st(f"{v}/5"), h=44.0)
        elif name == "TreeView":
            win.tre(TREE, ch=lambda i: log.st(f"узел {i}"), h=240.0)
        elif name == "PropertyGrid":
            win.pg([("Тема", "drk"), ("Ширина", "1180"), ("Высота", "820")],
                   h=200.0)
        elif name == "Calendar":
            win.cal(2026, 7, 14,
                    ch=lambda y, m, d: log.st(f"{d:02d}.{m:02d}.{y}"), h=320.0)
        elif name == "ColorPicker":
            win.clr(ch=lambda c: log.st(c), h=240.0)
        elif name == "TimePicker":
            win.tm(12, 0, ch=lambda h, m: log.st(f"{h:02d}:{m:02d}"), h=130.0)
        elif name == "SplitButton":
            win.sbt("Сохранить", ["Как…", "Экспорт", "Печать"],
                    clk=lambda: log.st("сохранено"),
                    ch=lambda i: log.st(f"пункт {i}"), h=48.0)
        elif name == "MenuBar":
            win.mb([("Файл", ["Открыть", "Выход"]), ("Правка", ["Отменить"])],
                   on_select=lambda m, i: log.st(f"меню {m}·{i}"), h=42.0)
        elif name == "Canvas":
            win.cv(SHAPES, h=240.0)
        elif name == "Terminal":
            win.term(["SSUI shell."], prompt="$",
                     on=lambda c: f"echo: {c}", h=300.0)
        elif name == "DropArea":
            win.drop("Перетащите файлы сюда",
                     on=lambda ps: log.st(f"{len(ps)} файл(ов)"), h=180.0)
        elif name == "Dock":
            with win.bx(ax="h", gp=10.0, h=260.0) as row:
                win.cls(row, "clear")
                with win.dock("Панель", side="l", size=200.0, gp=8.0):
                    win.lb("Внутри дока", h=30.0)
                    win.bt("Кнопка", h=40.0)
                win.lb("Клик по шапке — свернуть", h=40.0)
        elif name == "GroupBox":
            with win.grp("Группа", gp=8.0, h=160.0):
                win.ch("Флажок", h=32.0)
                win.sw("Свитч", h=36.0)
                elif name == "Accordion":
                secs = []
                for nm in ("Первая", "Вторая", "Третья"):
                    with win.acc(nm, grp=7, h=150.0,
                                 ch=lambda v, n=nm: log.st(
                                     f"{n} — {'открыта' if v else 'закрыта'}")) as a:
                        secs.append(a)
                        win.lb(f"Содержимое: {nm}", h=32.0)
                        win.bt("Кнопка", h=40.0)
                with win.bx(ax="h", gp=8.0, h=52.0) as arow:
                    win.cls(arow, "clear")
                    win.bt("Открыть первую", h=44.0,
                           clk=lambda: win.acc_open(secs[0], True))
                    win.bt("Свернуть все", h=44.0,
                           clk=lambda: [win.acc_open(s, False) for s in secs])
        elif name == "ScrollArea":
            with win.scr(h=240.0):
                for i in range(14):
                    win.lb(f"Строка {i}", h=32.0)
        elif name == "Stack":
            pg = ssui.sgnl(0)
            with win.bx(ax="h", gp=8.0, h=48.0) as row:
                win.cls(row, "clear")
                win.bt("1", h=40.0, clk=lambda: pg.st(0))
                win.bt("2", h=40.0, clk=lambda: pg.st(1))
            with win.stk(h=120.0) as st:
                win.lb("Страница 1", h=40.0)
                win.lb("Страница 2", h=40.0)
            win.bindv(st, lambda: float(pg()))
        elif name == "Splitter":
            with win.spl(h=240.0):
                win.lb("Слева", h=40.0)
                win.lb("Справа", h=40.0)
        elif name == "Tabs":
            with win.tab(["А", "Б"], h=220.0) as tabs:
                with win.bx(pr=tabs, pd=10.0):
                    win.lb("Вкладка А", h=40.0)
                with win.bx(pr=tabs, pd=10.0):
                    win.lb("Вкладка Б", h=40.0)
        elif name == "Tooltip":
            win.bt("Наведи курсор", tip="Это подсказка", h=48.0)
        elif name == "Toast":
            win.bt("Показать toast", h=48.0, toast="Готово ✓")
        elif name == "Dialog":
            win.bt("Открыть диалог", h=48.0,
                   clk=lambda: dlg("Диалог", "Модальное окно ядра.",
                                   ["Отмена", "Ок"],
                                   on=lambda i: log.st(f"кнопка {i}")))
        elif name == "MsgBox":
            win.bt("Message Box", h=48.0,
                   clk=lambda: dlg.msg("Готово", "Файл сохранён."))
        elif name == "Alert":
            win.bt("Alert", h=48.0,
                   clk=lambda: dlg.alert("Не удалось открыть файл."))
        elif name == "Notify":
            win.bt("Notification", h=48.0,
                   clk=lambda: nt("Обновление", "Доступна версия 0.15",
                                  action="Позже",
                                  on=lambda: log.st("уведомление")))
        elif name == "Snackbar":
            win.bt("Snackbar", h=48.0,
                   clk=lambda: nt.snack("Элемент удалён", action="Отменить",
                                        on=lambda: log.st("отменено")))
        else:
            win.lb(name, h=40.0)

    with win.bx(pd=16.0, gp=12.0) as root:
        win.cls(root, "clear")

        with win.bx(ax="h", gp=8.0, h=54.0) as head:
            win.cls(head, "clear")
            win.lb("Каталог виджетов", w=280.0, h=40.0)
            win.bt("wht", w=80.0, h=44.0, clk=lambda: thm("wht"))
            win.bt("lit", w=80.0, h=44.0, clk=lambda: thm("lit"))
            win.bt("drk", w=80.0, h=44.0, clk=lambda: thm("drk"))
            win.bt("blk", w=80.0, h=44.0, clk=lambda: thm("blk"))
            win.lb(bind=lambda: f"Событие: {log()}", h=40.0)

        with win.bx(ax="h", gp=8.0, h=54.0) as fbar:
                win.cls(fbar, "clear")
                win.lb("Шрифт:", w=80.0, h=40.0)
                win.dd(FONTS, sel=0, w=200.0, h=44.0,
                       ch=lambda i: (fam.st(FONTS[i]), apply_font()))
                win.lb("Размер:", w=80.0, h=40.0)
                win.spin(20, min=10, max=48, step=1, w=170.0, h=44.0,
                         ch=lambda v: (sz.st(int(v)), apply_font()))
                win.lb(bind=lambda: f"{fam()} · {int(sz())} px", h=40.0)

        with win.scr(gp=8.0, pd=6.0) as area:
            win.cls(area, "clear")
            for start in range(0, len(names), 6):
                chunk = names[start:start + 6]
                with win.bx(ax="h", gp=8.0, h=52.0) as row:
                    win.cls(row, "clear")
                    for offset, name in enumerate(chunk):
                        win.bt(name, h=48.0, clk=show(start + offset + 1))

    overlay = None
    blank = None
    with win.stk(pr=win.rt()) as ov:
        overlay = ov
        with win.bx(pd=0.0) as bl:
            blank = bl
            win.cls(bl, "clear")
        for name in names:
            with win.bx(pd=0.0) as scrim:
                win.cls(scrim, "scrim")
                with win.bx(pd=18.0, gp=12.0, w=560.0) as card:
                    win.cls(card, "card")
                    win.pin(card, l=300.0, t=120.0)
                    win.lb(name, h=36.0)
                    win.sep()
                    body(name)
                    win.bt("Закрыть", h=44.0, clk=lambda: page.st(0))
    win.pin(overlay, l=0.0, t=0.0, r=0.0, b=0.0)
    win.ghost(overlay, True)
    win.ghost(blank, True)
    win.bindv(overlay, lambda: float(page()))

    win.css(CSS)
    win.go()


if __name__ == "__main__":
    main()
