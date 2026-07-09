import ssui

CSS = """
.v  { gradient: #3b82f6 #1e3a8a v;  radius: 16; color: #ffffff; }
.h  { gradient: #22c55e #15803d h;  radius: 16; color: #ffffff; }
.d  { gradient: #f59e0b #7c2d12 d;  radius: 16; color: #ffffff; }
.du { gradient: #ec4899 #831843 du; radius: 16; color: #ffffff; }
"""


def panel(win, cls, text):
    with win.bx(rad=16.0, pd=18.0, gp=6.0, h=120.0) as p:
        win.cls(p, cls)
        win.lb(txt=text, h=26.0)


def main():
    win = ssui.W("SSUI · направления градиента", 660, 520, thm="drk")

    with win.bx(pd=24.0, gp=16.0):
        with win.bx(ax="h", gp=16.0):
            panel(win, "v", "Вертикаль ↓")
            panel(win, "h", "Горизонталь →")
        with win.bx(ax="h", gp=16.0):
            panel(win, "d", "Диагональ ↘")
            panel(win, "du", "Диагональ ↗")

        cta = win.bt("Кнопка-градиент", h=52.0)
        win.cls(cta, "h")

    win.css(CSS)
    win.go()


if __name__ == "__main__":
    main()