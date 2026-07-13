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


def main():
    win = ssui.W("SSUI Showcase", 1060, 760, thm="drk",
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

    with win.bx(pd=18.0, gp=14.0) as scr:
        win.cls(scr, "clear")

        with win.bx(ax="h", gp=12.0, pd=14.0, h=64.0) as head:
            win.align(head, justify="btw", cross="cnt")
            win.lb("● SSUI", w=160.0, h=32.0)
            win.lb(bind=lambda: f"Громкость {int(vol()*100)}%   ·   пробел — тема",
                   h=28.0)
            win.bt("Диалог", w=120.0, h=40.0,
                   clk=lambda: dlg("Привет", "Это модальный диалог SSUI.",
                                   ["Отмена", "Ок"], on=lambda i: None))

        with win.tab(["Виджеты", "Ввод", "Раскладка", "Данные",
                      "Секции", "Панели", "Окно"], h=600.0) as tabs:

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
                with win.bx(pd=16.0, gp=10.0):
                    win.lb("Значения", h=26.0)
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

    fab = win.bt("+", pr=win.rt(), w=54.0, h=54.0,
                 clk=lambda: fx(vol, 1.0, dur=0.5))
    win.pin(fab, r=22.0, b=22.0)

    win.css(CSS)
    win.go()


if __name__ == "__main__":
    main()