"""SSUI — дерево: цвет отдельной ячейки, клавиатура, полоса прокрутки.

Главное отличие от Tkinter: красить можно не только строку целиком,
но и каждую ячейку отдельно. Здесь этим показан результат сверки
значения с эталоном по каждой колонке независимо.

Путь в репозитории: python/examples/tree_cells.py
"""

import ssui

CSS = """
.clear { background: #00000000; }
.head  { color: #EEF3FF; font-size: 22; }
.dim   { color: #9AA4B2; }
"""

COLS = ["Имя", "Значение", "Ед.изм", "Действие"]

# Цвета состояний ячейки.
RED = "#4A1212"
GREEN = "#14321E"
AMBER = "#3B2F0B"
BLUE = "#0B2E4A"

FG_RED = "#FCA5A5"
FG_GREEN = "#A7F3D0"
FG_AMBER = "#FDE68A"

RAW = []
for grp, attrs in [
    ("Данные", ["Серийный номер", "Тип счётчика", "Версия ПО",
                "Дата выпуска"]),
    ("Регистры", ["Активная A+", "Активная A-", "Тариф 1", "Тариф 2",
                  "Тариф 3", "Тариф 4"]),
    ("Мгновенные", ["Напряжение A", "Напряжение B", "Напряжение C",
                    "Ток A", "Ток B", "Ток C", "Частота"]),
    ("Профили", ["Период записи", "Записей в буфере", "Глубина"]),
    ("Часы", ["Время прибора", "Часовой пояс", "Летнее время"]),
    ("Связь", ["HDLC адрес", "Скорость", "Таймаут", "Пароль"]),
]:
    RAW.append((0, grp, False))
    for a in attrs:
        RAW.append((1, a, True))


def main():
    win = ssui.W("SSUI · цвета ячеек", 1220, 840, thm="drk")

    # Состояние каждой ячейки: 0 нет, 1 совпало, 2 расхождение,
    # 3 изменено, 4 записано. Ключ — (строка, колонка).
    state = ssui.sgnl({})
    sel = ssui.sgnl(-1)
    col = ssui.sgnl(-1)
    log = ssui.sgnl("—")

    PAL = {
        0: (None, None),
        1: (GREEN, FG_GREEN),
        2: (RED, FG_RED),
        3: (AMBER, FG_AMBER),
        4: (BLUE, None),
    }

    def rows():
        st = state()
        out = []
        for i, (depth, name, leaf) in enumerate(RAW):
            vals = ["", "", ""]
            if leaf:
                vals = [f"{(i * 7919) % 100000:05d}",
                        ["кВт·ч", "В", "А", "с", ""][i % 5],
                        "чтение" if i % 3 else "запись"]
            cbg = []
            cfg = []
            for c in range(4):
                bg, fg = PAL[st.get((i, c), 0)]
                cbg.append(bg or "")
                cfg.append(fg or "")
            out.append({
                "depth": depth,
                "text": name,
                "leaf": leaf,
                "open": True,
                "values": vals,
                "cbg": cbg,
                "cfg": cfg,
            })
        return out

    def on_click(row, c):
        sel.st(row)
        col.st(c)
        log.st(f"строка {row}, колонка {COLS[c]}")

    def paint(code):
        def f():
            r, c = sel(), col()
            if r < 0 or c < 0:
                log.st("сначала кликни по ячейке")
                return
            cur = dict(state())
            cur[(r, c)] = code
            state.st(cur)
            log.st(f"{COLS[c]} строки {r} → {code}")
        return f

    def sweep(code):
        # Массовая раскраска одной колонки — так в приложении
        # помечается результат сверки по всем атрибутам сразу.
        def f():
            c = col() if col() >= 0 else 1
            cur = dict(state())
            for i, r in enumerate(RAW):
                if r[2]:
                    cur[(i, c)] = code
            state.st(cur)
            log.st(f"колонка {COLS[c]} → {code}")
        return f

    def rainbow():
        # Каждая ячейка своим цветом: проверка, что окраска
        # действительно поячеечная, а не построчная.
        cur = {}
        for i, r in enumerate(RAW):
            if not r[2]:
                continue
            for c in range(4):
                cur[(i, c)] = 1 + (i + c) % 4
        state.st(cur)
        log.st("каждая ячейка своим цветом")

    def clear():
        state.st({})
        log.st("окраска снята")

    with win.bx(pr=win.rt(), ax="h", pd=14.0, gp=14.0) as root:
        win.cls(root, "clear")

        with win.bx(pd=12.0, gp=8.0):
            t = win.lb("Кликни по ячейке и назначь ей цвет", h=34.0)
            win.cls(t, "head")
            win.tre([], bind=rows, clk=on_click,
                    cols=COLS,
                    widths=[360.0, 240.0, 100.0, 0.0],
                    h=690.0)

        with win.bx(pd=14.0, gp=6.0, w=320.0):
            t2 = win.lb("Цвет ячейки", h=34.0)
            win.cls(t2, "head")
            win.sep()
            win.lb(bind=lambda: (
                "Ячейка не выбрана" if sel() < 0
                else f"Строка {sel()}, {COLS[col()]}"
            ), h=30.0)
            win.bt("Совпало", h=40.0, clk=paint(1))
            win.bt("Расхождение", h=40.0, clk=paint(2))
            win.bt("Изменено", h=40.0, clk=paint(3))
            win.bt("Записано", h=40.0, clk=paint(4))
            win.bt("Снять с ячейки", h=40.0, clk=paint(0))

            win.sep()
            t3 = win.lb("Массово", h=32.0)
            win.cls(t3, "head")
            win.bt("Колонку — совпало", h=40.0, clk=sweep(1))
            win.bt("Колонку — расхождение", h=40.0, clk=sweep(2))
            win.bt("Каждой свой цвет", h=40.0, clk=rainbow)
            win.bt("Очистить всё", h=40.0, clk=clear)

            win.sep()
            win.lb(bind=lambda: f"Событие: {log()}", h=52.0, wrap=True)

            win.sep()
            d = win.lb("Tab доводит фокус до дерева, дальше стрелки: "
                       "вверх и вниз по строкам, вправо раскрывает, "
                       "влево сворачивает или уходит к родителю.",
                       h=96.0, wrap=True)
            win.cls(d, "dim")
            d2 = win.lb("Полосу прокрутки можно тянуть мышью; "
                        "верхняя строка больше не заезжает "
                        "под заголовок.", h=76.0, wrap=True)
            win.cls(d2, "dim")

    win.css(CSS)
    win.go()


if __name__ == "__main__":
    main()
