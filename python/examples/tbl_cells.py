"""SSUI — таблица: цвет ячейки, колонка в колбэке, прокрутка к строке.

Сверка прочитанных значений с эталоном: совпавшие ячейки зелёные,
расходящиеся красные. Клик сообщает и строку, и колонку.

Путь в репозитории: python/examples/tbl_cells.py
"""

import ssui

CSS = """
.clear { background: #00000000; }
.head  { color: #EEF3FF; font-size: 22; }
.dim   { color: #9AA4B2; }
.ok    { color: #22C55E; }
"""

COLS = ["Адрес", "Прочитано", "Эталон", "Ед."]

N = 60

# Строки: адрес, прочитанное, эталон, единица.
DATA = []
for i in range(N):
    addr = f"1-0:{i + 1}.8.0"
    read = f"{(i * 7919) % 100000:05d}"
    ref = read if i % 4 else f"{(i * 6271) % 100000:05d}"
    DATA.append([addr, read, ref, "кВт·ч" if i % 2 else "В"])


def main():
    win = ssui.W("SSUI · цвета ячеек таблицы", 1120, 800, thm="drk")

    rows = ssui.sgnl([list(r) for r in DATA])
    checked = ssui.sgnl(set())
    sel_row = ssui.sgnl(-1)
    sel_col = ssui.sgnl(-1)
    log = ssui.sgnl("—")

    def colors():
        # Красим колонки «Прочитано» и «Эталон» по совпадению.
        out = []
        for i in row_range():
            if i not in checked():
                continue
            r = rows()[i]
            same = r[1] == r[2]
            col = "#14321E" if same else "#4A1212"
            out.append(((i, 1), col))
            out.append(((i, 2), col))
        return out

    def row_range():
        return range(len(rows()))

    def on_click(row, col):
        sel_row.st(row)
        sel_col.st(col)
        log.st(f"строка {row}, колонка {col} — {COLS[col]}")

    def check_one():
        i = sel_row()
        if i < 0:
            log.st("сначала выбери строку")
            return
        c = set(checked())
        c.add(i)
        checked.st(c)
        r = rows()[i]
        log.st(f"строка {i}: "
               f"{'совпало' if r[1] == r[2] else 'расхождение'}")

    def check_all():
        checked.st(set(row_range()))
        bad = sum(1 for r in rows() if r[1] != r[2])
        log.st(f"сверено {len(rows())}, расхождений {bad}")

    def next_mismatch():
        # Прокрутка к первому расхождению — сценарий «показать
        # оператору проблемную строку».
        for i, r in enumerate(rows()):
            if r[1] != r[2]:
                win.tbl_see(tbl, i)
                sel_row.st(i)
                log.st(f"расхождение в строке {i}")
                return
        log.st("расхождений нет")

    def clear():
        checked.st(set())
        log.st("сверка сброшена")

    with win.bx(pr=win.rt(), ax="h", pd=14.0, gp=14.0) as root:
        win.cls(root, "clear")

        with win.bx(pd=12.0, gp=8.0):
            t = win.lb("Значения прибора", h=34.0)
            win.cls(t, "head")
            tbl = win.tbl(COLS, [list(r) for r in DATA],
                          ch=on_click, bg=colors,
                          hl=1.0, vl=1.0, h=680.0)

        with win.bx(pd=14.0, gp=8.0, w=320.0):
            t2 = win.lb("Сверка", h=34.0)
            win.cls(t2, "head")
            win.sep()
            win.bt("Сверить выбранную", h=44.0, clk=check_one)
            win.bt("Сверить все", h=44.0, clk=check_all)
            win.bt("К расхождению", h=44.0, clk=next_mismatch)
            win.bt("Сбросить", h=44.0, clk=clear)

            win.sep()
            c = win.lb(bind=lambda: f"Сверено: {len(checked())}", h=30.0)
            win.cls(c, "ok")
            win.lb(bind=lambda: (
                "Ничего не выбрано" if sel_row() < 0
                else f"Строка {sel_row()}, колонка {COLS[sel_col()]}"
            ), h=30.0)
            win.lb(bind=lambda: f"Событие: {log()}", h=56.0, wrap=True)

            win.sep()
            d = win.lb("Клик по любой колонке сообщает её номер — "
                       "старый однопараметрический ch продолжал бы "
                       "получать только строку.", h=76.0, wrap=True)
            win.cls(d, "dim")
            d2 = win.lb("Цвет ячейки перекрывает подсветку строки, "
                        "но не прячет текст.", h=56.0, wrap=True)
            win.cls(d2, "dim")

    win.css(CSS)
    win.go()


if __name__ == "__main__":
    main()
