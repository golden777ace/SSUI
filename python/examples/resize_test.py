"""SSUI — событие изменения размера окна.

on_resize приходит на каждый шаг перетаскивания края. Здесь: живой
размер, счётчик событий, полоса пропорциональна ширине, и «устоявшийся»
размер — через дебаунс after (один раз через 200 мс после остановки).

Путь в репозитории: python/examples/resize_test.py
"""

import ssui

CSS = """
.clear { background: #00000000; }
.head  { color: #EEF3FF; font-size: 22; }
.dim   { color: #9AA4B2; }
.ok    { color: #22C55E; }
.warn  { color: #F59E0B; }
"""


def main():
    win = ssui.W("SSUI · изменение размера", 820, 560, thm="drk")

    size = ssui.sgnl((820.0, 560.0))
    events = ssui.sgnl(0)
    settled = ssui.sgnl("ещё не менялся")
    deb = {"id": 0}

    def on_size(w, h):
        size.st((w, h))
        events.st(events() + 1)
        # Дебаунс: тяжёлый пересчёт откладываем до остановки.
        win.cancel(deb["id"])
        deb["id"] = win.after(
            200.0, lambda: settled.st(f"{int(w)} × {int(h)} px"))

    win.on_resize(on_size)

    with win.bx(pr=win.rt(), ax="v", pd=16.0, gp=12.0) as root:
        win.cls(root, "clear")

        t = win.lb("Тяни за край окна", h=34.0)
        win.cls(t, "head")

        a = win.lb(bind=lambda: f"Живой размер: "
                   f"{int(size()[0])} × {int(size()[1])} px", h=30.0)
        win.cls(a, "ok")
        c = win.lb(bind=lambda: f"Событий resize: {events()}", h=28.0)
        s = win.lb(bind=lambda: f"Устоявшийся размер: {settled()}", h=30.0)
        win.cls(s, "warn")

        win.sep()
        win.lb("Полоса ниже пропорциональна ширине окна "
               "(доля от 1920):", h=44.0, wrap=True)
        win.pr(0.0, bind=lambda: min(1.0, size()[0] / 2560.0), h=22.0)

        win.sep()
        win.lb("Событие приходит часто — дебаунс через after "
               "срабатывает один раз после остановки перетаскивания.",
               h=52.0, wrap=True)

    win.css(CSS)
    win.go()


if __name__ == "__main__":
    main()
