"""SSUI — ширины и минимумы колонок таблицы и дерева.

Демонстрирует:
  • win.cols(node, widths=[...]) — фиксированные ширины колонок;
  • win.cols(node, mins=[...]) — минимумы, ниже которых не сжать;
  • совместную работу с win.lim(node, min_w=...) у самого узла;
  • одинаковое поведение у tbl и tre.

Таблица и дерево лежат в разделителе: тяни рукоятку и смотри,
где колонки упираются в свои минимумы.
"""

import ssui

CSS = """
frame { background: #141c30cc; radius: 14; }
.clear { background: #00000000; }
label { color: #eef3ff; }
.head { color: #3b82f6; }
.dim  { color: #94a3b8; }
.pane { background: #16203a; radius: 10; }
"""

COLS = ["Параметр", "Значение", "Ед.", "Статус"]
DATA = [
    ["Напряжение фазы A", "231.4", "В", "норма"],
    ["Напряжение фазы B", "229.8", "В", "норма"],
    ["Напряжение фазы C", "198.2", "В", "просадка"],
    ["Ток фазы A", "5.42", "А", "норма"],
    ["Ток фазы B", "5.38", "А", "норма"],
    ["Ток фазы C", "5.51", "А", "норма"],
    ["Частота сети", "50.01", "Гц", "норма"],
    ["Коэффициент мощности", "0.97", "—", "норма"],
    ["Активная энергия A+", "184320.55", "кВт·ч", "норма"],
    ["Реактивная энергия R+", "42188.10", "квар·ч", "норма"],
]

TREE = [
    (0, "Прибор", False),
    (1, "Серийный номер", True),
    (1, "Версия прошивки", True),
    (0, "Регистры", False),
    (1, "Активная A+", True),
    (1, "Активная A-", True),
    (0, "Связь", False),
    (1, "HDLC адрес", True),
    (1, "Скорость порта", True),
]


def main():
    win = ssui.W("SSUI · минимумы колонок", 1380, 860, thm="drk",
                 glass=True, tint=0.12)

    m0 = ssui.sgnl(180.0)
    m1 = ssui.sgnl(90.0)
    m2 = ssui.sgnl(50.0)
    m3 = ssui.sgnl(110.0)
    w0 = ssui.sgnl(0.0)
    ratio = ssui.sgnl(0.55)
    tre_min = ssui.sgnl(200.0)
    log = ssui.sgnl("—")

    def push_tbl():
        win.cols(tbl,
                 widths=[w0(), 0.0, 0.0, 0.0],
                 mins=[m0(), m1(), m2(), m3()])
        log.st(f"минимумы: {m0():.0f} / {m1():.0f} / "
               f"{m2():.0f} / {m3():.0f}")

    def push_tre():
        win.cols(tre, mins=[tre_min(), 0.0, 0.0])
        log.st(f"минимум первой колонки дерева: {tre_min():.0f}")

    def rows_tre():
        out = []
        for depth, name, leaf in TREE:
            out.append({
                "depth": depth,
                "text": name,
                "leaf": leaf,
                "open": True,
                "values": ["—", "чтение"],
            })
        return out

    with win.bx(pr=win.rt(), ax="h", pd=14.0, gp=14.0) as root:
        win.cls(root, "clear")

        with win.bx(pd=12.0, gp=10.0):
            t = win.lb("Таблица и дерево в разделителе", h=28.0)
            win.cls(t, "head")

            with win.spl(ratio=0.55, h=640.0) as sp:
                with win.bx(pd=8.0, gp=6.0) as ltop:
                    win.cls(ltop, "pane")
                    win.lb("Таблица", h=26.0)
                    tbl = win.tbl(COLS, [list(r) for r in DATA],
                                  hl=1.0, vl=1.0, h=560.0,
                                  ch=lambda r, c: log.st(
                                      f"ячейка {r}·{COLS[c]}"))
                with win.bx(pd=8.0, gp=6.0) as rtop:
                    win.cls(rtop, "pane")
                    win.lb("Дерево", h=26.0)
                    tre = win.tre([], bind=rows_tre,
                                  cols=["Имя", "Значение", "Действие"],
                                  h=560.0,
                                  clk=lambda r, c: log.st(
                                      f"строка {r}, колонка {c}"))


        with win.bx(pd=12.0, gp=8.0, w=380.0):
            t2 = win.lb("Минимумы колонок таблицы", h=28.0)
            win.cls(t2, "head")
            win.sep()

            win.lb(bind=lambda: f"«Параметр»: {m0():.0f}", h=24.0)
            win.sl(0.45, h=32.0,
                   ch=lambda v: (m0.st(60.0 + v * 260.0), push_tbl()))
            win.lb(bind=lambda: f"«Значение»: {m1():.0f}", h=24.0)
            win.sl(0.25, h=32.0,
                   ch=lambda v: (m1.st(40.0 + v * 200.0), push_tbl()))
            win.lb(bind=lambda: f"«Ед.»: {m2():.0f}", h=24.0)
            win.sl(0.1, h=32.0,
                   ch=lambda v: (m2.st(30.0 + v * 200.0), push_tbl()))
            win.lb(bind=lambda: f"«Статус»: {m3():.0f}", h=24.0)
            win.sl(0.35, h=32.0,
                   ch=lambda v: (m3.st(40.0 + v * 200.0), push_tbl()))

            win.sep()
            win.lb(bind=lambda: (
                "Первая колонка делится поровну"
                if w0() <= 0.0 else f"Первая колонка: {w0():.0f} px"), h=24.0)
            win.sl(0.0, h=32.0,
                   ch=lambda v: (w0.st(0.0 if v < 0.05 else 80.0 + v * 320.0),
                                 push_tbl()))
            win.bt("Сбросить ширины", h=42.0,
                   clk=lambda: (w0.st(0.0), push_tbl()))

            win.sep()
            t3 = win.lb("Дерево", h=28.0)
            win.cls(t3, "head")
            win.lb(bind=lambda: f"Минимум «Имя»: {tre_min():.0f}", h=24.0)
            win.sl(0.4, h=32.0,
                   ch=lambda v: (tre_min.st(60.0 + v * 340.0), push_tre()))

            win.sep()
            t4 = win.lb("Разделитель", h=28.0)
            win.cls(t4, "head")
            win.lb(bind=lambda: f"Доля: {ratio():.2f}", h=24.0)
            win.sl(0.55, h=32.0,
                   ch=lambda v: (ratio.st(0.1 + v * 0.8),
                                 win.spl_ratio(sp, ratio())))

            win.sep()
            win.lb(bind=lambda: f"Событие: {log()}", h=44.0, wrap=True)
            d = win.lb("Сумма минимумов больше ширины — колонки "
                       "перестают делиться и уезжают под прокрутку.",
                       h=60.0, wrap=True)
            win.cls(d, "dim")
            d2 = win.lb("Ноль в mins возвращает умолчание: 40 px "
                        "для делимой колонки.", h=48.0, wrap=True)
            win.cls(d2, "dim")

    win.cols(tbl, mins=[m0(), m1(), m2(), m3()])
    win.cols(tre, mins=[tre_min(), 0.0, 0.0])
    win.css(CSS)
    win.go()


if __name__ == "__main__":
    main()
