"""SSUI — css() и build() после показа окна.

Демонстрирует:
  • win.css(text, replace=True) — живая смена палитры без перезапуска;
  • сборку CSS из значений ползунков — радиус и оттенок меняются на лету;
  • win.build(f) — достройку дерева после показа: новые слои pop;
  • pop_at / pop_off для созданных на лету слоёв.

Слои создаются кнопкой и сразу становятся рабочими.
"""

import ssui

PALETTES = {
    "Ночь": ("#141c30", "#1b2540", "#eef3ff", "#3b82f6"),
    "Графит": ("#1a1a1a", "#262626", "#e5e5e5", "#a855f7"),
    "Море": ("#0b2027", "#12333d", "#e0fbfc", "#2fbf71"),
    "Песок": ("#241c14", "#3a2d20", "#f5ecd7", "#f59e0b"),
}
NAMES = list(PALETTES)


def main():
    win = ssui.W("SSUI · css и build после показа", 1200, 800, thm="drk",
                 glass=True, tint=0.12)

    pal = ssui.sgnl(0)
    rad = ssui.sgnl(14.0)
    log = ssui.sgnl("—")
    layers = ssui.sgnl(0)
    draft = ssui.sgnl("")

    made = []

    def sheet():
        bg, card, fg, acc = PALETTES[NAMES[pal()]]
        r = rad()
        return f"""
        frame {{ background: {bg}cc; radius: {r:.0f}; }}
        .clear {{ background: #00000000; }}
        label {{ color: {fg}; }}
        .head {{ color: {acc}; }}
        .dim  {{ color: {fg}99; }}
        .card {{ background: {card}; radius: {r:.0f}; }}
        button {{ background: {card}; color: {fg}; radius: {r * 0.6:.0f}; }}
        button:hover {{ background: {acc}; color: {bg}; }}
        textbox {{ background: {card}; color: {fg}; }}
        .layer {{ background: {acc}; radius: {r * 0.6:.0f}; }}
        """

    def repaint(what):
        # replace=True: источник заменяется целиком, а не копится.
        win.css(sheet(), replace=True)
        log.st(what)

    def pick(i):
        pal.st(i)
        repaint(f"палитра: {NAMES[i]}")

    def set_rad(v):
        rad.st(4.0 + v * 26.0)
        repaint(f"радиус: {rad():.0f}")

    def add_layer():
        n = layers() + 1
        layers.st(n)

        def body():
            # Внутри build дерево снова принадлежит окну.
            with win.pop(w=280.0, h=40.0,
                         on_close=lambda: log.st(f"слой {n} закрыт")) as lay:
                win.cls(lay, "layer")
                fld = win.tx("", sig=draft, ph=f"Слой {n}", h=36.0)
                win.keys(fld, lambda c: (win.pop_off(lay),
                                         log.st("записано" if c == 1
                                                else "отменено")))
            made.append(lay)
            # Стили ложатся только на существующие узлы, поэтому
            # палитру после достройки применяем заново.
            win.css(sheet(), replace=True)
            log.st(f"слой {n} создан")

        win.build(body)

    def open_last():
        if not made:
            log.st("слоёв ещё нет")
            return
        win.pop_at(made[-1], 460.0, 320.0, 280.0, 40.0)
        log.st(f"слой {len(made)} показан")

    def close_last():
        if not made:
            log.st("слоёв ещё нет")
            return
        win.pop_off(made[-1])
        log.st(f"слой {len(made)} скрыт")

    with win.bx(pr=win.rt(), ax="h", pd=14.0, gp=14.0) as root:
        win.cls(root, "clear")

        with win.bx(pd=14.0, gp=10.0) as left:
            t = win.lb("Живая палитра", h=30.0)
            win.cls(t, "head")
            with win.bx(pd=12.0, gp=8.0, h=200.0) as demo:
                win.cls(demo, "card")
                win.lb("Карточка перекрашивается целиком", h=28.0)
                win.lb("Радиус и оттенок приходят из CSS", h=28.0)
                win.bt("Обычная кнопка", h=44.0,
                       clk=lambda: log.st("кнопка нажата"))
                win.tx("Поле ввода", h=38.0)

            win.sep()
            t2 = win.lb("Динамические слои", h=30.0)
            win.cls(t2, "head")
            with win.bx(pd=12.0, gp=8.0, h=220.0) as area:
                win.cls(area, "card")
                win.lb(bind=lambda: f"Создано слоёв: {layers()}", h=28.0)
                win.lb(bind=lambda: f"Черновик: {draft() or '—'}", h=28.0)
                win.bt("Создать слой", h=44.0, clk=add_layer)
                with win.bx(ax="h", gp=8.0, h=52.0) as row:
                    win.cls(row, "clear")
                    win.bt("Показать последний", h=44.0, clk=open_last)
                    win.bt("Скрыть", h=44.0, clk=close_last)

        with win.bx(pd=14.0, gp=8.0, w=360.0):
            t3 = win.lb("Управление", h=30.0)
            win.cls(t3, "head")
            win.sep()

            win.lb("Палитра:", h=24.0)
            win.dd(NAMES, ch=pick, h=44.0)

            win.lb(bind=lambda: f"Радиус: {rad():.0f}", h=24.0)
            win.sl(0.38, ch=set_rad, h=32.0)

            win.sep()
            win.bt("Вернуть исходную", h=44.0,
                   clk=lambda: pick(0))
            win.bt("Накопительный css()", h=44.0,
                   clk=lambda: (win.css("label { color: #ef4444; }"),
                                log.st("правило добавлено поверх")))

            win.sep()
            win.lb(bind=lambda: f"Событие: {log()}", h=48.0, wrap=True)
            d = win.lb("replace=True заменяет источник целиком — "
                       "без него правила копятся с каждым кадром.",
                       h=60.0, wrap=True)
            win.cls(d, "dim")
            d2 = win.lb("Слой создаётся уже после показа окна: "
                        "раньше здесь была ошибка «окно уже запущено».",
                        h=60.0, wrap=True)
            win.cls(d2, "dim")

    win.css(sheet(), replace=True)
    win.go()


if __name__ == "__main__":
    main()
