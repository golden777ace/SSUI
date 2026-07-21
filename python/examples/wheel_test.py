"""SSUI — колесо мыши как способ изменения значений.

Части даты и времени, веса арбитратора, окно массива, масштаб.
Внизу — проверка перехвата: колесо над фигурой меняет её цвет
и не прокручивает канву, колесо мимо фигур прокручивает.

Путь в репозитории: python/examples/wheel_test.py
"""

import ssui

CSS = """
.clear { background: #00000000; }
.head  { color: #EEF3FF; font-size: 22; }
.dim   { color: #9AA4B2; }
canvas { background: #0B1020; radius: 12; }
"""

PALETTE = ["#3B82F6", "#22C55E", "#F59E0B", "#EF4444", "#A855F7", "#14B8A6"]

DAYS = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]

FIELDS = ["день", "месяц", "год", "часы", "минуты", "секунды"]

# Ширина колонки поля даты на канве.
COL_W = 110.0
COL_Y = 30.0
COL_H = 70.0


def main():
    win = ssui.W("SSUI · колесо мыши", 1120, 800, thm="drk")

    parts = ssui.sgnl([14, 7, 2026, 12, 30, 0])
    # Части даты и времени, перебираются колесом.

    weights = ssui.sgnl([10, 20, 30, 40])
    # Веса арбитратора: четыре целых, каждое крутится отдельно.

    window = ssui.sgnl(0)
    zoom = ssui.sgnl(1.0)
    colors = ssui.sgnl([0] * 12)
    log = ssui.sgnl("—")

    def limits(i):
        y = parts()[2]
        leap = y % 4 == 0 and (y % 100 != 0 or y % 400 == 0)
        dmax = DAYS[parts()[1] - 1] + (1 if leap and parts()[1] == 2 else 0)
        return [(1, dmax), (1, 12), (2000, 2099),
                (0, 23), (0, 59), (0, 59)][i]

    def date_shapes():
        out = []
        for i, name in enumerate(FIELDS):
            x = 20.0 + i * COL_W
            out.append(("rect", [x, COL_Y, COL_W - 10.0, COL_H, 8.0, 0.0],
                        "#1E293B", ""))
            out.append(("text", [x + 10.0, COL_Y + 6.0, COL_W - 20.0, 26.0],
                        "#EEF3FF", f"{parts()[i]:02d}"))
            out.append(("text", [x + 10.0, COL_Y + 38.0, COL_W - 20.0, 22.0],
                        "#9AA4B2", name))
        out.append(("text", [20.0, COL_Y + COL_H + 16.0, 620.0, 24.0],
                    "#EEF3FF",
                    "{:02d}.{:02d}.{} {:02d}:{:02d}:{:02d}".format(
                        parts()[0], parts()[1], parts()[2],
                        parts()[3], parts()[4], parts()[5])))
        return out

    def date_wheel(d, x, y):
        # Какая колонка под курсором — определяем по координате.
        i = int((x - 20.0) // COL_W)
        if not (0 <= i < len(FIELDS)):
            return
        if not (COL_Y <= y <= COL_Y + COL_H):
            return
        lo, hi = limits(i)
        cur = list(parts())
        v = cur[i] + (1 if d > 0 else -1)
        # Перенос по кругу — как в переносимом приложении.
        cur[i] = lo if v > hi else (hi if v < lo else v)
        parts.st(cur)
        log.st(f"{FIELDS[i]} = {cur[i]}")

    # --- Веса арбитратора: колесо на каждой метке отдельно ---

    def weight_wheel(k):
        def f(d, x, y):
            cur = list(weights())
            cur[k] = max(0, min(255, cur[k] + (1 if d > 0 else -1)))
            weights.st(cur)
            log.st(f"вес {k} = {cur[k]}")
        return f

    # --- Канва с перехватом ---

    def cell(i):
        """Прямоугольник фигуры с учётом масштаба."""
        z = zoom()
        col = i % 3
        row = i // 3
        return (30.0 * z + col * 220.0 * z, 20.0 * z + row * 150.0 * z,
                190.0 * z, 110.0 * z)

    def field_shapes():
        z = zoom()
        out = []
        for i in range(12):
            x, y, w, h = cell(i)
            out.append(("rect", [x, y, w, h, 10.0 * z, 0.0],
                        PALETTE[colors()[i] % len(PALETTE)], ""))
            out.append(("text", [x + 12.0 * z, y + 12.0 * z, w - 24.0 * z,
                                 24.0 * z],
                        "#0B1020", f"фигура {window() + i}"))
        return out

    def field_wheel(d, x, y):
        # Ищем фигуру под курсором вручную: колбэк колеса даёт
        # координаты, а не индекс.
        for i in range(12):
            fx, fy, fw, fh = cell(i)
            if fx <= x <= fx + fw and fy <= y <= fy + fh:
                cur = list(colors())
                cur[i] += 1 if d > 0 else -1
                colors.st(cur)
                log.st(f"цвет фигуры {i}")
                return
        # Мимо фигур — прокручиваем сами, раз перехватили событие.
        win.cv_view(field, 0.0, view[0] - d * 60.0)
        view[0] = max(0.0, min(560.0, view[0] - d * 60.0))

    view = [0.0]

    def rezoom(d):
        # Масштаб меняет и фигуры, и границы прокрутки: без пересчёта
        # области нижняя часть поля стала бы недостижимой.
        z = max(0.25, min(4.0, zoom() + d * 0.05))
        zoom.st(z)
        win.cv_region(field, 0.0, 0.0, 700.0 * z, 640.0 * z)

    with win.bx(pr=win.rt(), ax="h", pd=14.0, gp=14.0) as root:
        win.cls(root, "clear")

        with win.bx(pd=12.0, gp=8.0):
            t = win.lb("Дата и время: крути над колонкой", h=34.0)
            win.cls(t, "head")
            date = win.cv([], bind=date_shapes, h=160.0)
            win.wheel(date, date_wheel)

            t2 = win.lb("Канва: колесо над фигурой меняет цвет", h=32.0)
            win.cls(t2, "head")
            field = win.cv([], bind=field_shapes, scroll=True, h=330.0)
            win.cv_region(field, 0.0, 0.0, 700.0, 640.0)
            win.wheel(field, field_wheel)
            d = win.lb("Над фигурой — цвет, мимо фигур — прокрутка. "
                       "Встроенная прокрутка канвы перехвачена целиком.",
                       h=52.0, wrap=True)
            win.cls(d, "dim")

        with win.bx(pd=14.0, gp=8.0, w=340.0):
            t3 = win.lb("Веса арбитратора", h=34.0)
            win.cls(t3, "head")
            win.sep()
            for k in range(4):
                lab = win.lb(bind=(lambda kk: lambda: (
                    f"Канал {kk}: {weights()[kk]}"))(k), h=40.0)
                win.wheel(lab, weight_wheel(k))
            d2 = win.lb("Наведи на строку и крути.", h=26.0)
            win.cls(d2, "dim")

            win.sep()
            t4 = win.lb("Окно массива и масштаб", h=32.0)
            win.cls(t4, "head")
            wlab = win.lb(bind=lambda: f"Первый элемент: {window()}", h=40.0)
            win.wheel(wlab, lambda d, x, y: window.st(
                max(0, window() + (1 if d > 0 else -1))))
            zlab = win.lb(bind=lambda: f"Масштаб: {zoom():.2f}x", h=40.0)
            win.wheel(zlab, lambda d, x, y: rezoom(d))

            win.sep()
            t5 = win.lb("Область прокрутки", h=32.0)
            win.cls(t5, "head")
            with win.scr(h=200.0, gp=6.0) as area:
                win.cls(area, "clear")
                for i in range(20):
                    win.lb(f"Строка {i}: колесо здесь прокручивает",
                           h=30.0)

            win.sep()
            win.lb(bind=lambda: f"Событие: {log()}", h=28.0)

    win.css(CSS)
    win.go()


if __name__ == "__main__":
    main()