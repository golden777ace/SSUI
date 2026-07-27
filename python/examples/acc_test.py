"""SSUI — аккордеон: колбэк, программное управление, группы.

Демонстрирует:
  • acc(ch=...) — колбэк раскрытия/сворачивания секции;
  • win.acc_open(node, on) — открытие и закрытие из кода;
  • acc(grp=N) — группа взаимного исключения;
  • bindb — живая правка отступа и зазора секции.

Левая колонка — группа 1: открытой остаётся ровно одна секция.
Средняя колонка — grp=0: секции независимы.
"""

import ssui

CSS = """
frame { background: #141c30cc; radius: 14; }
.clear { background: #00000000; }
label { color: #eef3ff; }
.head { color: #3b82f6; }
.dim  { color: #94a3b8; }
accordion { background: #1b2540; color: #eef3ff; radius: 10; }
accordion:hover { background: #24314f; }
"""

GROUPED = ["Соединение", "Тарифы", "Журнал"]
FREE = ["Отладка", "Экспорт"]


def main():
    win = ssui.W("SSUI · Аккордеон", 1240, 780, thm="drk",
                 glass=True, tint=0.12)

    log = ssui.sgnl("—")
    opened = ssui.sgnl("нет")
    hits = ssui.sgnl(0)
    pad = ssui.sgnl(8.0)
    gap = ssui.sgnl(8.0)

    nodes = {}
    state = {}

    def on(name):
        def cb(v):
            state[name] = v
            hits.st(hits() + 1)
            log.st(f"{name} — {'открыта' if v else 'закрыта'}")
            live = [k for k, s in state.items() if s]
            opened.st(", ".join(live) if live else "нет")
        return cb

    def section(name, group):
        state[name] = False
        with win.acc(name, grp=group, ch=on(name), h=190.0) as a:
            nodes[name] = a
            win.bindb(a, lambda: (pad(), gap()))
            win.lb(f"Содержимое секции «{name}»", h=30.0)
            win.bt("Действие", h=40.0,
                   clk=lambda n=name: log.st(f"кнопка в «{n}»"))
            d = win.lb("Секция помнит своё состояние.", h=30.0)
            win.cls(d, "dim")

    with win.bx(pr=win.rt(), ax="h", pd=14.0, gp=14.0) as root:
        win.cls(root, "clear")

        with win.bx(pd=12.0, gp=10.0):
            t = win.lb("Группа 1 · взаимное исключение", h=28.0)
            win.cls(t, "head")
            for nm in GROUPED:
                section(nm, 1)

        with win.bx(pd=12.0, gp=10.0):
            t2 = win.lb("Без группы · независимые", h=28.0)
            win.cls(t2, "head")
            for nm in FREE:
                section(nm, 0)

        with win.bx(pd=12.0, gp=8.0, w=360.0):
            t3 = win.lb("Управление", h=28.0)
            win.cls(t3, "head")
            win.sep()

            win.lb("Открыть секцию из кода:", h=24.0)
            names = GROUPED + FREE
            win.dd(names, h=44.0,
                   ch=lambda i: win.acc_open(nodes[names[i]], True))

            win.lb("Закрыть секцию из кода:", h=24.0)
            win.dd(names, h=44.0,
                   ch=lambda i: win.acc_open(nodes[names[i]], False))

            with win.bx(ax="h", gp=8.0, h=52.0) as row:
                win.cls(row, "clear")
                win.bt("Открыть все", h=44.0,
                       clk=lambda: [win.acc_open(nodes[n], True)
                                    for n in names])
                win.bt("Закрыть все", h=44.0,
                       clk=lambda: [win.acc_open(nodes[n], False)
                                    for n in names])

            win.sep()
            win.lb(bind=lambda: f"Отступ секции: {pad():.0f}", h=24.0)
            win.sl(0.33, ch=lambda v: pad.st(v * 24.0), h=32.0)
            win.lb(bind=lambda: f"Зазор секции: {gap():.0f}", h=24.0)
            win.sl(0.33, ch=lambda v: gap.st(v * 24.0), h=32.0)

            win.sep()
            t4 = win.lb("Колбэк ch", h=26.0)
            win.cls(t4, "head")
            win.lb(bind=lambda: f"Последнее: {log()}", h=28.0)
            win.lb(bind=lambda: f"Открыты: {opened()}", h=48.0, wrap=True)
            win.lb(bind=lambda: f"Событий ch: {hits()}", h=26.0)
            d2 = win.lb("«Открыть все» в группе 1 оставит одну "
                        "секцию: остальным придёт ch(False).",
                        h=60.0, wrap=True)
            win.cls(d2, "dim")

    win.css(CSS)
    win.go()


if __name__ == "__main__":
    main()
