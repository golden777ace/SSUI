import ssui


def main():
    clicks = ssui.sgnl(0)
    volume = ssui.sgnl(0.5)
    win = ssui.W("Hello SSUI", 800, 600, thm="drk")
    root = win.rt()
    panel = win.fr(root, rad=16.0, pd=24.0, gp=12.0)
    win.lb(panel, bind=lambda: f"Кликов: {clicks()}", h=40.0)
    win.bt(panel, "Нажми меня", w=200.0, h=48.0, clk=lambda: clicks.st(clicks() + 1))
    win.lb(panel, bind=lambda: f"Громкость: {int(volume() * 100)}%", h=32.0)
    win.sl(panel, vl=0.5, ch=lambda v: volume.st(v), w=280.0, h=40.0)
    win.pr(panel, bind=lambda: volume(), w=280.0, h=24.0)
    win.ch(panel, "Включить звук", chk=True, h=28.0)
    win.go()


if __name__ == "__main__":
    main()