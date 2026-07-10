"""SSUI — единая витрина возможностей.

Пробел — смена темы. ПКМ — контекстное меню.
Вкладка «Окно» — живая регулировка тона и размытия фона.
Все поверхности прозрачны/матовы; карточки плавают над стеклом.
"""

import ssui

CSS = """
frame { background: #1c2440cc; radius: 16; }
.clear { background: #00000000; }
tabs   { background: #1c2440cc; color: #eef3ff; }
label  { color: #eef3ff; }
"""


def main():
    win = ssui.W("SSUI Showcase", 980, 720, thm="drk",
                 glass=True, tint=0.15, blur=True)
    fx = win.fx()
    dlg = win.dlg()

    vol = ssui.sgnl(0.35)
    name = ssui.sgnl("")
    wifi = ssui.sgnl(True)
    plan = ssui.sgnl("Free")
    row = ssui.sgnl(-1)
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

        with win.tab(["Виджеты", "Раскладка", "Данные", "Окно"], h=560.0) as tabs:

            with win.bx(pr=tabs, ax="h", gp=16.0, pd=8.0) as p1:
                win.cls(p1, "clear")
                with win.bx(pd=16.0, gp=12.0):
                    win.lb("Управление", h=26.0)
                    win.bt("Кнопка", h=44.0, clk=lambda: fx(vol, 1.0, dur=0.5))
                    win.ch("Флажок", chk=True, h=32.0)
                    win.sw("Wi-Fi", on=True, clk=lambda v: wifi.st(v), h=36.0)
                    win.lb(bind=lambda: f"Wi-Fi: {'вкл' if wifi() else 'выкл'}",
                           h=24.0)
                with win.bx(pd=16.0, gp=12.0):
                    win.lb("Выбор", h=26.0)
                    win.rd("Free", grp=1, on=True, clk=lambda: plan.st("Free"),
                           h=30.0)
                    win.rd("Pro", grp=1, clk=lambda: plan.st("Pro"), h=30.0)
                    win.rd("Team", grp=1, clk=lambda: plan.st("Team"), h=30.0)
                    win.lb(bind=lambda: f"План: {plan()}", h=24.0)
                    win.dd(["Низко", "Средне", "Высоко"], sel=1, h=44.0)
                with win.bx(pd=16.0, gp=12.0):
                    win.lb("Значения", h=26.0)
                    win.lb(bind=lambda: f"Громкость: {int(vol()*100)}%", h=24.0)
                    win.pr(bind=lambda: vol(), h=18.0)
                    win.sl(vol(), ch=lambda v: vol.st(v), h=36.0)
                    win.tx("", sig=name, h=44.0)
                    win.lb(bind=lambda: f"Привет, {name() or '…'}", h=24.0)

            with win.bx(pr=tabs, pd=8.0, gp=14.0) as p2:
                win.cls(p2, "clear")
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

            with win.bx(pr=tabs, pd=8.0, gp=14.0) as p3:
                win.cls(p3, "clear")
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
                    h=360.0,
                )

            with win.bx(pr=tabs, pd=8.0, gp=16.0) as p4:
                win.cls(p4, "clear")
                with win.bx(pd=16.0, gp=12.0):
                    win.lb("Прозрачность фона окна", h=26.0)
                    win.lb(bind=lambda: f"Тон: {int(op()*100)}%", h=24.0)
                    win.sl(op(), ch=lambda v: op.st(v), h=36.0)
                    win.lb(bind=lambda: f"Размытие: {int(blurv()*100)}%", h=24.0)
                    win.sl(blurv(), ch=lambda v: blurv.st(v), h=36.0)
                with win.bx(pd=16.0, gp=12.0):
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

    win.css(CSS)  # ВАЖНО: css применяется к уже построенному дереву
    win.go()


if __name__ == "__main__":
    main()