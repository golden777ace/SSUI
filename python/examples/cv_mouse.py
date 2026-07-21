"""SSUI — Canvas: нажатие, перетаскивание, отпускание, двойной клик.

Модель суточного профиля: полоса разбита на интервалы, граница
интервала тянется мышью и задаётся долей внутри полосы.

Путь в репозитории: python/examples/cv_mouse.py
"""

import ssui

CSS = """
.clear { background: #00000000; }
.head  { color: #EEF3FF; font-size: 22; }
.dim   { color: #9AA4B2; }
canvas { background: #0B1020; radius: 12; }
"""

BAR_X = 30.0
BAR_Y = 60.0
BAR_W = 620.0
BAR_H = 90.0

PAD_X = 30.0
PAD_Y = 200.0
PAD_W = 620.0
PAD_H = 180.0

COLORS = ["#3B82F6", "#22C55E", "#F59E0B", "#EF4444"]


def main():
    win = ssui.W("SSUI · мышь на канве", 1080, 700, thm="drk")

    # Границы интервалов как доли ширины полосы. Первая и последняя
    # точки подразумеваются: 0.0 слева и 1.0 справа.
    marks = ssui.sgnl([0.25, 0.5, 0.75])

    grab = ssui.sgnl(-1)
    # Индекс границы, которую сейчас тянут. -1 — не тянут.

    fx = ssui.sgnl(0.0)
    fy = ssui.sgnl(0.0)
    phase = ssui.sgnl("—")
    hits = ssui.sgnl(0)
    dbls = ssui.sgnl(0)

    def bounds():
        return [0.0] + list(marks()) + [1.0]

    def shapes():
        out = []
        b = bounds()

        # Интервалы профиля. Индексы фигур с 0 по len(b)-2.
        for i in range(len(b) - 1):
            x1 = BAR_X + b[i] * BAR_W
            x2 = BAR_X + b[i + 1] * BAR_W
            out.append(("rect", [x1, BAR_Y, x2 - x1, BAR_H, 0.0, 0.0],
                        COLORS[i % len(COLORS)], ""))

        # Ручки границ. Идут сразу после интервалов, это используется
        # при разборе индекса в on_down.
        for i, m in enumerate(marks()):
            x = BAR_X + m * BAR_W
            wide = 12.0 if grab() == i else 8.0
            out.append(("rect",
                        [x - wide / 2.0, BAR_Y - 10.0, wide, BAR_H + 20.0,
                         3.0, 0.0],
                        "#EEF3FF" if grab() == i else "#94A3B8", ""))

        # Подписи времени под границами.
        for m in marks():
            x = BAR_X + m * BAR_W
            hh = int(m * 24.0)
            mm = int((m * 24.0 - hh) * 60.0)
            out.append(("text", [x - 30.0, BAR_Y + BAR_H + 8.0, 60.0, 22.0],
                        "#9AA4B2", f"{hh:02d}:{mm:02d}"))

        # Свободная площадка: показывает долю по двум осям сразу.
        out.append(("rect", [PAD_X, PAD_Y, PAD_W, PAD_H, 10.0, 2.0],
                    "#4B5563", ""))
        cx = PAD_X + fx() * PAD_W
        cy = PAD_Y + fy() * PAD_H
        out.append(("line", [cx, PAD_Y, cx, PAD_Y + PAD_H, 1.0], "#334155", ""))
        out.append(("line", [PAD_X, cy, PAD_X + PAD_W, cy, 1.0], "#334155", ""))
        out.append(("circle", [cx, cy, 8.0, 0.0], "#22C55E", ""))
        return out

    def on_down(i, x, y):
        phase.st("down")
        hits.st(hits() + 1)
        n = len(bounds()) - 1
        # Ручки лежат в фигурах с индексами n .. n+len(marks)-1.
        if n <= i < n + len(marks()):
            grab.st(i - n)
        else:
            grab.st(-1)
        track(x, y)

    def on_move(i, x, y):
        phase.st("move")
        k = grab()
        if k >= 0:
            # Доля внутри полосы — та самая формула из профиля:
            # (p.x - x1) / (x2 - x1).
            t = (x - BAR_X) / BAR_W
            b = bounds()
            lo = b[k] + 0.01
            hi = b[k + 2] - 0.01
            # Границы не проходят друг сквозь друга.
            cur = list(marks())
            cur[k] = min(max(t, lo), hi)
            marks.st(cur)
        track(x, y)

    def on_up(i, x, y):
        phase.st("up")
        grab.st(-1)
        track(x, y)

    def on_dbl(i, x, y):
        dbls.st(dbls() + 1)
        phase.st("двойной")
        # Двойной клик по площадке ставит точку в центр.
        if PAD_X <= x <= PAD_X + PAD_W and PAD_Y <= y <= PAD_Y + PAD_H:
            fx.st(0.5)
            fy.st(0.5)

    def track(x, y):
        # Доля по обеим осям внутри свободной площадки.
        if PAD_X <= x <= PAD_X + PAD_W and PAD_Y <= y <= PAD_Y + PAD_H:
            fx.st((x - PAD_X) / PAD_W)
            fy.st((y - PAD_Y) / PAD_H)

    with win.bx(pr=win.rt(), ax="h", pd=14.0, gp=14.0) as root:
        win.cls(root, "clear")

        with win.bx(pd=12.0, gp=8.0):
            t = win.lb("Тяни границы интервалов", h=34.0)
            win.cls(t, "head")
            win.cv([], bind=shapes,
                   down=on_down, move=on_move, up=on_up, dbl=on_dbl,
                   h=440.0)

        with win.bx(pd=14.0, gp=8.0, w=320.0):
            t2 = win.lb("События", h=34.0)
            win.cls(t2, "head")
            win.sep()
            win.lb(bind=lambda: f"Фаза: {phase()}", h=28.0)
            win.lb(bind=lambda: f"Нажатий: {hits()}", h=26.0)
            win.lb(bind=lambda: f"Двойных: {dbls()}", h=26.0)
            win.lb(bind=lambda: (
                "Ничего не тянем" if grab() < 0
                else f"Тянем границу #{grab()}"
            ), h=26.0)

            win.sep()
            t3 = win.lb("Доли на площадке", h=30.0)
            win.cls(t3, "head")
            win.lb(bind=lambda: f"x = {fx():.3f}", h=28.0)
            win.lb(bind=lambda: f"y = {fy():.3f}", h=28.0)
            win.pr(0.0, bind=fx, h=16.0)
            win.pr(0.0, bind=fy, h=16.0)

            win.sep()
            win.lb(bind=lambda: "Границы: " + ", ".join(
                f"{m:.3f}" for m in marks()), h=48.0, wrap=True)

            d = win.lb("Отпусти кнопку за пределами области — "
                       "фаза всё равно станет up.", h=56.0, wrap=True)
            win.cls(d, "dim")

    win.css(CSS)
    win.go()


if __name__ == "__main__":
    main()
