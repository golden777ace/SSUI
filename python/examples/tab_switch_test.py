"""SSUI — программное переключение вкладок через bindv.

Активная вкладка связана с сигналом: кнопки «Пред/След», выпадающий
список и клик по самой вкладке меняют один и тот же сигнал, а bindv
переключает tab. Клик по вкладке дополнительно шлёт ch.

Путь в репозитории: python/examples/tab_switch_test.py
"""

import ssui

CSS = """
.clear { background: #00000000; }
.head  { color: #EEF3FF; font-size: 22; }
.dim   { color: #9AA4B2; }
.ok    { color: #22C55E; }
"""

NAMES = ["Общие", "Сеть", "Тарифы", "Журнал"]


def main():
    win = ssui.W("SSUI · переключение вкладок", 860, 560, thm="drk")

    page = ssui.sgnl(0)
    src = ssui.sgnl("старт")

    def go(i):
        page.st(max(0, min(len(NAMES) - 1, i)))

    with win.bx(pr=win.rt(), ax="v", pd=16.0, gp=12.0) as root:
        win.cls(root, "clear")

        t = win.lb("Вкладка = функция сигнала", h=34.0)
        win.cls(t, "head")

        with win.bx(ax="h", gp=10.0, h=52.0) as bar:
            win.cls(bar, "clear")
            win.bt("← Пред", h=48.0, clk=lambda: go(page() - 1))
            win.bt("След →", h=48.0, clk=lambda: go(page() + 1))
            win.dd(NAMES, sel=0, w=200.0, h=48.0,
                   ch=lambda i: (go(i), src.st("список")))

        def click(i):
            # Клик по самой вкладке — тоже через ch.
            page.st(i)
            src.st("клик по вкладке")

        with win.tab(NAMES, sel=0, ch=click, h=300.0) as tabs:
            with win.bx(pr=tabs, pd=14.0, gp=8.0):
                win.lb("Общие параметры прибора", h=32.0)
                win.lb("Серийный номер, версия ПО, время.", h=28.0)
            with win.bx(pr=tabs, pd=14.0, gp=8.0):
                win.lb("Сетевые настройки", h=32.0)
                win.lb("Адрес, порт, таймауты обмена.", h=28.0)
            with win.bx(pr=tabs, pd=14.0, gp=8.0):
                win.lb("Тарифное расписание", h=32.0)
                win.lb("Зоны суток, сезоны, праздники.", h=28.0)
            with win.bx(pr=tabs, pd=14.0, gp=8.0):
                win.lb("Журнал событий", h=32.0)
                win.lb("Записи вскрытий, перезагрузок, ошибок.", h=28.0)

        # Связываем активную вкладку с сигналом.
        win.bindv(tabs, lambda: float(page()))

        win.sep()
        a = win.lb(bind=lambda: f"Активная вкладка: {page()} · "
                   f"{NAMES[page()]}", h=30.0)
        win.cls(a, "ok")
        s = win.lb(bind=lambda: f"Последний источник: {src()}", h=28.0)
        win.cls(s, "dim")

        win.sep()
        win.lb("Кнопки и список меняют сигнал → bindv переключает "
               "вкладку. Клик по вкладке шлёт ch и тоже пишет сигнал.",
               h=48.0, wrap=True)

    win.css(CSS)
    win.go()


if __name__ == "__main__":
    main()
