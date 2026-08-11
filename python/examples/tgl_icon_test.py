"""SSUI — иконочные кнопки-переключатели тулбара.

Четыре режима правки канваса: «Выбрать/Добавить/Изменить/Удалить».
Взаимоисключение сделано вручную через общий сигнал режима: каждый
tgl связан своим sig, а выбор режима отжимает остальные.

Иконки рисуются в PNG на лету, чтобы пример не тянул внешних файлов.

Путь в репозитории: python/examples/tgl_icon_test.py
"""

import os
import struct
import tempfile
import zlib

import ssui

CSS = """
.clear { background: #00000000; }
.head  { color: #EEF3FF; font-size: 22; }
.dim   { color: #9AA4B2; }
.ok    { color: #22C55E; }
.bar   { background: #171A21F2; }
"""

SIZE = 32
MODES = ["Выбрать", "Добавить", "Изменить", "Удалить"]


def _chunk(tag, data):
    body = tag + data
    return (struct.pack(">I", len(data)) + body
            + struct.pack(">I", zlib.crc32(body) & 0xFFFFFFFF))


def _png(path, pixel):
    rows = bytearray()
    for y in range(SIZE):
        rows.append(0)
        for x in range(SIZE):
            rows += bytes(pixel(x, y))
    head = struct.pack(">IIBBBBB", SIZE, SIZE, 8, 6, 0, 0, 0)
    blob = (b"\x89PNG\r\n\x1a\n" + _chunk(b"IHDR", head)
            + _chunk(b"IDAT", zlib.compress(bytes(rows)))
            + _chunk(b"IEND", b""))
    with open(path, "wb") as f:
        f.write(blob)
    return path


def _on(x, y):
    return (238, 243, 255, 255)


def _off():
    return (0, 0, 0, 0)


def arrow(x, y):
    return _on(x, y) if 4 <= y <= 26 and 6 <= x <= 6 + (y - 4) // 2 else _off()


def plus(x, y):
    thick = 13 <= x <= 18 or 13 <= y <= 18
    return _on(x, y) if thick and 5 <= x <= 26 and 5 <= y <= 26 else _off()


def frame(x, y):
    edge = x in (6, 25) or y in (6, 25)
    return _on(x, y) if edge and 6 <= x <= 25 and 6 <= y <= 25 else _off()


def cross(x, y):
    d1 = abs(x - y) <= 2
    d2 = abs(x + y - 31) <= 2
    return _on(x, y) if (d1 or d2) and 6 <= x <= 25 and 6 <= y <= 25 else _off()


def make_icons():
    folder = os.path.join(tempfile.gettempdir(), "ssui_tgl_icons")
    os.makedirs(folder, exist_ok=True)
    draws = [arrow, plus, frame, cross]
    names = ["select", "add", "edit", "delete"]
    return [_png(os.path.join(folder, f"{n}.png"), d)
            for n, d in zip(names, draws)]


def main():
    icons = make_icons()

    win = ssui.W("SSUI · иконочные переключатели", 900, 560, thm="drk")
    thm = win.thm()

    mode = ssui.sgnl(0)
    free = ssui.sgnl(False)
    flags = [ssui.sgnl(i == 0) for i in range(len(MODES))]

    def pick(i):
        def on(v):
            if free():
                return
            if v:
                mode.st(i)
                for j, s in enumerate(flags):
                    if j != i:
                        s.st(False)
            else:
                flags[i].st(True)
        return on

    with win.bx(pr=win.rt(), ax="v", pd=16.0, gp=14.0) as root:
        win.cls(root, "clear")

        t = win.lb("Тулбар правки виджетов канваса", h=34.0)
        win.cls(t, "head")

        with win.bx(ax="h", gp=8.0, h=64.0, pd=8.0) as bar:
            win.cls(bar, "bar")
            for i, name in enumerate(MODES):
                win.tgl("", icon=icons[i], tip=name, on=flags[i](),
                        sig=flags[i], clk=pick(i), w=48.0, h=48.0)
            win.sep(vertical=True)
            win.tgl("Сетка", icon=icons[2], tip="Показывать сетку",
                    w=170.0, h=48.0)

        win.sep()

        with win.bx(ax="h", gp=12.0, h=52.0) as ctl:
            win.cls(ctl, "clear")
            win.sw("Свободный выбор (без взаимоисключения)", sig=free,
                   h=44.0, w=420.0)
            win.dd(["drk", "blk", "lit", "wht"], sel=0, w=140.0, h=44.0,
                   ch=lambda i: thm(["drk", "blk", "lit", "wht"][i]))

        m = win.lb(bind=lambda: f"Режим: {MODES[mode()]}", h=32.0)
        win.cls(m, "ok")
        s = win.lb(bind=lambda: "Нажаты: " + ", ".join(
            MODES[i] for i, f in enumerate(flags) if f()) or "нет", h=30.0)
        win.cls(s, "dim")

        win.sep()
        win.lb("Пустой lb + icon даёт кнопку-иконку; фон включённой "
               "кнопки красится акцентом. Подсказка берётся из tip.",
               h=48.0, wrap=True)

    win.css(CSS)
    win.go()


if __name__ == "__main__":
    main()
