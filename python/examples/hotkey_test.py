"""SSUI — горячие клавиши уровня окна.

Полный набор из плана переноса: ctrl+c, ctrl+f, ctrl+a, ctrl+enter,
space, delete, escape, f2, plus, minus, s. В поле ввода хоткеи
молчат — курсор в него, и те же клавиши печатают текст.

Путь в репозитории: python/examples/hotkey_test.py
"""

import ssui

CSS = """
.clear { background: #00000000; }
.head  { color: #EEF3FF; font-size: 22; }
.dim   { color: #9AA4B2; }
.ok    { color: #22C55E; }
.warn  { color: #F59E0B; }
"""

ITEMS = [f"Параметр {i}" for i in range(14)]


def main():
    win = ssui.W("SSUI · горячие клавиши", 980, 720, thm="drk")

    log = ssui.sgnl("—")
    hits = ssui.sgnl(0)
    zoom = ssui.sgnl(100)
    paused = ssui.sgnl(False)

    def fire(name):
        def f():
            log.st(name)
            hits.st(hits() + 1)
        return f

    def zoom_by(d):
        def f():
            zoom.st(max(25, min(400, zoom() + d)))
            log.st(f"масштаб {zoom()}%")
            hits.st(hits() + 1)
        return f

    def toggle_pause():
        paused.st(not paused())
        log.st("пауза вкл" if paused() else "пауза выкл")
        hits.st(hits() + 1)

    def select_all():
        win.lst_sel(box, list(range(len(ITEMS))))
        log.st("выделить всё")
        hits.st(hits() + 1)

    def reset():
        win.lst_sel(box, [])
        log.st("сброс (escape)")
        hits.st(hits() + 1)

    # Регистрация до показа окна.
    win.hotkey("ctrl+c", fire("копировать (ctrl+c)"))
    win.hotkey("ctrl+f", fire("поиск (ctrl+f)"))
    win.hotkey("ctrl+a", select_all)
    win.hotkey("ctrl+enter", fire("применить (ctrl+enter)"))
    win.hotkey("space", toggle_pause)
    win.hotkey("delete", fire("удалить (delete)"))
    win.hotkey("escape", reset)
    win.hotkey("f2", fire("переименовать (f2)"))
    win.hotkey("plus", zoom_by(25))
    win.hotkey("minus", zoom_by(-25))
    win.hotkey("s", fire("сохранить (s)"))

    with win.bx(pr=win.rt(), ax="h", pd=14.0, gp=14.0) as root:
        win.cls(root, "clear")

        with win.bx(pd=12.0, gp=8.0):
            t = win.lb("Список — фокус мышью, потом клавиши", h=34.0)
            win.cls(t, "head")
            box = win.lst(ITEMS, multi=True, ch=lambda s: log.st(
                f"выбор мышью: {list(s)}"), h=560.0)

        with win.bx(pd=14.0, gp=10.0, w=340.0):
            t2 = win.lb("Состояние", h=34.0)
            win.cls(t2, "head")
            win.sep()
            a = win.lb(bind=lambda: f"Последний хоткей: {log()}",
                       h=52.0, wrap=True)
            win.cls(a, "ok")
            win.lb(bind=lambda: f"Срабатываний: {hits()}", h=28.0)
            win.lb(bind=lambda: f"Масштаб: {zoom()}%", h=28.0)
            p = win.lb(bind=lambda: "Пауза: "
                       + ("да" if paused() else "нет"), h=28.0)
            win.cls(p, "warn")

            win.sep()
            t3 = win.lb("Проверка поля ввода", h=32.0)
            win.cls(t3, "head")
            d = win.lb("Кликни в поле и жми те же клавиши — "
                       "хоткеи молчат, идёт обычный ввод.", h=54.0, wrap=True)
            win.cls(d, "dim")
            win.tx(ph="s, space, delete печатаются здесь", h=46.0)

            win.sep()
            win.lb("plus/minus ловят и цифровой блок клавиатуры.",
                   h=44.0, wrap=True)

    win.css(CSS)
    win.go()


if __name__ == "__main__":
    main()
