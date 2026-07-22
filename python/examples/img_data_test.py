"""SSUI — изображение из памяти (data=bytes).

PNG генерируется прямо в процессе средствами stdlib (zlib), без файла
и без сторонних библиотек, и показывается через img(data=...). Это тот
же путь, каким подаётся график matplotlib из BytesIO.

Путь в репозитории: python/examples/img_data_test.py
"""

import struct
import zlib

import ssui

CSS = """
.clear { background: #00000000; }
.head  { color: #EEF3FF; font-size: 22; }
.dim   { color: #9AA4B2; }
.stage { background: #0B1020; radius: 12; }
"""


def png(width, height, pixel):
    """Кодирует RGB-изображение в PNG. pixel(x, y) -> (r, g, b)."""
    raw = bytearray()
    for y in range(height):
        raw.append(0)  # тип фильтра строки
        for x in range(width):
            r, g, b = pixel(x, y)
            raw += bytes((r & 255, g & 255, b & 255))

    def chunk(tag, data):
        body = tag + data
        return (struct.pack(">I", len(data)) + body
                + struct.pack(">I", zlib.crc32(body) & 0xFFFFFFFF))

    sig = b"\x89PNG\r\n\x1a\n"
    ihdr = struct.pack(">IIBBBBB", width, height, 8, 2, 0, 0, 0)
    idat = zlib.compress(bytes(raw), 9)
    return sig + chunk(b"IHDR", ihdr) + chunk(b"IDAT", idat) + chunk(b"IEND", b"")


def gradient(w, h):
    return png(w, h, lambda x, y: (
        int(255 * x / w), int(255 * y / h), 160))


def checker(w, h):
    return png(w, h, lambda x, y: (
        (34, 197, 94) if (x // 20 + y // 20) % 2 else (16, 24, 40)))


def rings(w, h):
    cx, cy = w / 2, h / 2
    def pix(x, y):
        d = ((x - cx) ** 2 + (y - cy) ** 2) ** 0.5
        t = int(d) % 40
        v = 255 - abs(t - 20) * 10
        return (v, 130, 246)
    return png(w, h, pix)


def main():
    win = ssui.W("SSUI · изображение из памяти", 980, 460, thm="drk")

    W, H = 260, 180
    imgs = [
        ("Градиент", gradient(W, H)),
        ("Шахматка", checker(W, H)),
        ("Кольца", rings(W, H)),
    ]

    with win.bx(pr=win.rt(), ax="v", pd=16.0, gp=12.0) as root:
        win.cls(root, "clear")

        t = win.lb("PNG сгенерированы в памяти и показаны без файла",
                   h=34.0)
        win.cls(t, "head")

        with win.bx(ax="h", gp=14.0, h=240.0) as row:
            win.cls(row, "clear")
            for name, blob in imgs:
                with win.bx(pd=8.0, gp=6.0) as cell:
                    win.cls(cell, "stage")
                    win.img(data=blob, fit="contain", h=190.0)
                    c = win.lb(name, h=28.0)
                    win.cls(c, "dim")

        win.sep()
        win.lb(f"Каждое изображение — {W}×{H}, закодировано через zlib "
               "и передано параметром data=bytes. Так же подаётся вывод "
               "matplotlib: fig.savefig(buf, format='png').", h=52.0,
               wrap=True)

    win.css(CSS)
    win.go()


if __name__ == "__main__":
    main()
