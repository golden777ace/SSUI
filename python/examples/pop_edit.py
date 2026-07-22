"""SSUI — pop: слой поверх окна, редактирование значения в дереве.

Двойной клик по колонке «Значение» открывает поле ввода ровно
поверх ячейки. Enter записывает, Escape и клик мимо отменяют.
Дерево лежит внутри scr — слой границами прокрутки не обрезается.

Путь в репозитории: python/examples/pop_edit.py
"""

import ssui

CSS = """
.clear { background: #00000000; }
.head  { color: #EEF3FF; font-size: 22; }
.dim   { color: #9AA4B2; }
.layer { background: #0B1020; radius: 8; }
.field { background: #111827; radius: 6; color: #EEF3FF; }
.field:focus { background: #1E293B; }
"""

COLS = ["Имя", "Значение", "Ед.изм", "Действие"]

RAW = []
for grp, attrs in [
    ("Данные", [("Серийный номер", "12345678", ""),
                ("Тип счётчика", "КПЗ-2М", ""),
                ("Версия ПО", "3.14", "")]),
    ("Регистры", [("Активная A+", "148273.45", "кВт·ч"),
                  ("Тариф 1", "94112.30", "кВт·ч"),
                  ("Тариф 2", "54161.15", "кВт·ч")]),
    ("Профили", [("Период записи", "3600", "с"),
                 ("Записей в буфере", "1440", ""),
                 ("Глубина хранения", "60", "сут")]),
    ("Часы", [("Время прибора", "21.07.2026 14:32:11", ""),
              ("Часовой пояс", "180", "мин")]),
    ("Связь", [("HDLC адрес", "16", ""),
               ("Скорость порта", "9600", "бод"),
               ("Таймаут", "5000", "мс")]),
]:
    RAW.append((0, grp, False, "", ""))
    for name, val, unit in attrs:
        RAW.append((1, name, True, val, unit))


def main():
    win = ssui.W("SSUI · редактирование в дереве", 1180, 800, thm="drk")

    values = ssui.sgnl({i: r[3] for i, r in enumerate(RAW)})
    dirty = ssui.sgnl(set())
    target = ssui.sgnl(-1)
    draft = ssui.sgnl("")
    log = ssui.sgnl("—")
    # draft — сигнал текста редактируемого поля.

    def rows():
        v = values()
        d = dirty()
        out = []
        for i, (depth, name, leaf, _, unit) in enumerate(RAW):
            row = {
                "depth": depth,
                "text": name,
                "leaf": leaf,
                "open": True,
                "values": [v.get(i, ""), unit,
                           "запись" if leaf else ""],
            }
            if i in d:
                row["cbg"] = ["", "#3B2F0B", "", ""]
                row["cfg"] = ["", "#FDE68A", "", ""]
            out.append(row)
        return out

    def on_dbl(row, col):
        # Редактируется только колонка значения и только у листьев.
        if col != 1 or not RAW[row][2]:
            log.st("эта ячейка не редактируется")
            return
        r = win.tre_cell(tree, row, col)
        if r[2] <= 0.0:
            log.st("строка не видна")
            return
        target.st(row)
        draft.st(values().get(row, ""))
        win.pop_at(layer, r[0], r[1], r[2], r[3])
        win.focus(field, sel=True)
        # Значение подставляет сигнал draft; focus только ставит
        # курсор и выделяет текст.
        log.st(f"правка: {RAW[row][1]}")

    def finish(code):
        # Ядро зовёт с 1 по Enter и с 0 по Escape или потере фокуса.
        i = target()
        if i < 0:
            return
        if code == 1:
            cur = dict(values())
            cur[i] = draft()
            values.st(cur)
            d = set(dirty())
            d.add(i)
            dirty.st(d)
            log.st(f"записано: {RAW[i][1]} = {cur[i]}")
        else:
            log.st("отменено")
        target.st(-1)
        win.pop_off(layer)

    def on_close():
        # Закрытие пользователем: клик мимо или Escape.
        target.st(-1)
        log.st("слой закрыт")

    def revert():
        values.st({i: r[3] for i, r in enumerate(RAW)})
        dirty.st(set())
        log.st("значения восстановлены")

    with win.bx(pr=win.rt(), ax="h", pd=14.0, gp=14.0) as root:
        win.cls(root, "clear")

        with win.bx(pd=12.0, gp=8.0):
            t = win.lb("Двойной клик по значению", h=34.0)
            win.cls(t, "head")
            # Дерево внутри области прокрутки: проверка того, что
            # слой не обрезается её границами.
            with win.scr(h=640.0, pd=6.0) as area:
                win.cls(area, "clear")
                tree = win.tre([], bind=rows, dbl=on_dbl,
                               cols=COLS,
                               widths=[340.0, 240.0, 90.0, 0.0],
                               h=900.0)

        with win.bx(pd=14.0, gp=8.0, w=320.0):
            t2 = win.lb("Состояние", h=34.0)
            win.cls(t2, "head")
            win.sep()
            win.lb(bind=lambda: (
                "Ничего не правим" if target() < 0
                else f"Правим строку {target()}"), h=30.0)
            win.lb(bind=lambda: f"Изменено значений: {len(dirty())}",
                   h=28.0)
            win.lb(bind=lambda: f"Событие: {log()}", h=60.0, wrap=True)

            win.sep()
            win.bt("Восстановить всё", h=44.0, clk=revert)

            win.sep()
            d = win.lb("Enter записывает, Escape отменяет, "
                       "клик мимо тоже отменяет.", h=56.0, wrap=True)
            win.cls(d, "dim")
            d2 = win.lb("Изменённые значения помечаются цветом "
                        "самой ячейки, а не всей строки.",
                        h=56.0, wrap=True)
            win.cls(d2, "dim")

    # Слой создаётся последним и вне всех контейнеров: он цепляется
    # к корню окна независимо от того, где вызван.
    with win.pop(w=240.0, h=32.0, on_close=on_close) as layer:
        win.cls(layer, "layer")
        field = win.tx("", sig=draft, h=32.0)
        win.cls(field, "field")
        win.keys(field, finish)

    win.css(CSS)
    win.go()


if __name__ == "__main__":
    main()