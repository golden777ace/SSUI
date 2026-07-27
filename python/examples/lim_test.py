"""SSUI — пределы размеров узлов и spl_ratio.

Демонстрирует:
  • win.lim(node, min_w=..., max_w=...) — узел не сжать и не растянуть;
  • пределы областей разделителя — рукоятка упирается в них;
  • win.spl_ratio(node, r) — положение разделителя из кода;
  • min_h у строк списка и таблицы внутри scr.

Все пределы правятся ползунками на лету.
"""

import ssui

CSS = """
frame { background: #141c30cc; radius: 14; }
.clear { background: #00000000; }
label { color: #eef3ff; }
.head { color: #3b82f6; }
.dim  { color: #94a3b8; }
.card { background: #1b2540; radius: 10; }
.pane { background: #16203a; radius: 10; }
"""

COLS = ["Параметр", "Значение", "Ед."]
ROWS = [
    ["Напряжение A", "231.4", "В"],
    ["Напряжение B", "229.8", "В"],
    ["Напряжение C", "230.1", "В"],
    ["Ток A", "5.42", "А"],
    ["Ток B", "5.38", "А"],
    ["Частота", "50.01", "Гц"],
    ["Коэф. мощности", "0.97", "—"],
]


def main():
    win = ssui.W("SSUI · пределы размеров", 1320, 840, thm="drk",
                 glass=True, tint=0.12)

    lo_left = ssui.sgnl(220.0)
    lo_right = ssui.sgnl(260.0)
    ratio = ssui.sgnl(0.5)
    btn_min = ssui.sgnl(140.0)
    btn_max = ssui.sgnl(240.0)
    row_min = ssui.sgnl(24.0)
    row_max = ssui.sgnl(100.0)
    log = ssui.sgnl("—")

    with win.bx(pr=win.rt(), ax="h", pd=14.0, gp=14.0) as root:
        win.cls(root, "clear")

        with win.bx(pd=12.0, gp=10.0) as col:
            t = win.lb("Разделитель с пределами половин", h=28.0)
            win.cls(t, "head")

            with win.spl(ratio=0.5, h=300.0) as sp:
                with win.bx(pd=10.0, gp=6.0) as left:
                    win.cls(left, "pane")
                    win.lb("Левая область", h=28.0)
                    win.lb("Не уже своего min_w", h=26.0)
                    win.bt("Кнопка слева", h=40.0,
                           clk=lambda: log.st("клик слева"))
                with win.bx(pd=10.0, gp=6.0) as right:
                    win.cls(right, "pane")
                    win.lb("Правая область", h=28.0)
                    win.lb("Тоже со своим min_w", h=26.0)
                    win.bt("Кнопка справа", h=40.0,
                           clk=lambda: log.st("клик справа"))

            win.lim(left, min_w=lo_left())
            win.lim(right, min_w=lo_right())

            win.sep()
            t2 = win.lb("Кнопка между min_w и max_w", h=28.0)
            win.cls(t2, "head")
            with win.bx(ax="h", gp=8.0, h=64.0) as brow:
                win.cls(brow, "clear")
                b1 = win.bt("Ограниченная", h=48.0,
                            clk=lambda: log.st("ограниченная"))
                b2 = win.bt("Свободная", h=48.0,
                            clk=lambda: log.st("свободная"))
                b3 = win.bt("Свободная", h=48.0,
                            clk=lambda: log.st("свободная"))
            win.lim(b1, min_w=btn_min(), max_w=btn_max())

            win.sep()
            t3 = win.lb("Таблица со своим минимумом", h=28.0)
            win.cls(t3, "head")
            with win.bx(ax="h", gp=8.0, h=280.0) as trow:
                win.cls(trow, "clear")
                tbl = win.tbl(COLS, [list(r) for r in ROWS],
                              hl=1.0, vl=1.0, h=260.0,
                              ch=lambda i: log.st(f"строка {i}"))
                win.lim(tbl, min_w=420.0)
                with win.bx(pd=10.0, gp=6.0) as side:
                    win.cls(side, "card")
                    win.lb("Соседняя панель", h=26.0)
                    win.lb("Строки ниже держат min_h", h=26.0)
                    with win.scr(pd=6.0, gp=6.0, h=180.0) as area:
                        win.cls(area, "clear")
                        rows_n = []
                        for i in range(6):
                            r = win.lb(f"Строка {i + 1}", h=24.0)
                            win.cls(r, "pane")
                            rows_n.append(r)

        with win.bx(pd=12.0, gp=8.0, w=380.0):
            t4 = win.lb("Пределы", h=28.0)
            win.cls(t4, "head")
            win.sep()

            win.lb(bind=lambda: f"min_w левой: {lo_left():.0f}", h=24.0)
            win.sl(0.35, h=32.0,
                   ch=lambda v: (lo_left.st(80.0 + v * 400.0),
                                 win.lim(left, min_w=lo_left())))

            win.lb(bind=lambda: f"min_w правой: {lo_right():.0f}", h=24.0)
            win.sl(0.42, h=32.0,
                   ch=lambda v: (lo_right.st(80.0 + v * 400.0),
                                 win.lim(right, min_w=lo_right())))

            win.lb(bind=lambda: f"Доля разделителя: {ratio():.2f}", h=24.0)
            win.sl(0.5, h=32.0,
                   ch=lambda v: (ratio.st(0.1 + v * 0.8),
                                 win.spl_ratio(sp, ratio())))

            with win.bx(ax="h", gp=8.0, h=52.0) as srow:
                win.cls(srow, "clear")
                win.bt("Влево до упора", h=44.0,
                       clk=lambda: (win.spl_ratio(sp, 0.1),
                                    log.st("упор влево")))
                win.bt("Вправо до упора", h=44.0,
                       clk=lambda: (win.spl_ratio(sp, 0.9),
                                    log.st("упор вправо")))

            win.sep()
            win.lb(bind=lambda: f"min_w кнопки: {btn_min():.0f}", h=24.0)
            win.sl(0.3, h=32.0,
                   ch=lambda v: (btn_min.st(60.0 + v * 260.0),
                                 win.lim(b1, min_w=btn_min())))
            win.lb(bind=lambda: f"max_w кнопки: {btn_max():.0f}", h=24.0)
            win.sl(0.45, h=32.0,
                   ch=lambda v: (btn_max.st(80.0 + v * 360.0),
                                 win.lim(b1, max_w=btn_max())))
            win.bt("Снять максимум", h=42.0,
                   clk=lambda: (win.lim(b1, max_w=0.0),
                                log.st("максимум снят")))

            win.sep()
            win.lb(bind=lambda: f"min_h строк: {row_min():.0f}", h=24.0)
            win.sl(0.0, h=32.0,
                   ch=lambda v: (row_min.st(24.0 + v * 70.0),
                                 [win.lim(r, min_h=row_min())
                                  for r in rows_n]))
            win.lb(bind=lambda: f"max_h строк: {row_max():.0f}", h=24.0)
            win.sl(1.0, h=32.0,
                   ch=lambda v: (row_max.st(20.0 + v * 80.0),
                                 [win.lim(r, max_h=row_max())
                                  for r in rows_n]))
            d3 = win.lb("Подними min_h — строки раздвинутся. "
                        "Опусти max_h ниже min_h — победит min_h.",
                        h=60.0, wrap=True)
            win.cls(d3, "dim")

            win.sep()
            win.lb(bind=lambda: f"Событие: {log()}", h=44.0, wrap=True)
            d = win.lb("Тяни рукоятку мышью: она встанет там, где "
                       "обеим половинам хватает их min_w.",
                       h=60.0, wrap=True)
            win.cls(d, "dim")
            d2 = win.lb("Сузь окно — таблица держит 420 px, "
                        "а место отдаёт соседняя панель.",
                        h=60.0, wrap=True)
            win.cls(d2, "dim")

    win.css(CSS)
    win.go()


if __name__ == "__main__":
    main()
