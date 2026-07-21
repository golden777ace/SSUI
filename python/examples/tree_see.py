"""SSUI — дерево: прокрутка к строке, сворачивание, координаты ячейки.

Имитация длинного опроса: рабочий поток читает атрибуты, дерево
само прокручивается к текущей строке. Двойной клик показывает
прямоугольник ячейки — место для попапа редактирования.

Путь в репозитории: python/examples/tree_see.py
"""

import threading
import time

import ssui

CSS = """
.clear { background: #00000000; }
.head  { color: #EEF3FF; font-size: 22; }
.dim   { color: #9AA4B2; }
.ok    { color: #22C55E; }
.mark  { background: #F59E0B; radius: 4; }
"""

COLS = ["Имя", "Значение", "Ед.изм", "Действие"]

OBJECTS = [
    ("Данные", ["Серийный номер", "Тип счётчика", "Версия ПО",
                "Дата выпуска", "Изготовитель"]),
    ("Регистры энергии", ["Активная A+", "Активная A-", "Реактивная R+",
                          "Реактивная R-", "Тариф 1", "Тариф 2",
                          "Тариф 3", "Тариф 4"]),
    ("Мгновенные значения", ["Напряжение A", "Напряжение B",
                             "Напряжение C", "Ток A", "Ток B", "Ток C",
                             "Частота", "Коэффициент мощности"]),
    ("Профили", ["Суточный профиль", "Месячный профиль",
                 "Период записи", "Записей в буфере", "Глубина хранения"]),
    ("Журналы", ["Журнал событий", "Журнал вскрытий",
                 "Журнал питания", "Журнал коррекций"]),
    ("Часы", ["Время прибора", "Часовой пояс", "Переход на летнее",
              "Сдвиг времени"]),
    ("Тарифы", ["Расписание", "Сезоны", "Специальные дни",
                "Активный тариф"]),
    ("Связь", ["HDLC адрес", "Скорость порта", "Таймаут",
               "Пароль низкий", "Пароль высокий"]),
]

# Плоский список: (глубина, имя, лист).
RAW = []
for grp, attrs in OBJECTS:
    RAW.append((0, grp, False))
    for a in attrs:
        RAW.append((1, a, True))


