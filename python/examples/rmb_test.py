"""SSUI — правый клик по конкретному узлу.

У каждой карточки свой обработчик rmb с координатами. Правый клик
по фону открывает контекстное меню окна из menu(). Клик по карточке
меню не открывает — обработчик узла перехватывает событие.

Путь в репозитории: python/examples/rmb_test.py
"""

import ssui

CSS = """
.clear { background: #00000000; }
.head  { color: #EEF3FF; font-size: 22; }
.dim   { color: #9AA4B2; }
.ok    { color: #22C55E; }
.card  { background: #16213A; radius: 12; }
"""

CARDS = ["Профиль A", "Профиль B", "Профиль C"]


def main():
    win = ssui.W("SSUI · правый клик по узлу", 900, 620, thm="drk")

    log = ssui.sgnl("—")
    src = ssui.sgnl("—")

    def on_menu(i):
        items = ["Обновить всё", "Экспорт", "Настройки"]
        log.st(f"меню окна: {items[i]}")
        src.st("фон")

    win.menu(["Обновить всё", "Экспорт", "Настройки"], on_select=on_menu)

    def card_rmb(name):
        def f(x, y):
            log.st(f"{name}: правый клик ({x:.0f}, {y:.0f})")
            src.st(name)
        return f

    with win.bx(pr=win.rt(), ax="v", pd=16.0, gp=12.0) as root:
        win.cls(root, "clear")

        t = win.lb("Правый клик: по карточке — её меню, "
                   "по фону — меню окна", h=34.0)
        win.cls(t, "head")

        with win.bx(ax="h", gp=12.0, h=180.0) as row:
            win.cls(row, "clear")
            for name in CARDS:
                with win.bx(pd=14.0, gp=6.0) as card:
                    win.cls(card, "card")
                    c = win.lb(name, h=32.0)
                    win.cls(c, "head")
                    win.lb("Кликни правой кнопкой", h=28.0)
                    win.lb("внутри этой карточки", h=28.0)
                win.rmb(card, card_rmb(name))

        win.sep()
        a = win.lb(bind=lambda: f"Последнее событие: {log()}",
                   h=40.0, wrap=True)
        win.cls(a, "ok")
        s = win.lb(bind=lambda: f"Источник: {src()}", h=28.0)
        win.cls(s, "dim")

        win.sep()
        win.lb("Правый клик по свободному месту окна (не по карточке) "
               "откроет контекстное меню из menu().", h=48.0, wrap=True)

    win.css(CSS)
    win.go()


if __name__ == "__main__":
    main()
