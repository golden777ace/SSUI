"""SSUI — Canvas: фигуры, хит-тест, измерение текста.

Клик по фигуре подсвечивает её и печатает индекс.
Ползунки меняют геометрию стрелки, дуги и сектора.
Правая панель показывает автоподбор размера шрифта
через win.measure_text().
"""

import math

import ssui

CSS = """
frame { background: #141c30cc; radius: 14; }
.clear { background: #00000000; }
label { color: #eef3ff; }
.head { color: #3b82f6; }
canvas { background: #0b1020; radius: 12; }
"""

BASE = "#3b82f6"
HOT = "#f59e0b"
DIM = "#4b5563"
TXT = "#eef3ff"


def main():
    win = ssui.W("SSUI · Canvas", 1180, 760, thm="drk", glass=True, tint=0.9)

    hit = ssui.sgnl(-1)
    ang = ssui.sgnl(0.0)
    sweep = ssui.sgnl(120.0)
    rad = ssui.sgnl(70.0)
    head = ssui.sgnl(16.0)
    thick = ssui.sgnl(3.0)
    spin = ssui.sgnl(0.0)
    caption = ssui.sgnl("Автоподбор размера")
    boxw = ssui.sgnl(300.0)

    NAMES = ["прямоугольник", "круг", "линия", "текст",
             "стрелка", "дуга", "сектор", "вектор A", "вектор B"]

    def col(i):
        return HOT if hit() == i else BASE

    def shapes():
        a = ang()
        sw = sweep()
        r = rad()
        t = thick()
        phase = spin()
        cx, cy = 470.0, 210.0

        # Два вектора под углом друг к другу — как в векторной диаграмме.
        a1 = math.radians(phase)
        a2 = math.radians(phase + sw)
        vx1 = cx + r * math.cos(a1)
        vy1 = cy + r * math.sin(a1)
        vx2 = cx + r * math.cos(a2)
        vy2 = cy + r * math.sin(a2)

        return [
            ("rect", [30.0, 30.0, 150.0, 90.0, 12.0, 0.0], col(0), ""),
            ("circle", [105.0, 200.0, 48.0, 0.0], col(1), ""),
            ("line", [30.0, 290.0, 180.0, 340.0, t], col(2), ""),
            ("text", [30.0, 360.0, 220.0, 26.0], col(3), "текстовая фигура"),
            ("arrow", [230.0, 60.0, 380.0, 150.0, t, head()], col(4), ""),
            ("arc", [300.0, 300.0, 70.0, a, sw, t], col(5), ""),
            ("sector", [660.0, 330.0, 60.0, a, sw, 0.0], col(6), ""),
            ("arrow", [cx, cy, vx1, vy1, t, head()], col(7), ""),
            ("arrow", [cx, cy, vx2, vy2, t, head()], col(8), ""),
            ("arc", [cx, cy, r * 0.45, phase, sw, 1.5], DIM, ""),
            ("circle", [cx, cy, 4.0, 0.0], TXT, ""),
        ]

    def on_hit(i):
        hit.st(i)

    def fit_size():
        """Наибольший размер шрифта, при котором текст влезает в бокс."""
        target = boxw()
        size = 8.0
        best = 8.0
        while size <= 72.0:
            w, _ = win.measure_text(caption(), size)
            if w > target:
                break
            best = size
            size += 1.0
        return best

    def fit_shapes():
        s = fit_size()
        w, h = win.measure_text(caption(), s)
        return [
            ("rect", [10.0, 40.0, boxw(), 90.0, 10.0, 2.0], DIM, ""),
            ("rect", [10.0, 40.0 + (90.0 - h) / 2.0, w, h, 4.0, 0.0],
             "#1e293b", ""),
            ("text", [10.0, 40.0 + (90.0 - h) / 2.0, w + 4.0, h], TXT,
             caption()),
        ]

    with win.bx(pr=win.rt(), ax="h", pd=14.0, gp=14.0) as root:
        win.cls(root, "clear")

        with win.bx(pd=14.0, gp=10.0, w=300.0):
            t = win.lb("Параметры фигур", h=30.0)
            win.cls(t, "head")
            win.sep()

            win.lb(bind=lambda: f"Начальный угол: {ang():.0f}°", h=24.0)
            win.sl(0.0, ch=lambda v: ang.st(v * 360.0), h=32.0)

            win.lb(bind=lambda: f"Разворот: {sweep():.0f}°", h=24.0)
            win.sl(0.33, ch=lambda v: sweep.st(v * 360.0), h=32.0)

            win.lb(bind=lambda: f"Радиус: {rad():.0f}", h=24.0)
            win.sl(0.5, ch=lambda v: rad.st(20.0 + v * 120.0), h=32.0)

            win.lb(bind=lambda: f"Толщина линий: {thick():.1f}", h=24.0)
            win.sl(0.25, ch=lambda v: thick.st(1.0 + v * 9.0), h=32.0)

            win.lb(bind=lambda: f"Наконечник: {head():.0f}", h=24.0)
            win.sl(0.4, ch=lambda v: head.st(4.0 + v * 30.0), h=32.0)

            win.lb(bind=lambda: f"Поворот вектора: {spin():.0f}°", h=24.0)
            win.sl(0.0, ch=lambda v: spin.st(v * 360.0), h=32.0)

            win.sep()
            win.lb(bind=lambda: (
                "Фигура не выбрана" if hit() < 0
                else f"Выбрано #{hit()}: {NAMES[hit()] if hit() < len(NAMES) else 'вспомогательная'}"
            ), h=48.0, wrap=True)
            win.bt("Сбросить выбор", h=42.0, clk=lambda: hit.st(-1))

        with win.bx(pd=14.0, gp=10.0):
            t2 = win.lb("Кликай по фигурам — хит-тест по индексу", h=30.0)
            win.cls(t2, "head")
            win.cv([], bind=shapes, ch=on_hit, h=420.0)

            win.sep()
            t3 = win.lb("measure_text: подбор размера под ширину", h=28.0)
            win.cls(t3, "head")
            with win.bx(ax="h", gp=8.0, h=48.0) as row:
                win.cls(row, "clear")
                win.lb("Текст:", w=70.0, h=40.0)
                win.tx("", sig=caption, ph="введи строку", h=44.0)
            win.lb(bind=lambda: (
                f"Ширина бокса: {boxw():.0f} · подобрано: {fit_size():.0f}px"
            ), h=26.0)
            win.sl(0.5, ch=lambda v: boxw.st(120.0 + v * 400.0), h=32.0)
            win.cv([], bind=fit_shapes, h=150.0)

    win.css(CSS)
    win.go()


if __name__ == "__main__":
    main()
