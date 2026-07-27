"""SSUI — show(node, on): скрытие с исключением из раскладки.

Демонстрирует:
  • win.show(node, False) — узел уходит из потока, место отдаётся соседям;
  • разницу с win.ghost(node, True) — тот лишь отключает мышь;
  • работу в вертикальном потоке, горизонтальном ряду, сетке и scr;
  • скрытие контейнера целиком вместе с потомками.

Переключатели слева управляют видимостью, ползунки — зазором.
"""

import ssui

CSS = """
frame { background: #141c30cc; radius: 14; }
.clear { background: #00000000; }
label { color: #eef3ff; }
.head { color: #3b82f6; }
.dim  { color: #94a3b8; }
.cell { background: #1b2540; radius: 10; color: #eef3ff; }
.gh   { background: #3f2d1e; radius: 10; color: #f59e0b; }
"""

ROWS = ["Адрес", "Пароль", "Скорость", "Чётность", "Таймаут"]
CELLS = ["A1", "B2", "C3", "D4", "E5", "F6"]


def main():
    win = ssui.W("SSUI · show(node, on)", 1280, 800, thm="drk",
                 glass=True, tint=0.12)

    gap = ssui.sgnl(8.0)
    pad = ssui.sgnl(12.0)
    log = ssui.sgnl("—")
    vis = {}

    rows = {}
    cells = {}

    def toggle(name, node):
        def cb(v):
            vis[name] = v
            win.show(node, v)
            log.st(f"{name} — {'показан' if v else 'скрыт'}")
        return cb

    with win.bx(pr=win.rt(), ax="h", pd=14.0, gp=14.0) as root:
        win.cls(root, "clear")

        with win.bx(pd=12.0, gp=10.0) as mid:
            win.bindb(mid, lambda: (pad(), gap()))

            t = win.lb("Вертикальный поток", h=28.0)
            win.cls(t, "head")
            for nm in ROWS:
                r = win.lb(f"{nm}: значение", h=36.0)
                win.cls(r, "cell")
                rows[nm] = r
                vis[nm] = True

            win.sep()
            t2 = win.lb("Горизонтальный ряд", h=28.0)
            win.cls(t2, "head")
            with win.bx(ax="h", gp=8.0, h=54.0) as line:
                win.cls(line, "clear")
                for nm in CELLS:
                    c = win.bt(nm, h=46.0,
                               clk=lambda n=nm: log.st(f"клик {n}"))
                    cells[nm] = c
                    vis[nm] = True

            win.sep()
            t3 = win.lb("Контейнер целиком", h=28.0)
            win.cls(t3, "head")
            with win.bx(pd=10.0, gp=6.0, h=120.0) as block:
                win.cls(block, "cell")
                win.lb("Внутри блока — три метки", h=28.0)
                win.lb("Скрытие уносит и потомков", h=28.0)
                win.bt("Кнопка блока", h=40.0,
                       clk=lambda: log.st("кнопка блока"))
            vis["Блок"] = True

            win.sep()
            t4 = win.lb("Сравнение: ghost", h=28.0)
            win.cls(t4, "head")
            with win.bx(ax="h", gp=8.0, h=54.0) as gline:
                win.cls(gline, "clear")
                ghosted = win.bt("ghost", h=46.0,
                                 clk=lambda: log.st("ghost кликнут"))
                win.cls(ghosted, "gh")
                win.bt("сосед", h=46.0,
                       clk=lambda: log.st("сосед кликнут"))

        with win.bx(pd=12.0, gp=8.0, w=380.0):
            t5 = win.lb("Видимость", h=28.0)
            win.cls(t5, "head")
            win.sep()

            win.lb("Строки потока:", h=24.0)
            for nm in ROWS:
                win.sw(nm, on=True, h=34.0, clk=toggle(nm, rows[nm]))

            win.lb("Ячейки ряда:", h=24.0)
            with win.bx(ax="h", gp=6.0, h=40.0) as g1:
                win.cls(g1, "clear")
                for nm in CELLS[:3]:
                    win.tgl(nm, on=True, h=34.0,
                            clk=lambda v, n=nm: (
                                win.show(cells[n], v),
                                log.st(f"{n} — {'показан' if v else 'скрыт'}"),
                            ))
            with win.bx(ax="h", gp=6.0, h=40.0) as g2:
                win.cls(g2, "clear")
                for nm in CELLS[3:]:
                    win.tgl(nm, on=True, h=34.0,
                            clk=lambda v, n=nm: (
                                win.show(cells[n], v),
                                log.st(f"{n} — {'показан' if v else 'скрыт'}"),
                            ))

            win.sep()
            win.sw("Блок целиком", on=True, h=36.0,
                   clk=toggle("Блок", block))
            win.sw("ghost на оранжевой", h=36.0,
                   clk=lambda v: (win.ghost(ghosted, v),
                                  log.st(f"ghost {'включён' if v else 'снят'}")))

            win.sep()
            win.lb(bind=lambda: f"Отступ: {pad():.0f}", h=24.0)
            win.sl(0.5, ch=lambda v: pad.st(v * 24.0), h=32.0)
            win.lb(bind=lambda: f"Зазор: {gap():.0f}", h=24.0)
            win.sl(0.33, ch=lambda v: gap.st(v * 24.0), h=32.0)

            win.sep()
            win.lb(bind=lambda: f"Событие: {log()}", h=48.0, wrap=True)
            win.lb(bind=lambda: (
                "Видимых элементов: "
                f"{sum(1 for v in vis.values() if v)} из {len(vis)}"), h=26.0)

            d = win.lb("show убирает место, ghost — только мышь: "
                       "оранжевая кнопка остаётся на экране.",
                       h=60.0, wrap=True)
            win.cls(d, "dim")

    win.css(CSS)
    win.go()


if __name__ == "__main__":
    main()
