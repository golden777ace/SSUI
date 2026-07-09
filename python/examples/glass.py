import ssui

CSS = """
.clear { background: #00000000; padding: 44; gap: 0; }
.glass { background: #16213ab3; radius: 24; padding: 26; gap: 14; shadow: 30; }
label { color: #eaf1ff; }
"""


def main():
    op = ssui.sgnl(0.15)  # альфа фона: 0 — прозрачно, 1 — плотно

    win = ssui.W("SSUI · прозрачность", 760, 460, thm="drk", glass=True, tint=0.15)
    win.tint(op)  # фон окна следует за сигналом op

    with win.bx(pd=0.0) as clear:
        win.cls(clear, "clear")

        with win.bx(rad=24.0, pd=26.0, gp=14.0) as card:
            win.cls(card, "glass")

            win.lb(txt="Прозрачность фона — слайдером", h=34.0)
            win.lb(bind=lambda: f"Плотность фона: {int(op() * 100)}%", h=28.0)
            win.sl(0.15, ch=lambda v: op.st(v), h=40.0)
            win.lb(txt="Двигай слайдер — фон окна меняется вживую.", h=26.0)

    win.css(CSS)
    win.go()


if __name__ == "__main__":
    main()