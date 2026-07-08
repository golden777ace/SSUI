import ssui


def main():
    vol = ssui.sgnl(0.0)
    fruit = ssui.sgnl("яблоко")
    opts = ["яблоко", "банан", "вишня", "груша"]
    tabname = ssui.sgnl("Управление")

    win = ssui.W("SSUI Вкладки", 860, 720, thm="drk")
    fx = win.fx()

    with win.bx(rad=16.0, pd=20.0, gp=12.0):
        win.lb(bind=lambda: f"Активная вкладка: {tabname()}", h=28.0)

        with win.tab(
            ["Управление", "Список"],
            ch=lambda i: tabname.st(["Управление", "Список"][i]),
            h=360.0,
        ):
            with win.bx(rad=12.0, pd=18.0, gp=12.0):
                win.lb(bind=lambda: f"Громкость: {int(vol() * 100)}%", h=32.0)
                win.pr(bind=lambda: vol(), w=320.0, h=22.0)
                win.bt("100%", w=160.0, h=42.0, clk=lambda: fx(vol, 1.0, dur=0.5))
                win.bt("0%", w=160.0, h=42.0, clk=lambda: fx(vol, 0.0, dur=0.5, ease="io"))
            with win.bx(rad=12.0, pd=18.0, gp=12.0):
                win.lb(bind=lambda: f"Фрукт: {fruit()}", h=32.0)
                win.dd(opts, sel=0, ch=lambda i: fruit.st(opts[i]), w=320.0, h=44.0)

        win.lb(txt="Tab — фокус · Enter/Space · стрелки · F12", h=24.0)

    fx(vol, 0.6, dur=1.0)

    win.go()


if __name__ == "__main__":
    main()