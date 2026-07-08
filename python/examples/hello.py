import ssui


def main():
    clicks = ssui.sgnl(0)
    name = ssui.sgnl("")

    win = ssui.W("Hello SSUI", 800, 640, thm="drk")

    with win.bx(rad=16.0, pd=24.0, gp=12.0) as panel:
        win.lb(bind=lambda: f"Кликов: {clicks()}", h=40.0)
        win.bt("Нажми меня", w=200.0, h=48.0, clk=lambda: clicks.st(clicks() + 1))

        win.lb(txt="Введите имя:", h=24.0)
        win.tx(sig=name, w=280.0, h=44.0)
        win.lb(bind=lambda: f"Привет, {name()}!", h=32.0)

    win.go()


if __name__ == "__main__":
    main()