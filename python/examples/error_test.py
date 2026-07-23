"""SSUI — хук ошибок on_error и очередь диалогов.

Слева: исключения из разных источников (клик, таймер, посланный из
рабочего потока колбэк) попадают в on_error с полным traceback.
Справа: три диалога, запрошенных подряд, показываются по очереди —
ни один запрос не теряется.

Путь в репозитории: python/examples/error_test.py
"""

import threading
import time

import ssui

CSS = """
.clear { background: #00000000; }
.head  { color: #EEF3FF; font-size: 22; }
.dim   { color: #9AA4B2; }
.ok    { color: #22C55E; }
.err   { color: #EF4444; }
"""


def main():
    win = ssui.W("SSUI · ошибки и очередь диалогов", 1040, 700, thm="drk")
    post = win.post()
    dlg = win.dlg()

    trace = ssui.sgnl("")
    caught = ssui.sgnl(0)
    answers = ssui.sgnl([])

    def on_error(text):
        caught.st(caught() + 1)
        trace.st(text.strip())

    win.on_error(on_error)

    # --- Источники исключений ---

    def boom_click():
        raise ValueError("ошибка прямо в обработчике клика")

    def boom_timer():
        win.after(100.0, lambda: 1 / 0)

    def boom_thread():
        def work():
            time.sleep(0.2)
            post(lambda: {}["нет ключа"])
        threading.Thread(target=work, daemon=True).start()

    def boom_nested():
        def inner():
            raise RuntimeError("сбой обмена с прибором")

        def outer():
            inner()
        outer()

    # --- Очередь диалогов ---

    def answered(tag):
        def on(i):
            answers.st(answers() + [f"{tag}:{i}"])
        return on

    def burst():
        # Три запроса до ближайшего кадра: должны показаться все три.
        answers.st([])
        dlg("Запрос 1", "Первый из трёх подряд.", ["Ок"], on=answered("A"))
        dlg("Запрос 2", "Второй — не должен потеряться.",
            ["Нет", "Да"], on=answered("B"))
        dlg("Запрос 3", "Третий закрывает серию.", ["Готово"],
            on=answered("C"))

    def burst_thread():
        # То же самое, но заявки приходят из рабочего потока.
        answers.st([])

        def work():
            for n in range(3):
                time.sleep(0.1)
                post(lambda n=n: dlg(
                    f"Из потока {n + 1}",
                    "Ошибка обмена с прибором.",
                    ["Повторить", "Отмена"], on=answered(f"T{n + 1}")))
        threading.Thread(target=work, daemon=True).start()

    with win.bx(pr=win.rt(), ax="h", pd=14.0, gp=14.0) as root:
        win.cls(root, "clear")

        with win.bx(pd=14.0, gp=10.0):
            t = win.lb("Исключения в колбэках", h=34.0)
            win.cls(t, "head")
            win.lb("Без on_error всё это ушло бы в stderr и пропало "
                   "в сборке без консоли.", h=44.0, wrap=True)

            with win.bx(ax="h", gp=8.0, h=50.0) as b1:
                win.cls(b1, "clear")
                win.bt("В клике", h=46.0, clk=boom_click)
                win.bt("В таймере", h=46.0, clk=boom_timer)
            with win.bx(ax="h", gp=8.0, h=50.0) as b2:
                win.cls(b2, "clear")
                win.bt("Из потока", h=46.0, clk=boom_thread)
                win.bt("Вложенный", h=46.0, clk=boom_nested)

            c = win.lb(bind=lambda: f"Перехвачено: {caught()}", h=30.0)
            win.cls(c, "ok")
            win.sep()
            win.lb("Traceback:", h=26.0)
            tr = win.lb(bind=lambda: trace() or "—", h=300.0, wrap=True)
            win.cls(tr, "err")

        with win.bx(pd=14.0, gp=10.0, w=420.0):
            t2 = win.lb("Очередь диалогов", h=34.0)
            win.cls(t2, "head")
            win.lb("Раньше слот был один: второй запрос затирал "
                   "первый, и его on не срабатывал никогда.",
                   h=64.0, wrap=True)

            win.bt("Три подряд", h=48.0, clk=burst)
            win.bt("Три из потока", h=48.0, clk=burst_thread)

            win.sep()
            a = win.lb(bind=lambda: "Ответы: "
                       + (", ".join(answers()) if answers() else "—"),
                       h=60.0, wrap=True)
            win.cls(a, "ok")
            n = win.lb(bind=lambda: f"Ответов получено: {len(answers())}",
                       h=28.0)
            win.cls(n, "dim")

            win.sep()
            win.lb("Ожидание: три запроса — три ответа. Диалоги "
                   "показываются строго по очереди, в порядке "
                   "поступления.", h=70.0, wrap=True)

    win.css(CSS)
    win.go()


if __name__ == "__main__":
    main()
