"""SSUI — Canvas: прокрутка колесом и панорамирование.

Поле 4000x4000 с 200 фигурами. Колесо прокручивает по вертикали,
Shift+колесо по горизонтали, перетаскивание панорамирует.
Правая канва показывает конфликт с move: там панорамирования нет.

Путь в репозитории: python/examples/cv_scroll.py
"""

import ssui

CSS = """
.clear { background: #00000000; }
.head  { color: #EEF3FF; font-size: 22; }
.dim   { color: #9AA4B2; }
canvas { background: #0B1020; radius: 12; }
"""

W = 4000.0
H = 4000.0
N = 200

COLORS = ["#3B82F6", "#22C55E", "#F59E0B", "#EF4444", "#A855F7"]


def build():
    """200 фигур, разложенных по всему полю."""
    out = []
    for i in range(N):
        col = i % 10
        row = i // 10
        x = 40.0 + col * 390.0
        y = 40.0 + row * 195.0
        c = COLORS[i % len(COLORS)]
        if i % 3 == 0:
            out.append(("rect", [x, y, 300.0, 120.0, 12.0, 0.0], c, ""))
        elif i % 3 == 1:
            out.append(("circle", [x + 150.0, y + 60.0, 60.0, 0.0], c, ""))
        else:
            out.append(("poly",
                        [x, y + 120.0, x + 150.0, y, x + 300.0, y + 120.0,
                         0.0], c, ""))
        out.append(("text", [x + 8.0, y + 130.0, 200.0, 24.0],
                    "#EEF3FF", f"#{i}"))
    return out


SHAPES = build()


def main():
    win = ssui.W("SSUI · прокрутка канвы", 1180, 800, thm="drk")

    hit = ssui.sgnl(-1)
    cx = ssui.sgnl(0.0)
    cy = ssui.sgnl(0.0)
    goto = ssui.sgnl(0)

    # Правая канва: своя логика перетаскивания, панорамирования нет.
    trace = ssui.sgnl("—")
    px = ssui.sgnl(0.0)
    py = ssui.sgnl(0.0)

    def on_hit(i):
        hit.st(i)

    def on_down(i, x, y):
        cx.st(x)
        cy.st(y)

    def jump():
        # Перевод номера фигуры в координаты и прокрутка к ней.
        i = max(0, min(N - 1, int(goto())))
        col = i % 10
        row = i // 10
        x = 40.0 + col * 390.0
        y = 40.0 + row * 195.0
        win.cv_view(board, x - 200.0, y - 200.0)

    def right_shapes():
        # Крестик под курсором: доказательство, что move работает
        # и что канва при этом не панорамируется.
        x = px()
        y = py()
        return [
            ("rect", [0.0, 0.0, 1200.0, 900.0, 0.0, 2.0], "#334155", ""),
            ("line", [x - 20.0, y, x + 20.0, y, 2.0], "#22C55E", ""),
            ("line", [x, y - 20.0, x, y + 20.0, 2.0], "#22C55E", ""),
            ("text", [10.0, 10.0, 400.0, 24.0], "#9AA4B2",
             "move задан: перетаскивание рисует, а не панорамирует"),
            ("text", [x + 12.0, y + 8.0, 260.0, 24.0], "#EEF3FF",
             f"{x:.0f}, {y:.0f}"),
        ]

    def right_move(i, x, y):
        px.st(x)
        py.st(y)
        trace.st(f"move в {x:.0f}, {y:.0f}")

    with win.bx(pr=win.rt(), ax="h", pd=14.0, gp=14.0) as root:
        win.cls(root, "clear")

        with win.bx(pd=12.0, gp=8.0):
            t = win.lb("Поле 4000×4000, 200 фигур", h=34.0)
            win.cls(t, "head")
            board = win.cv(SHAPES, ch=on_hit, down=on_down,
                           scroll=True, h=420.0)
            win.cv_region(board, 0.0, 0.0, W, H)
            # Без cv_region прокрутка никуда не поедет: границы
            # виртуальной области задаются только здесь.

            win.sep()
            t2 = win.lb("Прокручиваемая канва с move", h=30.0)
            win.cls(t2, "head")
            side = win.cv([], bind=right_shapes, move=right_move,
                          scroll=True, h=240.0)
            win.cv_region(side, 0.0, 0.0, 1200.0, 900.0)

        with win.bx(pd=14.0, gp=8.0, w=330.0):
            t3 = win.lb("Управление", h=34.0)
            win.cls(t3, "head")
            win.sep()

            win.lb("Колесо — вертикально.", h=24.0)
            win.lb("Shift+колесо — горизонтально.", h=24.0)
            win.lb("Перетаскивание — панорамирование.", h=24.0)

            win.sep()
            win.lb("Перейти к фигуре:", h=26.0)
            win.spin(0, min=0, max=N - 1, step=1,
                     ch=lambda v: goto.st(int(v)), h=44.0)
            win.bt("Прокрутить к ней", h=46.0, clk=jump)

            win.sep()
            win.lb(bind=lambda: (
                "Фигура не выбрана" if hit() < 0
                else f"Выбрана фигура #{hit()}"
            ), h=28.0)
            win.lb(bind=lambda: (
                f"Точка нажатия: {cx():.0f}, {cy():.0f}"), h=28.0)
            d = win.lb("Координаты — в системе содержимого: "
                       "прокрути и нажми снова, числа останутся "
                       "привязанными к фигурам.", h=76.0, wrap=True)
            win.cls(d, "dim")

            win.sep()
            win.lb(bind=lambda: f"Нижняя канва: {trace()}", h=28.0)
            d2 = win.lb("У нижней канвы задан move, поэтому левая "
                        "кнопка ей и принадлежит. Прокрутка там "
                        "только колесом.", h=76.0, wrap=True)
            win.cls(d2, "dim")

    win.css(CSS)
    win.go()


if __name__ == "__main__":
    main()
