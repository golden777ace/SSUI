import ssui


def main():
    text = ssui.sgnl("")

    win = ssui.W("SSUI · IME", 700, 360, thm="drk")

    with win.bx(rad=16.0, pd=24.0, gp=14.0):
        win.lb(txt="Введите текст (в т.ч. через IME):", h=28.0)
        win.tx(sig=text, h=46.0)
        win.lb(bind=lambda: f"Вы ввели: {text()}", h=30.0)

    win.go()


if __name__ == "__main__":
    main()