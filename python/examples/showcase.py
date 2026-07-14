"""SSUI — единая витрина возможностей.

Пробел — смена темы. ПКМ — контекстное меню. F12 — инспектор.
Вкладка «Окно» — живая регулировка тона и размытия фона.
"""

import ssui

CSS = """
frame { background: #1c2440cc; radius: 16; }
.clear { background: #00000000; }
tabs   { background: #1c2440cc; color: #eef3ff; }
label  { color: #eef3ff; }
"""

DATA = [10.2, 1.1, 1.0, 0.9, 0.8, 0.7, 0.6]

TREE = [
    (0, "Проект", False),
    (1, "core", False),
    (2, "device.rs", True),
    (2, "canvas.rs", True),
    (1, "python", False),
    (2, "lib.rs", True),
    (2, "showcase.py", True),
    (1, "README.md", True),
]


def main():
    win = ssui.W("SSUI Showcase", 1600, 1000, thm="drk",
                 glass=True, tint=0.15, blur=True)
    fx = win.fx()
    dlg = win.dlg()

    vol = ssui.sgnl(0.35)
    name = ssui.sgnl("")
    note = ssui.sgnl("")
    qty = ssui.sgnl(3)
    mono = ssui.sgnl(False)
    wifi = ssui.sgnl(True)
    plan = ssui.sgnl("Free")
    row = ssui.sgnl(-1)
    pick = ssui.sgnl(-1)
    clicks = ssui.sgnl(0)
    page = ssui.sgnl(0)
    op = ssui.sgnl(0.15)
    blurv = ssui.sgnl(0.5)
    lvl = ssui.sgnl(0.6)
    load = ssui.sgnl(0.45)
    rlo = ssui.sgnl(0.25)
    rhi = ssui.sgnl(0.75)
    act = ssui.sgnl("—")
    knob = ssui.sgnl(0.4)
    leaf = ssui.sgnl(-1)
    day = ssui.sgnl("—")
    hexc = ssui.sgnl("#3B82F6")
    tmv = ssui.sgnl("12:00")
    prow = ssui.sgnl(-1)
    path = ssui.sgnl("Проект")
    inbox = ssui.sgnl(3)
    pageno = ssui.sgnl(0)
    stars = ssui.sgnl(4)
    cvx = ssui.sgnl(0.5)
    cvr = ssui.sgnl(0.4)
    cmds = ssui.sgnl(0)
    files = ssui.sgnl("файлов нет")

    def run_cmd(cmd):
        c = cmd.strip()
        if c == "help":
            return "help — список команд\nver — версия\necho <текст>\nclear — очистка"
        if c == "ver":
            return "SSUI 0.14 · Rust + Direct2D"
        if c.startswith("echo "):
            return c[5:]
        if not c:
            return ""
        return f"неизвестная команда: {c}"

    def scene(cx, rr):
        px = 20.0 + cx * 320.0
        rad = 20.0 + rr * 60.0
        return [
            ("rect", [16.0, 16.0, 360.0, 200.0, 12.0, 2.0], "#4B5563", ""),
            ("line", [16.0, 216.0, 376.0, 216.0, 3.0], "#3B82F6", ""),
            ("circle", [px, 120.0, rad, 0.0], "#22C55E", ""),
            ("circle", [px, 120.0, rad + 8.0, 2.0], "#EEF3FF", ""),
            ("text", [20.0, 230.0, 360.0, 28.0], "#EEF3FF",
             f"x={int(cx * 100)}%  r={int(rad)}px"),
        ]

    win.tint(op)
    win.blur(blurv)
    win.menu(
        ["Обновить", "Сбросить громкость", "О программе"],
        on_select=lambda i: (
            fx(vol, 0.0, dur=0.4) if i == 1
            else dlg("SSUI", "GPU-GUI на Rust + Python.", ["Ок"]) if i == 2
            else None
        ),
    )

    nt = win.nt()

    with win.bx(pd=18.0, gp=14.0) as scr:
        win.cls(scr, "clear")

        win.mb([("Файл", ["Открыть", "Сохранить", "Выход"]),
                ("Правка", ["Отменить", "Повторить"]),
                ("Справка", ["О программе"])],
               on_select=lambda m, i: act.st(f"меню {m}·{i}"), h=40.0)

        with win.bx(ax="h", gp=12.0, pd=14.0, h=64.0) as head:
            win.align(head, justify="btw", cross="cnt")
            win.lb("● SSUI", w=160.0, h=32.0)
            win.lb(bind=lambda: f"Громкость {int(vol()*100)}%   ·   пробел — тема",
                   h=28.0)
            win.bt("Диалог", w=120.0, h=40.0,
                   tip="Открыть модальное окно",
                   clk=lambda: dlg("Привет", "Это модальный диалог SSUI.",
                                   ["Отмена", "Ок"], on=lambda i: None))
            win.bt("Toast", w=110.0, h=40.0,
                   tip="Показать уведомление",
                   toast="Настройки сохранены ✓")
            win.bt("MsgBox", w=120.0, h=40.0,
                   clk=lambda: dlg.msg("Готово", "Файл сохранён на диск."))
            win.bt("Alert", w=110.0, h=40.0,
                   clk=lambda: dlg.alert("Не удалось открыть файл."))
            win.bt("Notify", w=120.0, h=40.0,
                   clk=lambda: nt("Обновление", "Доступна версия 0.14",
                                  action="Позже", secs=6.0,
                                  on=lambda: act.st("уведомление закрыто")))
            win.bt("Snack", w=110.0, h=40.0,
                   clk=lambda: nt.snack("Элемент удалён", action="Отменить",
                                        on=lambda: act.st("отменено")))

        with win.tab(["Виджеты", "Ввод", "Раскладка", "Данные",
                      "Выбор", "Док", "Секции", "Панели", "Окно"],
                     h=600.0) as tabs:

            # --- Виджеты ---
            with win.bx(pr=tabs, ax="h", gp=16.0, pd=8.0) as p1:
                win.cls(p1, "clear")
                with win.bx(pd=16.0, gp=10.0):
                    with win.grp("Переключатели", gp=8.0):
                        win.ch("Флажок", chk=True, h=32.0)
                        win.sw("Wi-Fi", on=True, clk=lambda v: wifi.st(v), h=36.0)
                        win.lb(bind=lambda: f"Wi-Fi: {'вкл' if wifi() else 'выкл'}",
                               h=24.0)
                        win.sep()
                        win.tgl("Моно-режим", clk=lambda v: mono.st(v), h=40.0)
                        win.lb(bind=lambda: f"Моно: {'да' if mono() else 'нет'}",
                               h=24.0)
                    with win.grp("Ссылки", gp=6.0, h=120.0):
                        win.lnk("Открыть документацию",
                                clk=lambda: clicks.st(clicks() + 1))
                        win.lnk("Сообщить об ошибке",
                                clk=lambda: clicks.st(clicks() + 1))
                        win.lb(bind=lambda: f"Кликов по ссылкам: {clicks()}",
                               h=24.0)
                with win.bx(pd=16.0, gp=10.0):
                    win.lb("Выбор", h=26.0)
                    win.rd("Free", grp=1, on=True, clk=lambda: plan.st("Free"),
                           h=30.0)
                    win.rd("Pro", grp=1, clk=lambda: plan.st("Pro"), h=30.0)
                    win.rd("Team", grp=1, clk=lambda: plan.st("Team"), h=30.0)
                    win.lb(bind=lambda: f"План: {plan()}", h=24.0)
                    win.sep()
                    win.dd(["Низко", "Средне", "Высоко"], sel=1, h=44.0)
                    win.sep()
                    win.sbt("Сохранить",
                            ["Сохранить как…", "Экспорт", "Печать"],
                            clk=lambda: act.st("сохранено"),
                            ch=lambda i: act.st(f"пункт {i}"), h=44.0)
                    win.lb(bind=lambda: f"Действие: {act()}", h=24.0)
                with win.bx(pd=16.0, gp=10.0):
                    win.lb("Значения", h=26.0)
                    win.gg(bind=lambda: vol(), lb="VOL", h=130.0)
                    win.spn(h=44.0)
                    win.lb(bind=lambda: f"Громкость: {int(vol()*100)}%", h=24.0)
                    win.pr(bind=lambda: vol(), h=18.0)
                    win.sl(vol(), ch=lambda v: vol.st(v), h=36.0)
                    win.sep()
                    win.lb(bind=lambda: f"Количество: {qty()}", h=24.0)
                    win.spin(3, min=0, max=10, step=1,
                             ch=lambda v: qty.st(int(v)), h=44.0)

            # --- Ввод ---
            with win.bx(pr=tabs, ax="h", gp=16.0, pd=8.0) as p2:
                win.cls(p2, "clear")
                with win.bx(pd=16.0, gp=10.0):
                    win.lb("Однострочное поле", h=26.0)
                    win.tx("", sig=name, h=44.0)
                    win.lb(bind=lambda: f"Привет, {name() or '…'}", h=26.0)
                    win.sep()
                    win.lb("Ctrl+C/V/Z, выделение мышью", h=24.0)
                with win.bx(pd=16.0, gp=10.0):
                    win.lb("Многострочное поле (Enter — перенос)", h=26.0)
                    win.ta("", sig=note, h=220.0)
                    win.lb(bind=lambda: f"Символов: {len(note())}", h=26.0)
                    win.sep()
                    win.lb("Диапазон (два ползунка)", h=26.0)
                    win.rsl(rlo(), rhi(),
                            ch=lambda a, b: (rlo.st(a), rhi.st(b)), h=36.0)
                    win.lb(bind=lambda: (
                        f"Диапазон: {int(rlo()*100)}–{int(rhi()*100)}%"
                    ), h=26.0)

            # --- Раскладка ---
            with win.bx(pr=tabs, pd=8.0, gp=12.0) as p3:
                win.cls(p3, "clear")
                win.lb("justify: st / cnt / end / btw", h=24.0)
                for j in ["st", "cnt", "end", "btw"]:
                    with win.bx(ax="h", gp=8.0, pd=8.0, h=54.0) as r:
                        win.align(r, justify=j, cross="cnt")
                        for c in ["A", "B", "C"]:
                            win.bt(c, w=68.0, h=38.0)
                win.lb("grow: веса 1 / 2 / 1", h=24.0)
                with win.bx(ax="h", gp=8.0, pd=8.0, h=54.0):
                    a = win.bt("1", h=38.0)
                    b = win.bt("2", h=38.0)
                    c = win.bt("1", h=38.0)
                    win.grow(a, 1.0)
                    win.grow(b, 2.0)
                    win.grow(c, 1.0)

            # --- Данные ---
            with win.bx(pr=tabs, ax="h", pd=8.0, gp=12.0) as p4:
                win.cls(p4, "clear")
                with win.bx(pd=12.0, gp=8.0, w=280.0):
                    win.lb(bind=lambda: (
                        "Пункт не выбран" if pick() < 0
                        else f"Пункт #{pick()+1}"
                    ), h=26.0)
                    win.lst([f"Элемент {i}" for i in range(1, 31)],
                            ch=lambda i: pick.st(i), h=400.0)
                with win.bx(pd=12.0, gp=8.0):
                    win.lb(bind=lambda: (
                        "Строка не выбрана" if row() < 0
                        else f"Выбрана строка #{row()+1}"
                    ), h=26.0)
                    win.tbl(
                        ["Имя", "Роль", "Статус"],
                        [
                            ["Анна", "Дизайнер", "Онлайн"],
                            ["Борис", "Бэкенд", "Отошёл"],
                            ["Вера", "Фронтенд", "Онлайн"],
                            ["Глеб", "QA", "Не в сети"],
                            ["Дина", "PM", "Онлайн"],
                            ["Егор", "DevOps", "Отошёл"],
                        ],
                        ch=lambda i: row.st(i),
                        h=400.0,
                    )
                with win.bx(pd=12.0, gp=8.0, w=280.0):
                    with win.bx(ax="h", gp=8.0, h=34.0) as brow:
                        win.cls(brow, "clear")
                        win.lb("Входящие", h=34.0)
                        win.bdg(bind=lambda: str(inbox()), h=26.0)
                        win.bdg(dot=True, h=26.0)
                    with win.bx(ax="h", gp=8.0, h=48.0) as bcnt:
                        win.cls(bcnt, "clear")
                        win.bt("+1", h=40.0,
                               clk=lambda: inbox.st(inbox() + 1))
                        win.bt("Сброс", h=40.0, clk=lambda: inbox.st(0))
                    crumbs = win.crumb(
                        ["Проект", "core", "render", "device.rs"],
                        ch=lambda i: path.st(f"уровень {i}"), h=34.0)
                    win.lb(bind=lambda: f"Путь: {path()}", h=24.0)
                    win.sep()
                    win.lb("Страницы", h=26.0)
                    win.pgn(5, page=0, ch=lambda i: pageno.st(i), h=44.0)
                    win.lb(bind=lambda: f"Страница: {pageno() + 1} из 5",
                           h=24.0)
                    win.lb("Оценка", h=26.0)
                    win.rat(4, max=5, ch=lambda v: stars.st(v), h=40.0)
                    win.lb(bind=lambda: f"Оценка: {stars()}/5", h=24.0)
                    win.sep()
                    win.lb(bind=lambda: (
                        "Узел не выбран" if leaf() < 0
                        else f"Узел #{leaf()}"
                    ), h=26.0)
                    win.tre(TREE, ch=lambda i: leaf.st(i), h=260.0)
                    win.sep()
                    win.lb("Регулятор (тянуть вверх)", h=26.0)
                    win.dl(knob(), lb="MIX", ch=lambda v: knob.st(v), h=140.0)
                    win.lb(bind=lambda: f"Микс: {int(knob() * 100)}%", h=24.0)
                    win.sl(knob(), ch=lambda v: knob.st(v), h=36.0)
                with win.bx(pd=12.0, gp=8.0, w=420.0):
                    win.lb("Терминал (Enter — выполнить)", h=26.0)
                    tm_out = win.term(["SSUI shell. Введите help."],
                                      prompt="$",
                                      on=lambda c: (cmds.st(cmds() + 1),
                                                    run_cmd(c))[1],
                                      h=320.0)
                    win.lb(bind=lambda: f"Команд выполнено: {cmds()}", h=24.0)
                with win.bx(pd=12.0, gp=8.0, w=400.0):
                    win.lb("Область рисования", h=26.0)
                    win.cv(bind=lambda: scene(cvx(), cvr()), h=280.0)
                    win.lb("Позиция", h=24.0)
                    win.sl(cvx(), ch=lambda v: cvx.st(v), h=36.0)
                    win.lb("Радиус", h=24.0)
                    win.sl(cvr(), ch=lambda v: cvr.st(v), h=36.0)
                    with win.bx(ax="h", gp=8.0, h=48.0) as cvrow:
                        win.cls(cvrow, "clear")
                        win.bt("Влево", h=40.0,
                               clk=lambda: fx(cvx, 0.0, dur=0.5))
                        win.bt("Вправо", h=40.0,
                               clk=lambda: fx(cvx, 1.0, dur=0.5))
                with win.bx(pd=12.0, gp=8.0, w=320.0):
                    win.lb("Диаграмма", h=26.0)
                    win.cht(DATA, bind=lambda: [v * lvl() for v in DATA],
                            h=200.0)
                    win.lb(bind=lambda: f"Масштаб: {int(lvl() * 100)}%", h=24.0)
                    win.sl(lvl(), ch=lambda v: lvl.st(v), h=36.0)
                    with win.bx(ax="h", gp=8.0, h=48.0) as crow:
                        win.cls(crow, "clear")
                        win.bt("30%", h=40.0, clk=lambda: fx(lvl, 0.3, dur=0.4))
                        win.bt("100%", h=40.0, clk=lambda: fx(lvl, 1.0, dur=0.4))
                    win.sep()
                    win.lb("Шкала-метр", h=26.0)
                    win.mt(load(), bind=lambda: load(), seg=10, h=28.0)
                    win.mt(load(), bind=lambda: load(), seg=20, h=22.0)
                    win.lb(bind=lambda: f"Нагрузка: {int(load() * 100)}%", h=24.0)
                    win.sl(load(), ch=lambda v: load.st(v), h=36.0)

            # --- Выбор ---
            with win.bx(pr=tabs, ax="h", pd=8.0, gp=12.0) as p6:
                win.cls(p6, "clear")
                with win.bx(pd=12.0, gp=8.0, w=380.0):
                    win.lb("Календарь", h=26.0)
                    win.cal(2026, 7, 13,
                            ch=lambda y, m, d: day.st(f"{d:02d}.{m:02d}.{y}"),
                            h=320.0)
                    win.lb(bind=lambda: f"Дата: {day()}", h=26.0)
                with win.bx(pd=12.0, gp=8.0):
                    win.lb("Палитра", h=26.0)
                    win.clr(ch=lambda c: hexc.st(c), h=240.0)
                    win.lb(bind=lambda: f"Цвет: {hexc()}", h=26.0)
                    win.sep()
                    win.lb("Время (клик по стрелкам)", h=26.0)
                    win.tm(12, 0, ch=lambda h, m: tmv.st(f"{h:02d}:{m:02d}"),
                           h=120.0)
                    win.lb(bind=lambda: f"Время: {tmv()}", h=26.0)
                with win.bx(pd=12.0, gp=8.0, w=340.0):
                    win.lb("Свойства", h=26.0)
                    win.pg([("—", "—")], bind=lambda: [
                        ("Тема", "по пробелу"),
                        ("Громкость", f"{int(vol()*100)}%"),
                        ("Диапазон", f"{int(rlo()*100)}–{int(rhi()*100)}%"),
                        ("Микс", f"{int(knob()*100)}%"),
                        ("Дата", day()),
                        ("Время", tmv()),
                        ("Цвет", hexc()),
                        ("Действие", act()),
                    ], ch=lambda i: prow.st(i), h=300.0)
                    win.lb(bind=lambda: (
                        "Строка не выбрана" if prow() < 0
                        else f"Строка #{prow()}"
                    ), h=26.0)

            # --- Док ---
            with win.bx(pr=tabs, ax="h", pd=8.0, gp=12.0) as p7:
                win.cls(p7, "clear")
                with win.dock("Инструменты", side="l", size=260.0, gp=8.0):
                    win.lb("Панель слева", h=26.0)
                    win.bt("Действие", h=40.0,
                           clk=lambda: act.st("док-кнопка"))
                    win.sl(vol(), ch=lambda v: vol.st(v), h=36.0)
                    win.lb("Клик по шапке — свернуть", h=24.0)
                with win.bx(pd=12.0, gp=10.0):
                    win.lb("Приём файлов", h=26.0)
                    win.drop("Перетащите файлы сюда",
                             on=lambda ps: files.st(
                                 f"{len(ps)} шт · {ps[0].split(chr(92))[-1]}"),
                             h=180.0)
                    win.lb(bind=lambda: f"Принято: {files()}", h=26.0)
                    win.bt("Сброс", h=40.0,
                           clk=lambda: files.st("файлов нет"))

            # --- Секции ---
            with win.bx(pr=tabs, ax="h", pd=8.0, gp=16.0) as pacc:
                win.cls(pacc, "clear")
                with win.bx(pd=12.0, gp=8.0, w=420.0):
                    win.lb("Аккордеон — клик по заголовку", h=26.0)
                    with win.acc("Общие", open=True, h=180.0):
                        win.sw("Автозапуск", h=36.0)
                        win.ch("Показывать подсказки", chk=True, h=32.0)
                        win.sl(0.5, h=36.0)
                    with win.acc("Сеть", h=140.0):
                        win.tgl("Прокси", h=40.0)
                        win.tx("proxy.local:8080", h=44.0)
                    with win.acc("О программе", h=110.0):
                        win.lb("SSUI — GPU-GUI для Windows", h=26.0)
                        win.lnk("github.com/golden777ace/SSUI")
                with win.bx(pd=12.0, gp=8.0):
                    win.lb("Область прокрутки — колесо мыши", h=26.0)
                    with win.scr(h=460.0):
                        for i in range(1, 26):
                            win.bt(f"Кнопка {i}", h=44.0,
                                   clk=lambda i=i: clicks.st(i))
                    win.lb(bind=lambda: f"Последняя: {clicks()}", h=26.0)

            # --- Панели ---
            with win.bx(pr=tabs, pd=8.0, gp=12.0) as pstk:
                win.cls(pstk, "clear")

                with win.bx(ax="h", gp=8.0, h=52.0) as nav:
                    win.cls(nav, "clear")
                    win.bt("Страница 1", h=44.0, clk=lambda: page.st(0))
                    win.bt("Страница 2", h=44.0, clk=lambda: page.st(1))
                    win.bt("Страница 3", h=44.0, clk=lambda: page.st(2))

                with win.stk(bind=lambda: float(page()), h=200.0) as pages:
                    with win.bx(pd=16.0, gp=8.0):
                        win.lb("Первая страница стопки", h=28.0)
                        win.sl(0.3, h=36.0)
                    with win.bx(pd=16.0, gp=8.0):
                        win.lb("Вторая страница стопки", h=28.0)
                        win.tx("Ввод на странице 2", h=44.0)
                    with win.bx(pd=16.0, gp=8.0):
                        win.lb("Третья страница стопки", h=28.0)
                        win.sw("Опция", on=True, h=36.0)

                win.lb(bind=lambda: f"Splitter · страница {page() + 1}", h=26.0)
                with win.spl(ratio=0.4, h=260.0):
                    with win.bx(pd=12.0, gp=8.0):
                        win.lb("Левая область", h=26.0)
                        win.lst([f"Файл {i}.txt" for i in range(1, 15)], h=180.0)
                    with win.bx(pd=12.0, gp=8.0):
                        win.lb("Правая область", h=26.0)
                        win.ta("Содержимое…", h=180.0)

            # --- Окно ---
            with win.bx(pr=tabs, pd=8.0, gp=16.0) as p5:
                win.cls(p5, "clear")
                with win.bx(pd=16.0, gp=10.0):
                    win.lb("Прозрачность фона окна", h=26.0)
                    win.lb(bind=lambda: f"Тон: {int(op()*100)}%", h=24.0)
                    win.sl(op(), ch=lambda v: op.st(v), h=36.0)
                    win.sep()
                    win.lb(bind=lambda: f"Размытие: {int(blurv()*100)}%", h=24.0)
                    win.sl(blurv(), ch=lambda v: blurv.st(v), h=36.0)
                with win.bx(pd=16.0, gp=10.0):
                    win.lb("Анимация громкости", h=26.0)
                    win.pr(bind=lambda: vol(), h=18.0)
                    with win.bx(ax="h", gp=8.0, h=48.0) as arow:
                        win.cls(arow, "clear")
                        win.bt("0%", h=40.0, clk=lambda: fx(vol, 0.0, dur=0.4))
                        win.bt("50%", h=40.0, clk=lambda: fx(vol, 0.5, dur=0.4))
                        win.bt("100%", h=40.0, clk=lambda: fx(vol, 1.0, dur=0.4))

    bar = win.stb(pr=win.rt(), h=32.0, bind=lambda: (
        f"{act()} · громкость {int(vol() * 100)}% · "
        f"диапазон {int(rlo() * 100)}–{int(rhi() * 100)}%"
    ))
    win.pin(bar, l=0.0, r=0.0, b=0.0)

    fab = win.bt("+", pr=win.rt(), w=54.0, h=54.0,
                 clk=lambda: fx(vol, 1.0, dur=0.5))
    win.pin(fab, r=22.0, b=22.0)

    win.css(CSS)
    win.go()


if __name__ == "__main__":
    main()