import ssui

CSS = """
.hero { gradient: #3b82f6 #1e3a8a; radius: 18; shadow: 16; }
.cta  { gradient: #22c55e #15803d; radius: 22; color: #ffffff; }
"""


def main():
    win = ssui.W("SSUI · градиенты", 620, 440, thm="drk")

    with win.bx(pd=32.0, gp=20.0):
        with win.bx(rad=18.0, pd=24.0, gp=12.0, elev=16.0) as hero:
            win.cls(hero, "hero")
            win.lb(txt="Панель с градиентом и тенью", h=30.0)
            win.lb(txt="Заливка сверху вниз двумя цветами", h=26.0)

        cta = win.bt("Градиентная кнопка", h=52.0)
        win.cls(cta, "cta")

    win.css(CSS)

    win.go()


if __name__ == "__main__":
    main()