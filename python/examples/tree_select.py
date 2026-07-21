"""SSUI — дерево: строка и колонка, двойной клик, множественный выбор.

Ctrl добавляет строку, Shift выделяет диапазон, двойной клик
по колонке значения имитирует вызов редактора.

Путь в репозитории: python/examples/tree_select.py
"""

import ssui

CSS = """
.clear { background: #00000000; }
.head  { color: #EEF3FF; font-size: 22; }
.dim   { color: #9AA4B2; }
.ok    { color: #22C55E; }
"""

COLS = ["Имя", "Значение", "Ед.изм", "Действие"]

RAW = [
    (0, "Данные", False, "", "", ""),
    (1, "Серийный номер", True, "12345678", "", "чтение"),
    (1, "Тип счётчика", True, "КПЗ-2М", "", "чтение"),
    (1, "Версия ПО", True, "3.14", "", "чтение"),
    (0, "Регистры", False, "", "", ""),
    (1, "Активная энергия", True, "148273.45", "кВт·ч", "чтение"),
    (1, "Тариф 1", True, "94112.30", "кВт·ч", "чтение"),
    (1, "Тариф 2", True, "54161.15", "кВт·ч", "чтение"),
    (1, "Тариф 3", True, "0.00", "кВт·ч", "чтение"),
    (0, "Профили", False, "", "", ""),
    (1, "Период записи", True, "3600", "с", "запись"),
    (1, "Записей в буфере", True, "1440", "", "чтение"),
    (0, "Часы", False, "", "", ""),
    (1, "Время прибора", True, "21.07.2026 14:32:11", "", "запись"),
    (1, "Часовой пояс", True, "180", "мин", "запись"),
]


def main():
    win = ssui.W("SSUI · выбор в дереве", 1200, 820, thm="drk")

    values = ssui.sgnl({i: r[3] for i, r in enumerate(RAW)})
    picked = ssui.sgnl([])
    last = ssui.sgnl("—")
    edit = ssui.sgnl("—")
    counter = ssui.sgnl(0)

    def rows():
        out = []
        for i, (d, name, leaf, _, unit, act) in enumerate(RAW):
            row = {
                "depth": d,
                "text": name,
                "leaf": leaf,
                "open": True,
                "values": [values().get(i, ""), unit, act],
            }
            if i in picked():
                row["fg"] = "#A7F3D0"
            out.append(row)
        return out

    def on_change(sel):
        # При multi=True приходит список индексов, а не число.
        picked.st(list(sel))
        counter.st(len(sel))

    def on_click(row, col):
        last.st(f"строка {row}, колонка {col} — {COLS[col]}")

    def on_double(row, col):
        # Двойной клик по колонке значения — вход в редактирование.
        # Здесь просто помечаем значение, на шаге 8 сюда встанет pop.
        if col != 1:
            edit.st(f"колонка {COLS[col]} не редактируется")
            return
        if not RAW[row][2]:
            edit.st("узел не редактируется")
            return
        cur = dict(values())
        cur[row] = cur.get(row, "") + "*"
        values.st(cur)
        edit.st(f"правка строки {row}: {RAW[row][1]}")

    def select_all():
        win.tre_sel(tree, list(range(len(RAW))))
        picked.st(list(range(len(RAW))))
        counter.st(len(RAW))

    def select_leaves():
        idx = [i for i, r in enumerate(RAW) if r[2]]
        win.tre_sel(tree, idx)
        picked.st(idx)
        counter.st(len(idx))

    def clear_sel():
        win.tre_sel(tree, [])
        picked.st([])
        counter.st(0)

    def bump():
        # Массовое действие над выделением — типичный сценарий
        # «записать выбранные атрибуты».
        cur = dict(values())
        for i in picked():
            if RAW[i][2]:
                cur[i] = cur.get(i, "") + "!"
        values.st(cur)
        edit.st(f"обработано строк: {len(picked())}")

    with win.bx(pr=win.rt(), ax="h", pd=14.0, gp=14.0) as root:
        win.cls(root, "clear")

        with win.bx(pd=12.0, gp=8.0):
            t = win.lb("Ctrl — добавить, Shift — диапазон", h=34.0)
            win.cls(t, "head")
            tree = win.tre([], bind=rows, multi=True,
                           ch=on_change, clk=on_click, dbl=on_double,
                           cols=COLS,
                           widths=[360.0, 260.0, 90.0, 0.0],
                           h=660.0)

        with win.bx(pd=14.0, gp=8.0, w=320.0):
            t2 = win.lb("Выделение", h=34.0)
            win.cls(t2, "head")
            win.sep()
            c = win.lb(bind=lambda: f"Выбрано строк: {counter()}", h=30.0)
            win.cls(c, "ok")
            win.bt("Выделить всё", h=44.0, clk=select_all)
            win.bt("Только атрибуты", h=44.0, clk=select_leaves)
            win.bt("Снять выделение", h=44.0, clk=clear_sel)
            win.bt("Пометить выбранные", h=44.0, clk=bump)

            win.sep()
            t3 = win.lb("События", h=32.0)
            win.cls(t3, "head")
            win.lb(bind=lambda: f"Клик: {last()}", h=52.0, wrap=True)
            win.lb(bind=lambda: f"Двойной: {edit()}", h=52.0, wrap=True)

            win.sep()
            d = win.lb("Колонка считается по тем же границам, "
                       "что и отрисовка: попробуй кликать по краям "
                       "разделителей.", h=76.0, wrap=True)
            win.cls(d, "dim")
            d2 = win.lb("Клик по треугольнику раскрытия не считается "
                        "ни кликом, ни выбором.", h=56.0, wrap=True)
            win.cls(d2, "dim")

    win.css(CSS)
    win.go()


if __name__ == "__main__":
    main()
