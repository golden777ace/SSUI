"""SSUI — Canvas: вид poly, произвольный многоугольник.

Галочка флага, стрелки прокрутки массива, звезда
и самопересекающийся контур. Клик подсвечивает фигуру.

Путь в репозитории: python/examples/cv_poly.py
"""

import math

import ssui

CSS = """
.clear { background: #00000000; }
.head  { color: #EEF3FF; font-size: 22; }
.dim   { color: #9AA4B2; }
canvas { background: #0B1020; radius: 12; }
"""

BASE = "#3B82F6"
HOT = "#F59E0B"
OK = "#22C55E"
DIM = "#4B5563"
TXT = "#EEF3FF"

# Исходная галочка из задания.
CHECK = [20.0, 10.0, 50.0, 100.0, 100.0, 0.0, 50.0, 75.0]


def star(cx, cy, r, tips=5, phase=0.0):
    """Вершины звезды: чередование внешнего и внутреннего радиуса."""
    out = []
    for i in range(tips * 2):
        rr = r if i % 2 == 0 else r * 0.45
        a = math.radians(phase - 90.0 + i * 180.0 / tips)
        out.append(cx + rr * math.cos(a))
        out.append(cy + rr * math.sin(a))
    return out


def tri(cx, cy, size, direction):
    """Треугольная стрелка прокрутки: l, r, u, d."""
    s = size
    if direction == "l":
        return [cx + s, cy - s, cx + s, cy + s, cx - s, cy]
    if direction == "r":
        return [cx - s, cy - s, cx - s, cy + s, cx + s, cy]
    if direction == "u":
        return [cx - s, cy + s, cx + s, cy + s, cx, cy - s]
    return [cx - s, cy - s, cx + s, cy - s, cx, cy + s]


def main():
    win = ssui.W("SSUI · poly", 1080, 720, thm="drk")

    hit = ssui.sgnl(-1)
    checked = ssui.sgnl(True)
    stroke = ssui.sgnl(0.0)
    tips = ssui.sgnl(5)
    spin = ssui.sgnl(0.0)
    window = ssui.sgnl(0)
    # Окно отображаемых элементов массива — им управляют стрелки.

    NAMES = ["галочка", "стрелка влево", "стрелка вправо",
             "стрелка вверх", "стрелка вниз", "звезда",
             "самопересечение"]

    def col(i):
        return HOT if hit() == i else BASE

    def shapes():
        st = stroke()
        out = []

        # 0 — галочка флага. Смещена в рамку, как в списке параметров.
        pts = list(CHECK)
        if checked():
            out.append(("poly", pts + [st], OK if hit() != 0 else HOT, ""))
        else:
            out.append(("poly", pts + [1.0], DIM, ""))

        # 1..4 — стрелки прокрутки массива.
        out.append(("poly", tri(200.0, 60.0, 16.0, "l") + [st], col(1), ""))
        out.append(("poly", tri(260.0, 60.0, 16.0, "r") + [st], col(2), ""))
        out.append(("poly", tri(200.0, 130.0, 16.0, "u") + [st], col(3), ""))
        out.append(("poly", tri(260.0, 130.0, 16.0, "d") + [st], col(4), ""))

        # 5 — звезда с настраиваемым числом лучей.
        out.append(("poly",
                    star(430.0, 100.0, 70.0, tips(), spin()) + [st],
                    col(5), ""))

        # 6 — самопересекающийся контур: видно правило чётности.
        loop = [560.0, 40.0, 700.0, 170.0, 700.0, 40.0, 560.0, 170.0]
        out.append(("poly", loop + [st], col(6), ""))

        # Подписи под фигурами.
        out.append(("text", [20.0, 200.0, 300.0, 24.0], TXT,
                    f"Окно массива: {window()}"))

        # Полоса элементов массива — то, чем управляют стрелки.
        for i in range(8):
            x = 20.0 + i * 78.0
            idx = window() + i
            out.append(("rect", [x, 240.0, 70.0, 46.0, 6.0, 0.0],
                        "#1E293B", ""))
            out.append(("text", [x + 8.0, 250.0, 60.0, 24.0], TXT,
                        f"#{idx}"))
        return out

    def on_hit(i):
        hit.st(i)
        if i == 0:
            checked.st(not checked())
        elif i == 1:
            window.st(max(0, window() - 1))
        elif i == 2:
            window.st(window() + 1)
        elif i == 3:
            window.st(max(0, window() - 8))
        elif i == 4:
            window.st(window() + 8)

    with win.bx(pr=win.rt(), ax="h", pd=14.0, gp=14.0) as root:
        win.cls(root, "clear")

        with win.bx(pd=12.0, gp=8.0):
            t = win.lb("Кликай по фигурам", h=34.0)
            win.cls(t, "head")
            win.cv([], bind=shapes, ch=on_hit, h=420.0)

        with win.bx(pd=14.0, gp=8.0, w=320.0):
            t2 = win.lb("Параметры", h=34.0)
            win.cls(t2, "head")
            win.sep()

            win.lb(bind=lambda: (
                f"Обводка: {stroke():.1f}" if stroke() > 0
                else "Обводка: нет, заливка"
            ), h=28.0)
            win.sl(0.0, ch=lambda v: stroke.st(v * 8.0), h=34.0)

            win.lb(bind=lambda: f"Лучей звезды: {tips()}", h=26.0)
            win.spin(5, min=3, max=12, step=1,
                     ch=lambda v: tips.st(int(v)), h=44.0)

            win.lb(bind=lambda: f"Поворот: {spin():.0f}°", h=26.0)
            win.sl(0.0, ch=lambda v: spin.st(v * 360.0), h=34.0)

            win.sep()
            win.lb(bind=lambda: (
                "Фигура не выбрана" if hit() < 0 or hit() >= len(NAMES)
                else f"#{hit()}: {NAMES[hit()]}"
            ), h=44.0, wrap=True)
            win.lb(bind=lambda: (
                f"Флаг: {'установлен' if checked() else 'снят'}"), h=28.0)

            win.sep()
            d = win.lb("Клик по галочке переключает флаг, "
                       "по стрелкам — сдвигает окно массива.",
                       h=56.0, wrap=True)
            win.cls(d, "dim")
            d2 = win.lb("У самопересечения при заливке видна дырка: "
                        "работает правило чётности.", h=56.0, wrap=True)
            win.cls(d2, "dim")

    win.css(CSS)
    win.go()


if __name__ == "__main__":
    main()
