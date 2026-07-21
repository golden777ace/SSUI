"""SSUI — постановка вызова в очередь UI-потока.

Имитация обмена с прибором: рабочий поток читает атрибуты
с задержкой и возвращает результаты в UI-поток через post.

Путь в репозитории: python/examples/post_test.py
"""

import threading
import time

import ssui

CSS = """
.clear { background: #00000000; }
.head  { color: #EEF3FF; font-size: 22; }
.warn  { color: #F59E0B; }
.ok    { color: #22C55E; }
"""

ATTRS = [
    "Логический адрес",
    "Серийный номер",
    "Дата и время",
    "Тариф 1, кВт·ч",
    "Тариф 2, кВт·ч",
    "Тариф 3, кВт·ч",
    "Напряжение фазы A",
    "Напряжение фазы B",
    "Напряжение фазы C",
    "Ток фазы A",
    "Ток фазы B",
    "Ток фазы C",
]


def main():
    win = ssui.W("SSUI · очередь UI-потока", 900, 720, thm="drk")

    post = win.post()
    # Получаем контроллер до показа окна, в UI-потоке. Именно этот
    # вызов ставит хук разбора очереди в дерево: без него очередь
    # никто не разбирает. Сам объект post можно свободно передавать
    # в рабочие потоки — он единственный в SSUI потокобезопасный.

    nt = win.nt()

    rows = ssui.sgnl([])
    # Накопленные строки отчёта. Список меняется на месте, а сигнал
    # переписывается целиком, чтобы пометить биндинги грязными.

    state = ssui.sgnl("готов")
    done = ssui.sgnl(0)
    busy = ssui.sgnl(False)
    direct = ssui.sgnl(0)
    # Счётчик для проверки прямой записи из рабочего потока.

    def read_all():
        # Тело рабочего потока. Ни одного обращения к win отсюда нет:
        # всё, что должно попасть в окно, проходит через post.

        post(lambda: state.st("обмен идёт"))

        for i, name in enumerate(ATTRS):
            time.sleep(0.25)
            # Место реального обмена по DLMS. Задержка сети и разбор
            # ответа — всё это чужой поток, окно в это время живое.

            value = f"{(i + 1) * 137 % 1000:04d}"

            def apply(n=name, v=value, k=i + 1):
                # Замыкание с фиксацией текущих значений. Выполнится
                # в UI-потоке в начале ближайшего кадра.
                rows.st(rows() + [(n, v)])
                done.st(k)

            post(apply)

        post(lambda: state.st("готово"))
        post(lambda: busy.st(False))
        post(lambda: nt("Обмен", f"Прочитано атрибутов: {len(ATTRS)}"))
        # Уведомление тоже ставится через post. Внутри посланного
        # колбэка nt() вызывать можно: колбэк уже в UI-потоке.

    def start():
        if busy():
            return
        busy.st(True)
        rows.st([])
        done.st(0)
        state.st("запуск")
        threading.Thread(target=read_all, daemon=True).start()

    def bad_thread():
        # Демонстрация от противного: запись сигнала напрямую
        # из рабочего потока. Значение поменяется, но подписки
        # биндингов живут отдельно в каждом потоке, поэтому
        # UI-поток об изменении не узнает и метка не обновится.
        def body():
            time.sleep(0.3)
            direct.st(direct() + 1)

        threading.Thread(target=body, daemon=True).start()

    def fixed_thread():
        # То же самое, но правильно.
        def body():
            time.sleep(0.3)
            post(lambda: direct.st(direct() + 1))

        threading.Thread(target=body, daemon=True).start()

    def report():
        out = []
        for i, (name, value) in enumerate(rows()):
            y = 10.0 + i * 26.0
            out.append(("text", [12.0, y, 300.0, 24.0], "#9AA4B2", name))
            out.append(("text", [320.0, y, 120.0, 24.0], "#EEF3FF", value))
        return out

    with win.bx(pr=win.rt(), ax="h", pd=14.0, gp=14.0) as root:
        win.cls(root, "clear")

        with win.bx(pd=14.0, gp=10.0, w=320.0):
            t = win.lb("Обмен с прибором", h=34.0)
            win.cls(t, "head")
            win.sep()

            win.bt("Прочитать всё", h=48.0, clk=start)
            win.lb(bind=lambda: f"Состояние: {state()}", h=28.0)
            win.lb(bind=lambda: f"Прочитано: {done()} из {len(ATTRS)}", h=28.0)
            win.pr(0.0, bind=lambda: done() / len(ATTRS), h=18.0)

            win.sep()
            w = win.lb("Проверка прямой записи", h=30.0)
            win.cls(w, "warn")
            win.bt("Из потока напрямую", h=44.0, clk=bad_thread)
            win.bt("Из потока через post", h=44.0, clk=fixed_thread)
            win.lb(bind=lambda: f"Счётчик: {direct()}", h=28.0)
            win.lb("Первая кнопка меняет значение, но метку "
                   "не обновляет. Вторая обновляет.", h=64.0, wrap=True)

        with win.bx(pd=10.0, gp=8.0):
            t2 = win.lb("Результат", h=34.0)
            win.cls(t2, "head")
            win.sep()
            win.cv([], bind=report, h=560.0)

    win.css(CSS)
    win.go()


if __name__ == "__main__":
    main()
