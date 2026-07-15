import ssui

# Слайдер задаёт глубину z красной панели (0..2).
# При z=0 она уходит под синие; при z=2 — поверх них.

zsig = ssui.sgnl(2)  # текущая глубина движущейся панели

win = ssui.W("SSUI — глубина (z)", 760, 560, thm="drk")

with win.bx(pd=20.0, gp=12.0):
    win.lb(bind=lambda: f"Глубина красной панели: z = {zsig()}", h=28.0)
    win.sl(1.0, ch=lambda v: zsig.st(int(round(v * 2))), h=40.0)
    win.lb(txt="Двигай слайдер. Клик уходит верхней панели.", h=26.0)

    with win.bx(rad=12.0, w=720.0, h=380.0):
        a = win.bt("A · z = 1", toast="Клик: A (z=1)")
        b = win.bt("B · z = 1", toast="Клик: B (z=1)")
        mover = win.bt("MOVER", toast="Клик: MOVER")

        win.pl(a, x=80.0, y=50.0, w=260.0, h=170.0)
        win.pl(b, x=380.0, y=50.0, w=260.0, h=170.0)
        win.pl(mover, x=230.0, y=130.0, w=260.0, h=170.0)

        win.dep(a, 1)
        win.dep(b, 1)
        win.bindz(mover, lambda: zsig())

        win.cls(a, "ref")
        win.cls(b, "ref")
        win.cls(mover, "mover")

win.css("""
.ref   { background: #3B82F6; color: #FFFFFF; }
.mover { background: #E5484D; color: #FFFFFF; }
""")

win.go()