def main():
    win = ssui.W("SSUI · прокрутка и ячейки", 1220, 840, thm="drk")

    post = win.post()

    values = ssui.sgnl({})
    done = ssui.sgnl(set())
    cur = ssui.sgnl(-1)
    busy = ssui.sgnl(False)
    cell = ssui.sgnl((0.0, 0.0, 0.0, 0.0))
    info = ssui.sgnl("—")

    def rows():
        v = values()
        d = done()
        c = cur()
        out = []
        for i, (depth, name, leaf) in enumerate(RAW):
            row = {
                "depth": depth,
                "text": name,
                "leaf": leaf,
                "open": True,
                "values": [v.get(i, ""), "", "чтение" if leaf else ""],
            }
            if i == c:
                row["bg"] = "#3B2F0B"
                row["fg"] = "#FDE68A"
            elif i in d:
                row["fg"] = "#A7F3D0"
            out.append(row)
        return out

    def poll():
        # Рабочий поток: читает атрибуты по одному и возвращает
        # результат в UI-поток. Прокрутка к текущей строке — то,
        # по чему оператор видит ход опроса.
        for i, (depth, name, leaf) in enumerate(RAW):
            if not leaf:
                continue
            time.sleep(0.12)

            def apply(k=i, n=name):
                cur.st(k)
                cur_v = dict(values())
                cur_v[k] = f"{(k * 7919) % 100000:05d}"
                values.st(cur_v)
                d = set(done())
                d.add(k)
                done.st(d)
                win.tre_see(tree, k)
                # Заявка на прокрутку: применится в начале кадра,
                # уже после того, как строки пересобраны.

            post(apply)

        post(lambda: cur.st(-1))
        post(lambda: busy.st(False))
        post(lambda: win.nt()("Опрос", f"Прочитано: {len(done())}"))

    def start():
        if busy():
            return
        busy.st(True)
        values.st({})
        done.st(set())
        threading.Thread(target=poll, daemon=True).start()

    def on_dbl(row, col):
        # Прямоугольник ячейки в координатах окна. На шаге 8 сюда
        # встанет pop с полем ввода; пока рисуем рамку.
        r = win.tre_cell(tree, row, col)
        cell.st(r)
        if r[2] <= 0.0:
            info.st("строка не видна — сначала tre_see")
        else:
            info.st(f"{COLS[col]} строки {row}: "
                    f"x={r[0]:.0f} y={r[1]:.0f} w={r[2]:.0f} h={r[3]:.0f}")

    def go_to(i):
        def f():
            win.tre_see(tree, i)
            info.st(f"переход к строке {i}")
        return f

    with win.bx(pr=win.rt(), ax="h", pd=14.0, gp=14.0) as root:
        win.cls(root, "clear")

        with win.bx(pd=12.0, gp=8.0):
            t = win.lb("Опрос атрибутов", h=34.0)
            win.cls(t, "head")
            tree = win.tre([], bind=rows, dbl=on_dbl,
                           cols=COLS,
                           widths=[380.0, 240.0, 90.0, 0.0],
                           h=680.0)

            # Рамка ячейки: абсолютное размещение по tre_cell.
            frame = win.lb("", h=1.0)
            win.cls(frame, "mark")
            win.pl(frame, x=-1000.0, y=-1000.0, w=1.0, h=1.0)
            win.bindp(frame, lambda: (
                (cell()[0], cell()[1], cell()[2], cell()[3])
                if cell()[2] > 0.0 else (-1000.0, -1000.0, 1.0, 1.0)))
            win.dep(frame, 400)
            win.ghost(frame)

        with win.bx(pd=14.0, gp=8.0, w=330.0):
            t2 = win.lb("Опрос", h=34.0)
            win.cls(t2, "head")
            win.sep()
            win.bt("Прочитать всё", h=48.0, clk=start)
            win.lb(bind=lambda: f"Прочитано: {len(done())} из "
                               f"{sum(1 for r in RAW if r[2])}", h=28.0)
            win.pr(0.0, bind=lambda: len(done()) / max(
                1, sum(1 for r in RAW if r[2])), h=18.0)
            c = win.lb(bind=lambda: (
                "Ожидание" if cur() < 0 else f"Читаем: {RAW[cur()][1]}"),
                h=30.0)
            win.cls(c, "ok")

            win.sep()
            t3 = win.lb("Поддеревья", h=32.0)
            win.cls(t3, "head")
            win.bt("Раскрыть всё", h=42.0,
                   clk=lambda: win.tre_open(tree, []))
            win.bt("Свернуть всё", h=42.0,
                   clk=lambda: win.tre_open(tree, [], on=False))
            win.bt("Свернуть регистры", h=42.0,
                   clk=lambda: win.tre_open(tree, [6], on=False))

            win.sep()
            t4 = win.lb("Переход к строке", h=32.0)
            win.cls(t4, "head")
            win.bt("К последнему объекту", h=42.0,
                   clk=go_to(len(RAW) - 1))
            win.bt("К середине", h=42.0, clk=go_to(len(RAW) // 2))
            win.bt("К началу", h=42.0, clk=go_to(0))
            d = win.lb("Переход раскрывает свёрнутых предков строки.",
                       h=48.0, wrap=True)
            win.cls(d, "dim")

            win.sep()
            win.lb(bind=lambda: f"Ячейка: {info()}", h=76.0, wrap=True)
            d2 = win.lb("Двойной клик по ячейке обводит её рамкой — "
                        "ровно туда встанет поле редактирования.",
                        h=76.0, wrap=True)
            win.cls(d2, "dim")

    win.css(CSS)
    win.go()


if __name__ == "__main__":
    main()
