import ssui


def main():
    vol = ssui.sgnl(0.0)

    win = ssui.W("SSUI Анимации", 800, 640, thm="drk")
    fx = win.fx()

    with win.bx(rad=16.0, pd=24.0, gp=14.0) as panel:
        win.lb(bind=lambda: f"Громкость: {int(vol() * 100)}%", h=36.0)
        win.pr(bind=lambda: vol(), w=300.0, h=24.0)
        win.bt("В 100%", w=200.0, h=44.0, clk=lambda: fx(vol, 1.0, dur=0.5, ease="out"))
        win.bt("В 0%", w=200.0, h=44.0, clk=lambda: fx(vol, 0.0, dur=0.5, ease="io"))
        win.lb(txt="F12 — инспектор раскладки", h=24.0)

    fx(vol, 0.6, dur=1.0, ease="out")

    win.go()


if __name__ == "__main__":
    main()