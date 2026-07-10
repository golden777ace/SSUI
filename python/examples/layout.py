import ssui


def main():
    win = ssui.W("SSUI — раскладка", 760, 640, thm="drk")

    with win.bx(rad=16.0, pd=16.0, gp=16.0):

        win.lb("justify: st / cnt / end / btw", h=22.0)
        for j in ["st", "cnt", "end", "btw"]:
            with win.bx(rad=10.0, ax="h", gp=8.0, pd=8.0, h=56.0) as row:
                win.align(row, justify=j, cross="cnt")
                for c in ["A", "B", "C"]:
                    win.bt(c, w=70.0, h=40.0)

        win.lb("grow: веса 1 / 2 / 1", h=22.0)
        with win.bx(rad=10.0, ax="h", gp=8.0, pd=8.0, h=56.0):
            a = win.bt("1", h=40.0)
            b = win.bt("2", h=40.0)
            c = win.bt("1", h=40.0)
            win.grow(a, 1.0)
            win.grow(b, 2.0)
            win.grow(c, 1.0)

        win.lb("pin: кнопка в правом нижнем углу", h=22.0)
        with win.bx(rad=10.0, pd=8.0, h=120.0) as panel:
            badge = win.bt("+", pr=win.rt(), w=48.0, h=48.0)
            win.pin(badge, r=16.0, b=16.0)

    win.go()


if __name__ == "__main__":
    main()