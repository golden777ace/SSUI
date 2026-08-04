"""SSUI — единая витрина возможностей.

Пробел — смена темы. ПКМ — контекстное меню. F12 — инспектор.
Вкладка «Окно» — живая регулировка тона и размытия фона.
"""

from pathlib import Path

import ssui

CSS = """
frame { background: #1c2440cc; radius: 16; }
.clear { background: #00000000; }
tabs   { background: #1c2440cc; color: #eef3ff; }
label  { color: #eef3ff; }

.demo { background: #101a33cc; radius: 14; color: #f59e0b; }
.demo > label { color: #eef3ff; }
.demo .hint, .demo .warn { color: #22c55e; }
.demo button:hover { background: #3b82f6; color: #ffffff; }
.demo > .row > button:focus { background: #a855f7; }

.dza { background: #3B82F6; color: #FFFFFF; }
.dzb { background: #2FBF71; color: #FFFFFF; }
.dzc { background: #E5484D; color: #FFFFFF; }
"""
LIVE = str(Path(__file__).with_name("live.css"))

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
                 glass=True, tint=0.15, blur=True, insp=True)
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
    padv = ssui.sgnl(10.0)
    gapv = ssui.sgnl(6.0)
    psecs = ssui.sgnl(3.0)
    psize = ssui.sgnl(16.0)
    pbg = ssui.sgnl(0)
    pcorner = ssui.sgnl(1)
    pcenter = ssui.sgnl(0)
    pushes = ssui.sgnl(0)
    query = ssui.sgnl("")
    online = ssui.sgnl(False)
    items30 = [f"Элемент {i}" for i in range(1, 31)]
    TEAM = [
        ["Анна", "Дизайнер", "Онлайн"],
        ["Борис", "Бэкенд", "Отошёл"],
        ["Вера", "Фронтенд", "Онлайн"],
        ["Глеб", "QA", "Не в сети"],
        ["Дина", "PM", "Онлайн"],
        ["Егор", "DevOps", "Отошёл"],
    ]

    PUSH_BG = [
        ("Тёмный", "#202632", "#eef3ff"),
        ("Успех", "#14532d", "#dcfce7"),
        ("Ошибка", "#7f1d1d", "#fee2e2"),
        ("Внимание", "#78350f", "#fef3c7"),
    ]
    PUSH_FONT = [None, "Consolas", "Georgia", "Segoe UI"]
    CORNERS = [("Слева вверху", "tl"), ("Справа вверху", "tr"),
               ("Слева внизу", "bl"), ("Справа внизу", "br")]
    CENTERS = [("screen", "screen"), ("parent", "parent"), ("нет", False)]

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

    def open_child(mode):
        sub = win.subwin("Дочернее окно", 420, 260, center=mode)
        with sub.bx(pr=sub.rt(), pd=16.0, gp=10.0):
            sub.lb(f"Центрирование: {mode}", h=30.0)
            sub.lb("screen — центр монитора по рабочей области.\n"
                   "parent — центр главного окна.\n"
                   "нет — позиция от системы.",
                   h=90.0, wrap=True)
            sub.bt("Закрыть", h=44.0, clk=sub.close)
        sub.css(CSS)
        sub.show()
        act.st(f"окно: center={mode}")

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

        with win.tab(["Виджеты", "Ввод", "Раскладка", "Данные", "Данные 2",
                      "Выбор", "Док", "Секции", "Панели", "Окно", "CSS",
                      "Глубина"],
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
                    win.tx("", sig=name, ph="Введите имя…", h=44.0)
                    win.lb(bind=lambda: f"Привет, {name() or '…'}", h=26.0)
                    win.sep()
                    win.lb("Ctrl+C/V/Z, выделение мышью", h=24.0)
                with win.bx(pd=16.0, gp=10.0):
                    win.lb("Многострочное поле (Enter — перенос)", h=26.0)
                    win.ta("", sig=note, ph="Текст заметки…", h=220.0)
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

                win.lb("gr — сетка · pk — упаковка · pl — абсолют", h=24.0)
                with win.bx(ax="h", gp=12.0, pd=0.0, h=64.0) as ctl:
                    win.cls(ctl, "clear")
                    win.lb(bind=lambda: f"padding: {int(padv())}", w=150.0, h=28.0)
                    win.sl(0.25, ch=lambda v: padv.st(v * 40.0), h=32.0)
                    win.lb(bind=lambda: f"gap: {int(gapv())}", w=110.0, h=28.0)
                    win.sl(0.2, ch=lambda v: gapv.st(v * 30.0), h=32.0)
                with win.bx(ax="h", gp=12.0, pd=0.0, h=260.0) as lrow:
                    win.cls(lrow, "clear")
                    with win.bx(pd=10.0, gp=6.0) as gbox:
                        for i in range(8):
                            cell = win.bt(f"{i}", clk=lambda: act.st("сетка"))
                            win.gr(cell, i // 3, i % 3)
                        wide = win.bt("cs=2", clk=lambda: act.st("сетка cs=2"))
                        win.gr(wide, 2, 1, cs=2)
                    win.bindb(gbox, lambda: (padv(), gapv()))
                    with win.bx(pd=10.0, gp=6.0) as pbox:
                        head = win.lb("top · fill=x", h=28.0)
                        win.pk(head, "t", fill="x")
                        left = win.bt("left", w=90.0)
                        win.pk(left, "l", fill="y")
                        right = win.bt("right", w=90.0)
                        win.pk(right, "r", fill="y")
                        mid = win.bt("exp", clk=lambda: act.st("упаковка"))
                        win.pk(mid, "t", fill="both", exp=True)
                        foot = win.lb("bottom", h=28.0)
                        win.pk(foot, "b", fill="x")
                    win.bindb(pbox, lambda: (padv(), gapv()))
                    with win.bx(pd=10.0):
                        chip = win.bt("pl 20,20", clk=lambda: act.st("абсолют"))
                        win.pl(chip, x=20.0, y=20.0, w=130.0, h=44.0)
                        mid2 = win.bt("pl центр", clk=lambda: act.st("абсолют"))
                        win.pl(mid2, w=150.0, h=44.0)
                        corner = win.bt("pl угол", clk=lambda: act.st("абсолют"))
                        win.pl(corner, r=16.0, b=16.0, w=120.0, h=44.0)

            # --- Данные ---
            with win.bx(pr=tabs, ax="h", pd=8.0, gp=12.0) as p4:
                win.cls(p4, "clear")
                with win.bx(pd=12.0, gp=8.0, w=280.0):
                    win.lb(bind=lambda: (
                        "Пункт не выбран" if pick() < 0
                        else f"Пункт #{pick()+1}"
                    ), h=26.0)
                    with win.bx(ax="h", gp=8.0, pd=0.0, h=44.0) as frow:
                        win.cls(frow, "clear")
                        win.lb("Фильтр:", w=80.0, h=36.0)
                        win.tx("", sig=query, ph="поиск…", h=40.0)
                    dyn = win.lst([], ch=lambda i: pick.st(i), h=360.0)
                    win.bindl(dyn, lambda: [
                        s for s in items30 if query().lower() in s.lower()
                    ])
                with win.bx(pd=12.0, gp=8.0):
                    win.lb(bind=lambda: (
                        "Строка не выбрана" if row() < 0
                        else f"Выбрана строка #{row()+1}"
                    ), h=26.0)
                    with win.bx(ax="h", gp=8.0, pd=0.0, h=44.0) as trow:
                        win.cls(trow, "clear")
                        win.lb("Онлайн только", w=150.0, h=36.0)
                        win.sw("", on=False, clk=lambda v: online.st(v), h=36.0)
                    team_tbl = win.tbl(
                        ["Имя", "Роль", "Статус"],
                        [],
                        ch=lambda i: row.st(i),
                        hl=1.0,
                        vl=1.0,
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
            # --- Данные 2 ---
            with win.bx(pr=tabs, ax="h", pd=8.0, gp=12.0) as p8:
                win.cls(p8, "clear")
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
                    win.lb("Аккордеон — открыта одна секция (grp)", h=26.0)
                    with win.acc("Общие", open=True, grp=1, h=180.0,
                                 ch=lambda v: act.st(f"Общие: {int(v)}")):
                        win.sw("Автозапуск", h=36.0)
                        win.ch("Показывать подсказки", chk=True, h=32.0)
                        win.sl(0.5, h=36.0)
                    with win.acc("Сеть", grp=1, h=140.0,
                                 ch=lambda v: act.st(f"Сеть: {int(v)}")) as anet:
                        win.tgl("Прокси", h=40.0)
                        win.tx("proxy.local:8080", h=44.0)
                    with win.acc("О программе", grp=1, h=110.0,
                                 ch=lambda v: act.st(f"О программе: {int(v)}")):
                        win.lb("SSUI — GPU-GUI для Windows", h=26.0)
                        win.lnk("github.com/golden777ace/SSUI")
                    win.bt("Открыть «Сеть» из кода", h=40.0,
                           clk=lambda: win.acc_open(anet, True))
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

                    win.sep()
                    win.lb("Уведомления по углам окна", h=26.0)
                    win.dd([c[0] for c in CORNERS], sel=1, h=44.0,
                           ch=lambda i: pcorner.st(i))
                    with win.bx(ax="h", gp=8.0, h=48.0) as nrow:
                        win.cls(nrow, "clear")
                        win.bt("Уведомление", h=40.0,
                               clk=lambda: nt("Обмен", "Прочитано 24 регистра",
                                              secs=4.0,
                                              corner=CORNERS[pcorner()][1]))
                        win.bt("Снэкбар", h=40.0,
                               clk=lambda: nt.snack("Записано",
                                                    corner=CORNERS[pcorner()][1]))
                        win.bt("Снэкбар внизу", h=40.0,
                               clk=lambda: nt.snack("По центру внизу"))

                with win.bx(pd=16.0, gp=10.0):
                    win.lb("push — поверх всех окон", h=26.0)
                    win.lb("Отдельное безрамочное окно по центру "
                           "монитора. Гаснет по клику и по времени.",
                           h=52.0, wrap=True)

                    win.lb("Оформление:", h=24.0)
                    win.dd([p[0] for p in PUSH_BG], h=44.0,
                           ch=lambda i: pbg.st(i))
                    win.lb("Шрифт:", h=24.0)
                    pfont = ssui.sgnl(0)
                    win.dd(["как в теме", "Consolas", "Georgia", "Segoe UI"],
                           h=44.0, ch=lambda i: pfont.st(i))

                    win.lb(bind=lambda: f"Размер шрифта: {psize():.0f}",
                           h=24.0)
                    win.sl(0.3, h=34.0,
                           ch=lambda v: psize.st(11.0 + v * 17.0))
                    win.lb(bind=lambda: f"Время показа: {psecs():.1f} с",
                           h=24.0)
                    win.sl(0.3, h=34.0,
                           ch=lambda v: psecs.st(v * 10.0))

                    def fire_push(x=None, y=None):
                        name, bg, fg = PUSH_BG[pbg()]
                        pushes.st(pushes() + 1)
                        win.push(
                            name,
                            f"Сообщение №{pushes()}.\n"
                            "Клик закрывает окно досрочно.",
                            secs=psecs(), bg=bg, fg=fg,
                            font=PUSH_FONT[pfont()], size=psize(), x=x, y=y,
                            on_close=lambda: act.st("push закрыт"),
                        )

                    with win.bx(ax="h", gp=8.0, h=48.0) as prow2:
                        win.cls(prow2, "clear")
                        win.bt("По центру", h=40.0, clk=fire_push)
                        win.bt("В угол 40·40", h=40.0,
                               clk=lambda: fire_push(40, 40))
                        win.bt("Без таймера", h=40.0,
                               clk=lambda: (psecs.st(0.0), fire_push()))
                    win.lb(bind=lambda: f"Показано push: {pushes()}", h=24.0)

                    win.sep()
                    win.lb("Центрирование дочернего окна", h=26.0)
                    win.dd([c[0] for c in CENTERS], h=44.0,
                           ch=lambda i: pcenter.st(i))
                    win.bt("Открыть окно", h=44.0,
                           clk=lambda: open_child(CENTERS[pcenter()][1]))

        # --- CSS ---
        with win.bx(pr=tabs, ax="h", pd=8.0, gp=12.0) as p6:
            win.cls(p6, "clear")
            with win.bx(pd=14.0, gp=8.0) as demo:
                win.cls(demo, "demo")
                win.lb("Каскад: комбинаторы и специфичность", h=26.0)
                win.lb("Прямой потомок: .demo > label", h=24.0)
                with win.bx(pd=0.0, gp=6.0) as nest:
                    win.cls(nest, "clear")
                    h1 = win.lb("Класс сильнее типа: .demo .hint", h=24.0)
                    win.cls(h1, "hint")
                    w1 = win.lb("Группа селекторов через запятую", h=24.0)
                    win.cls(w1, "warn")
                    win.ch("Наследование color от .demo", chk=True, h=30.0)
                    win.lnk("Ссылка тоже наследует цвет",
                            clk=lambda: act.st("css-ссылка"))
                with win.bx(ax="h", gp=8.0, h=52.0) as crow:
                    win.cls(crow, "row")
                    win.bt("hover", h=44.0, clk=lambda: act.st("hover-кнопка"))
                    win.bt("focus", h=44.0, clk=lambda: act.st("focus-кнопка"))
                    win.bt("сброс", h=44.0, clk=lambda: act.st("—"))
            with win.bx(pd=14.0, gp=8.0) as hot:
                win.cls(hot, "demo")
                win.lb("Горячая перезагрузка", h=26.0)
                win.lb(f"Файл: {LIVE}", h=24.0)
                win.lb("Сохрани файл — стили применятся сразу.", h=24.0)
                win.sep()
                with win.bx(pd=12.0, gp=8.0) as card:
                    win.cls(card, "live")
                    win.lb("Живая карточка", h=26.0)
                    win.bt("Кнопка", h=44.0, clk=lambda: act.st("live-кнопка"))
                    win.sl(0.5, ch=lambda v: lvl.st(v), h=36.0)

        # --- Глубина ---
        with win.bx(pr=tabs, pd=8.0, gp=12.0) as pdepth:
            win.cls(pdepth, "clear")
            win.lb("Клик по панели поднимает её наверх (front).", h=28.0)
            with win.bx(rad=12.0, w=760.0, h=420.0):
                dz_a = win.bt("Панель A", toast="Наверху: A")
                dz_b = win.bt("Панель B", toast="Наверху: B")
                dz_c = win.bt("Панель C", toast="Наверху: C")
                win.pl(dz_a, x=60.0, y=50.0, w=280.0, h=200.0)
                win.pl(dz_b, x=220.0, y=120.0, w=280.0, h=200.0)
                win.pl(dz_c, x=380.0, y=190.0, w=280.0, h=200.0)
                win.dep(dz_a, 0)
                win.dep(dz_b, 1)
                win.dep(dz_c, 2)
                for _p in (dz_a, dz_b, dz_c):
                    win.front(_p)
                win.cls(dz_a, "dza")
                win.cls(dz_b, "dzb")
                win.cls(dz_c, "dzc")

    bar = win.stb(pr=win.rt(), h=32.0, bind=lambda: (
        f"{act()} · громкость {int(vol() * 100)}% · "
        f"диапазон {int(rlo() * 100)}–{int(rhi() * 100)}%"
    ))
    win.pin(bar, l=0.0, r=0.0, b=0.0)

    fab = win.bt("+", pr=win.rt(), w=54.0, h=54.0,
                 clk=lambda: fx(vol, 1.0, dur=0.5))
    win.pin(fab, r=22.0, b=22.0)

    win.bindt(team_tbl, lambda: [
        r for r in TEAM if not online() or r[2] == "Онлайн"
    ])
    win.css(CSS)
    win.css_hot(LIVE)
    win.go()


if __name__ == "__main__":
    main()