"""SSUI — дерево: колонки, признаки строк, иконки.

Модель дерева атрибутов DLMS: четыре колонки, восемь признаков
строки с разным фоном и цветом текста, плоский список с depth.

Путь в репозитории: python/examples/tree_cols.py
"""

import ssui

CSS = """
.clear { background: #00000000; }
.head  { color: #EEF3FF; font-size: 22; }
.dim   { color: #9AA4B2; }
"""

# Восемь признаков строки: фон и цвет текста.
STATES = {
    "обычный":    (None, None),
    "прочитано":  ("#14321E", "#A7F3D0"),
    "изменено":   ("#3B2F0B", "#FDE68A"),
    "записано":   ("#0B2E4A", "#BFDBFE"),
    "ошибка":     ("#4A1212", "#FCA5A5"),
    "нет доступа": ("#2A2A2A", "#6B7280"),
    "только чтение": (None, "#9AA4B2"),
    "устарело":   ("#2B1B3A", "#D8B4FE"),
}

ORDER = list(STATES)

# Дерево объектов: класс, объект, атрибуты.
RAW = [
    (0, "Данные (class 1)", False, "", "", ""),
    (1, "0-0:96.1.0 Серийный номер", False, "", "", ""),
    (2, "logical_name", True, "0-0:96.1.0*255", "", "чтение"),
    (2, "value", True, "12345678", "", "чтение"),
    (1, "0-0:96.1.1 Тип счётчика", False, "", "", ""),
    (2, "logical_name", True, "0-0:96.1.1*255", "", "чтение"),
    (2, "value", True, "КПЗ-2М", "", "чтение"),
    (0, "Регистры (class 3)", False, "", "", ""),
    (1, "1-0:1.8.0 Активная энергия", False, "", "", ""),
    (2, "logical_name", True, "1-0:1.8.0*255", "", "чтение"),
    (2, "value", True, "148273.45", "кВт·ч", "чтение"),
    (2, "scaler_unit", True, "-2, 30", "", "чтение"),
    (1, "1-0:1.8.1 Тариф 1", False, "", "", ""),
    (2, "value", True, "94112.30", "кВт·ч", "чтение"),
    (1, "1-0:1.8.2 Тариф 2", False, "", "", ""),
    (2, "value", True, "54161.15", "кВт·ч", "чтение"),
    (0, "Профили (class 7)", False, "", "", ""),
    (1, "1-0:99.1.0 Суточный профиль", False, "", "", ""),
    (2, "buffer", True, "1440 записей", "", "выгрузка"),
    (2, "capture_period", True, "3600", "с", "запись"),
    (2, "entries_in_use", True, "1440", "", "чтение"),
    (0, "Часы (class 8)", False, "", "", ""),
    (1, "0-0:1.0.0 Время прибора", False, "", "", ""),
    (2, "time", True, "21.07.2026 14:32:11", "", "запись"),
    (2, "time_zone", True, "180", "мин", "запись"),
]


def main():
    win = ssui.W("SSUI · дерево с колонками", 1180, 820, thm="drk")

    # Признак каждой строки: индекс в ORDER.
    marks = ssui.sgnl([0] * len(RAW))
    sel = ssui.sgnl(-1)
    only_objects = ssui.sgnl(False)
    icons = ssui.sgnl(False)
    log = ssui.sgnl("—")

    def rows():
        out = []
        for i, (d, name, leaf, val, unit, act) in enumerate(RAW):
            if only_objects() and d == 2:
                continue
            # Глубина 2 — это атрибуты. При фильтре остаются только
            # классы и объекты, узлы объектов становятся листьями.
            bg, fg = STATES[ORDER[marks()[i]]]
            row = {
                "depth": d,
                "text": name,
                "leaf": leaf or (only_objects() and d == 1),
                "open": True,
                "values": [val, unit, act],
            }
            if bg:
                row["bg"] = bg
            if fg:
                row["fg"] = fg
            if icons() and leaf:
                row["icon"] = "logo.ico"
            out.append(row)
        return out

    def on_sel(i):
        sel.st(i)
        if 0 <= i < len(RAW):
            log.st(f"строка {i}: {RAW[i][1]}")

    def mark(state):
        def f():
            i = sel()
            if i < 0 or i >= len(RAW):
                return
            cur = list(marks())
            cur[i] = ORDER.index(state)
            marks.st(cur)
            log.st(f"{RAW[i][1]} → {state}")
        return f

    def mark_all(state):
        def f():
            marks.st([ORDER.index(state)] * len(RAW))
            log.st(f"все строки → {state}")
        return f

    with win.bx(pr=win.rt(), ax="h", pd=14.0, gp=14.0) as root:
        win.cls(root, "clear")

        with win.bx(pd=12.0, gp=8.0):
            t = win.lb("Атрибуты прибора", h=34.0)
            win.cls(t, "head")
            win.tre([], bind=rows, ch=on_sel,
                    cols=["Имя", "Значение", "Ед.изм", "Действие"],
                    widths=[380.0, 240.0, 90.0, 0.0],
                    h=640.0)

        with win.bx(pd=14.0, gp=6.0, w=300.0):
            t2 = win.lb("Признак строки", h=34.0)
            win.cls(t2, "head")
            win.sep()
            for state in ORDER:
                win.bt(state, h=38.0, clk=mark(state))

            win.sep()
            win.bt("Сбросить все", h=42.0, clk=mark_all("обычный"))
            win.bt("Все прочитаны", h=42.0, clk=mark_all("прочитано"))

            win.sep()
            win.sw("Скрыть атрибуты", on=False,
                   clk=lambda v: only_objects.st(v), h=38.0)
            win.sw("Иконки на листьях", on=False,
                   clk=lambda v: icons.st(v), h=38.0)

            win.sep()
            win.lb(bind=lambda: (
                "Строка не выбрана" if sel() < 0
                else f"Выбрана строка #{sel()}"
            ), h=28.0)
            win.lb(bind=lambda: f"Событие: {log()}", h=48.0, wrap=True)
            d = win.lb("Ширины: 380, 240, 90 и остаток. "
                       "Последняя колонка тянется за окном.",
                       h=56.0, wrap=True)
            win.cls(d, "dim")

    win.css(CSS)
    win.go()


if __name__ == "__main__":
    main()