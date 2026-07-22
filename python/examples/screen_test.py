"""SSUI — размер экрана, размер окна, перемещение.

screen() — размер монитора, size() — клиентская область окна,
move() — позиция левого верхнего угла. Кнопки раскладывают окно
по углам и центру; size обновляется таймером, видно при ресайзе.

Путь в репозитории: python/examples/screen_test.py
"""

import ssui


def main():
    win = ssui.W("SSUI · экран и окно", 720, 500, thm="drk")

    sw, sh = ssui.W.screen()
    size = ssui.sgnl((0.0, 0.0))
    where = ssui.sgnl("—")

    def refresh():
        size.st(win.size())

    win.every(300.0, refresh)

    def to_center():
        w, h = win.size()
        win.move((sw - w) / 2, (sh - h) / 2)
        where.st("центр")

    def to_tl():
        win.move(20.0, 20.0)
        where.st("левый верх")

    def to_tr():
        w, _ = win.size()
        win.move(sw - w - 20.0, 20.0)
        where.st("правый верх")

    def to_bl():
        _, h = win.size()
        win.move(20.0, sh - h - 60.0)
        where.st("левый низ")

    CSS = """
    .clear { background: #00000000; }
    .head  { color: #EEF3FF; font-size: 22; }
    .dim   { color: #9AA4B2; }
    .ok    { color: #22C55E; }
    """

    with win.bx(pr=win.rt(), ax="v", pd=16.0, gp=12.0) as root:
        win.cls(root, "clear")

        t = win.lb("Геометрия экрана и окна", h=34.0)
        win.cls(t, "head")

        e = win.lb(f"Экран: {int(sw)} × {int(sh)} px", h=30.0)
        win.cls(e, "ok")
        s = win.lb(bind=lambda: f"Окно (клиент): "
                   f"{int(size()[0])} × {int(size()[1])} px", h=30.0)
        win.cls(s, "ok")

        win.sep()
        win.lb("Переместить окно:", h=28.0)
        with win.bx(ax="h", gp=10.0, h=52.0) as bar:
            win.cls(bar, "clear")
            win.bt("Центр", h=48.0, clk=to_center)
            win.bt("Левый верх", h=48.0, clk=to_tl)
            win.bt("Правый верх", h=48.0, clk=to_tr)
            win.bt("Левый низ", h=48.0, clk=to_bl)

        w = win.lb(bind=lambda: f"Последняя позиция: {where()}", h=28.0)
        win.cls(w, "dim")

        win.sep()
        win.lb("Правый край считается по клиентской ширине, поэтому "
               "рамка окна может немного выходить за экран.",
               h=48.0, wrap=True)

    win.css(CSS)
    win.go()


if __name__ == "__main__":
    main()
