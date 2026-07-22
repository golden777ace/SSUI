"""SSUI — диалоги: индекс кнопки, отмена, длинный текст.

on вызывается ровно один раз: индексом кнопки при подтверждении и -1
при Esc. Счётчик доказывает единственность вызова. Длинное сообщение
демонстрирует перенос и обрезание области текста.

Путь в репозитории: python/examples/dlg_test.py
"""

import ssui

CSS = """
.clear { background: #00000000; }
.head  { color: #EEF3FF; font-size: 22; }
.ok    { color: #22C55E; }
.warn  { color: #F59E0B; }
"""

LONG = (
    "Это намеренно длинное сообщение, чтобы проверить перенос строк "
    "по словам и поведение при переполнении фиксированной области "
    "текста диалога. Диалог не предназначен для больших объёмов: "
    "если текст не помещается, он обрежется по нижней границе панели. "
    "Для подробных отчётов используйте обычные виджеты окна или "
    "прокручиваемую область scr вместо модального диалога ядра."
)


def main():
    win = ssui.W("SSUI · диалоги", 860, 560, thm="drk")
    dlg = win.dlg()

    last = ssui.sgnl("—")
    calls = ssui.sgnl(0)

    def answered(text):
        def on(i):
            calls.st(calls() + 1)
            if i < 0:
                last.st("отмена (-1)")
            else:
                last.st(f"кнопка {i}: {text[i]}")
        return on

    def ask_save():
        btns = ["Отмена", "Не сохранять", "Сохранить"]
        dlg("Сохранить изменения?", "Профиль счётчика был изменён.",
            btns, on=answered(btns))

    def ask_yesno():
        btns = ["Нет", "Да"]
        dlg("Подтверждение", "Стереть конфигурацию прибора?",
            btns, on=answered(btns))

    def show_msg():
        dlg.msg("Готово", "Файл выгружен на прибор.",
                on=lambda i: last.st(f"msg закрыт: {i}"))

    def show_alert():
        dlg.alert("Не удалось открыть COM-порт.",
                  on=lambda i: last.st(f"alert закрыт: {i}"))

    def show_long():
        dlg("Длинное сообщение", LONG, ["Понятно"],
            on=lambda i: last.st(f"длинный диалог: {i}"))

    with win.bx(pr=win.rt(), ax="v", pd=16.0, gp=12.0) as root:
        win.cls(root, "clear")

        t = win.lb("Нажми кнопку, затем ответь в диалоге или нажми Esc",
                   h=34.0)
        win.cls(t, "head")

        with win.bx(ax="h", gp=10.0, h=54.0) as bar:
            win.cls(bar, "clear")
            win.bt("Сохранить?", h=50.0, clk=ask_save)
            win.bt("Да/Нет", h=50.0, clk=ask_yesno)
            win.bt("Message", h=50.0, clk=show_msg)
            win.bt("Alert", h=50.0, clk=show_alert)
            win.bt("Длинный текст", h=50.0, clk=show_long)

        win.sep()
        a = win.lb(bind=lambda: f"Последний ответ: {last()}", h=34.0,
                   wrap=True)
        win.cls(a, "ok")
        c = win.lb(bind=lambda: f"Всего вызовов on: {calls()}", h=28.0)
        win.cls(c, "warn")

        win.sep()
        win.lb("Esc отменяет диалог и присылает -1. Enter нажимает "
               "правую (основную) кнопку. Клик мимо панели не "
               "закрывает — диалог модальный.", h=56.0, wrap=True)

    win.css(CSS)
    win.go()


if __name__ == "__main__":
    main()
