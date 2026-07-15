import ssui

# Клик по панели поднимает её поверх соседей (как окна).
# Панели помечены win.front(...). Клацай по перекрытиям.

win = ssui.W("SSUI — подъём кликом", 760, 560, thm="drk")

with win.bx(pd=20.0, gp=12.0):
    win.lb(txt="Клик по панели поднимает её наверх.", h=28.0)

    with win.bx(rad=12.0, w=720.0, h=420.0):
        a = win.bt("Панель A", toast="Наверху: A")
        b = win.bt("Панель B", toast="Наверху: B")
        c = win.bt("Панель C", toast="Наверху: C")

        win.pl(a, x=60.0, y=50.0, w=280.0, h=200.0)
        win.pl(b, x=200.0, y=120.0, w=280.0, h=200.0)
        win.pl(c, x=340.0, y=190.0, w=280.0, h=200.0)

        win.dep(a, 0)
        win.dep(b, 1)
        win.dep(c, 2)

        for p in (a, b, c):
            win.front(p)

        win.cls(a, "pa")
        win.cls(b, "pb")
        win.cls(c, "pc")

win.css("""
.pa { background: #3B82F6; color: #FFFFFF; }
.pb { background: #2FBF71; color: #FFFFFF; }
.pc { background: #E5484D; color: #FFFFFF; }
""")

win.go()