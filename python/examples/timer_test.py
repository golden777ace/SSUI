"""SSUI — одноразовые и отменяемые таймеры.

Три сценария из переносимого приложения: дебаунс, автообновление
с перезапуском, отложенная задача с отменой.

Путь в репозитории: python/examples/timer_test.py
"""

import time

import ssui

CSS = """
.clear { background: #00000000; }
.head  { color: #EEF3FF; font-size: 22; }
.dim   { color: #9AA4B2; }
.ok    { color: #22C55E; }
.warn  { color: #F59E0B; }
"""


def main():
    win = ssui.W("SSUI · таймеры", 980, 760, thm="drk")

    # --- Сценарий 1: дебаунс ---
    # Ползунок меняется непрерывно, а тяжёлый пересчёт должен
    # выполниться один раз, через 200 мс после последнего движения.

    raw = ssui.sgnl(0.5)
    calc = ssui.sgnl("—")
    calls = ssui.sgnl(0)
    moves = ssui.sgnl(0)
    deb = {"id": 0}
    # Идентификатор отложенного пересчёта хранится в словаре,
    # потому что замыкания пишут в него, а не читают.

    def heavy():
        calls.st(calls() + 1)
        calc.st(f"{raw() * 1000:.1f}")

    def on_slide(v):
        raw.st(v)
        moves.st(moves() + 1)
        win.cancel(deb["id"])
        # Отмена предыдущей заявки. Если её уже нет, вызов безвреден.
        deb["id"] = win.after(200.0, heavy)

    # --- Сценарий 2: автообновление с перезапуском ---
    # Периодический опрос, интервал которого меняется на лету.
    # every отменяется и ставится заново с новым периодом.

    ticks = ssui.sgnl(0)
    period = ssui.sgnl(1000)
    auto = {"id": 0}
    running = ssui.sgnl(False)

    def poll():
        ticks.st(ticks() + 1)

    def restart():
        win.cancel(auto["id"])
        auto["id"] = 0
        if running():
            auto["id"] = win.every(float(period()), poll)

    def toggle(on):
        running.st(on)
        restart()

    def set_period(i):
        period.st([250, 500, 1000, 2000][i])
        restart()
        # Перезапуск обязателен: период таймера задаётся при
        # регистрации и на лету не меняется.

    # --- Сценарий 3: отложенная задача с отменой ---
    # Действие через три секунды, которое можно успеть отменить.

    plan = ssui.sgnl("нет задачи")
    late = {"id": 0}

    def fire():
        plan.st(f"выполнено в {time.strftime('%H:%M:%S')}")
        late["id"] = 0

    def schedule():
        win.cancel(late["id"])
        late["id"] = win.after(3000.0, fire)
        plan.st("запланировано через 3 с")

    def drop():
        if late["id"]:
            win.cancel(late["id"])
            late["id"] = 0
            plan.st("отменено")

    # --- Сценарий 4: часы ---
    # Простейший every, поставленный до показа окна.

    clock = ssui.sgnl("--:--:--")
    win.every(1000.0, lambda: clock.st(time.strftime("%H:%M:%S")))

    with win.bx(pr=win.rt(), ax="h", pd=14.0, gp=14.0) as root:
        win.cls(root, "clear")

        with win.bx(pd=14.0, gp=10.0, w=440.0):
            t = win.lb("Дебаунс", h=34.0)
            win.cls(t, "head")
            win.sep()
            win.lb("Тяни ползунок быстро — пересчёт один, "
                   "через 200 мс после остановки.", h=50.0, wrap=True)
            win.sl(0.5, ch=on_slide, h=40.0)
            win.lb(bind=lambda: f"Положение: {raw():.3f}", h=26.0)
            win.lb(bind=lambda: f"Событий ползунка: {moves()}", h=26.0)
            r = win.lb(bind=lambda: f"Пересчётов: {calls()}", h=26.0)
            win.cls(r, "ok")
            win.lb(bind=lambda: f"Результат: {calc()}", h=26.0)

            win.sep()
            t3 = win.lb("Отложенная задача", h=34.0)
            win.cls(t3, "head")
            win.sep()
            with win.bx(ax="h", gp=8.0, h=52.0) as bar:
                win.cls(bar, "clear")
                win.bt("Запланировать", h=46.0, clk=schedule)
                win.bt("Отменить", h=46.0, clk=drop)
            w = win.lb(bind=lambda: f"Состояние: {plan()}", h=28.0)
            win.cls(w, "warn")

        with win.bx(pd=14.0, gp=10.0):
            t2 = win.lb("Автообновление", h=34.0)
            win.cls(t2, "head")
            win.sep()
            win.sw("Опрашивать", on=False, clk=toggle, h=40.0)
            win.lb("Период:", h=26.0)
            win.dd(["250 мс", "500 мс", "1 с", "2 с"], sel=2,
                   ch=set_period, h=44.0)
            win.lb(bind=lambda: f"Текущий период: {period()} мс", h=26.0)
            win.lb(bind=lambda: f"Опросов: {ticks()}", h=26.0)
            win.pr(0.0, bind=lambda: (ticks() % 20) / 20.0, h=18.0)

            win.sep()
            c = win.lb(bind=lambda: f"Часы: {clock()}", h=40.0)
            win.cls(c, "head")
            d = win.lb("Часы поставлены через every до показа окна "
                       "и не отменяются.", h=46.0, wrap=True)
            win.cls(d, "dim")

            win.sep()
            win.lb("Смена периода — это cancel плюс новый every: "
                   "интервал задаётся при регистрации.", h=64.0, wrap=True)

    win.css(CSS)
    win.go()


if __name__ == "__main__":
    main()
