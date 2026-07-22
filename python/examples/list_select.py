"""SSUI — список: множественный выбор.

Ctrl добавляет пункт, Shift выделяет диапазон от текущего.
Кнопки задают выбор программно через lst_sel.

Путь в репозитории: python/examples/list_select.py
"""

import ssui

CSS = """
.clear { background: #00000000; }
.head  { color: #EEF3FF; font-size: 22; }
.dim   { color: #9AA4B2; }
.ok    { color: #22C55E; }
"""

ITEMS = [
    "Серийный номер",
    "Тип счётчика",
    "Версия ПО",
    "Активная энергия",
    "Тариф 1",
    "Тариф 2",
    "Тариф 3",
    "Период записи",
    "Записей в буфере",
    "Время прибора",
    "Часовой пояс",
    "Напряжение фазы A",
    "Напряжение фазы B",
    "Напряжение фазы C",
    "Ток фазы A",
    "Ток фазы B",
]


def main():
    win = ssui.W("SSUI · выбор в списке", 900, 720, thm="drk")

    picked = ssui.sgnl([0])
    counter = ssui.sgnl(1)

    def on_change(sel):
        # При multi=True приходит список индексов, а не число.
        picked.st(list(sel))
        counter.st(len(sel))

    def select_all():
        idx = list(range(len(ITEMS)))
        win.lst_sel(box, idx)
        picked.st(idx)
        counter.st(len(idx))

    def select_odd():
        idx = [i for i in range(len(ITEMS)) if i % 2]
        win.lst_sel(box, idx)
        picked.st(idx)
        counter.st(len(idx))

    def clear_sel():
        win.lst_sel(box, [])
        picked.st([])
        counter.st(0)

    with win.bx(pr=win.rt(), ax="h", pd=14.0, gp=14.0) as root:
        win.cls(root, "clear")

        with win.bx(pd=12.0, gp=8.0):
            t = win.lb("Ctrl — добавить, Shift — диапазон", h=34.0)
            win.cls(t, "head")
            box = win.lst(ITEMS, multi=True, sel=[0], ch=on_change, h=620.0)

        with win.bx(pd=14.0, gp=8.0, w=300.0):
            t2 = win.lb("Выделение", h=34.0)
            win.cls(t2, "head")
            win.sep()
            c = win.lb(bind=lambda: f"Выбрано: {counter()}", h=30.0)
            win.cls(c, "ok")
            win.bt("Выделить всё", h=44.0, clk=select_all)
            win.bt("Нечётные", h=44.0, clk=select_odd)
            win.bt("Снять выделение", h=44.0, clk=clear_sel)

            win.sep()
            p = win.lb(bind=lambda: "Индексы: "
                       + ", ".join(str(i) for i in picked()),
                       h=140.0, wrap=True)
            win.cls(p, "dim")

    win.css(CSS)
    win.go()


if __name__ == "__main__":
    main()
