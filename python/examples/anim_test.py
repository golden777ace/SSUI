"""SSUI — анимированный GIF, динамические изображения и таймер.

Демонстрирует:
  • W.frames(path) — число кадров в файле;
  • img(src_bind=...) — источник изображения из колбэка;
  • синтаксис "файл|номер" для выбора кадра GIF;
  • win.every(ms, cb) — периодический вызов без блокировки окна;
  • смену статичной иконки по коду состояния.

Файл logo_anim.gif ожидается рядом с этим скриптом.
"""

import os

import ssui

CSS = """
frame { background: #141c3000; radius: 14; }
.clear { background: #00000000; }
label { color: #eef3ff; }
.head { color: #3b82f6; }
.stage { background: #0b1020; radius: 12; }
.strip { background: #0b1020; radius: 10; }
"""

HERE = os.path.dirname(os.path.abspath(__file__))
GIF = os.path.join(HERE, "logo_anim.gif")

FITS = ["contain", "cover", "fill", "none"]
STATES = ["ok", "warn", "err", "off"]
TITLES = {
    "ok": "Связь установлена",
    "warn": "Просадка напряжения",
    "err": "Реле разомкнуто",
    "off": "Устройство отключено",
}
DOTS = {
    "ok": "#2fbf71",
    "warn": "#f59e0b",
    "err": "#e5484d",
    "off": "#64748b",
}


def main():
    total = ssui.W.frames(GIF)
    # Число кадров читается из самого файла через WIC.
    # Для статичного PNG вернётся 1.

    win = ssui.W("SSUI · анимация и таймер", 1060, 740, thm="drk",
                 glass=True, tint=0.0)

    frame_i = ssui.sgnl(0)
    running = ssui.sgnl(True)
    delay = ssui.sgnl(00.0)
    fit = ssui.sgnl(0.0)
    status = ssui.sgnl(0)
    ticks = ssui.sgnl(0)
    clock = ssui.sgnl(0.0)
    acc = {"ms": 0.0}

    def src():
        # Кадр выбирается суффиксом: "путь|номер".
        return f"{GIF}|{frame_i()}"

    def tick():
        ticks.st(ticks() + 1)
        clock.st(clock() + 20.0)
        if not running():
            return
        acc["ms"] += 20.0
        # Накопление, а не жёсткий интервал: задержку можно менять на лету.
        if acc["ms"] >= delay():
            acc["ms"] = 0.0
            frame_i.st((frame_i() + 1) % total)

    def step(d):
        frame_i.st((frame_i() + d) % total)

    def dial():
        # Круговая шкала прогресса анимации плюс маркер текущего кадра.
        a = -90.0
        sweep = 360.0 * (frame_i() + 1) / total
        return [
            ("arc", [70.0, 70.0, 54.0, 0.0, 360.0, 6.0], "#1e293b", ""),
            ("arc", [70.0, 70.0, 54.0, a, sweep, 6.0], "#3b82f6", ""),
            ("circle", [70.0, 70.0, 6.0, 0.0], DOTS[STATES[status()]], ""),
            ("text", [30.0, 58.0, 80.0, 24.0], "#eef3ff",
             f"{frame_i() + 1}/{total}"),
        ]

    with win.bx(pr=win.rt(), ax="h", pd=14.0, gp=14.0) as root:
        win.cls(root, "clear")

        with win.bx(pd=14.0, gp=10.0):
            t = win.lb("Анимированный GIF покадрово", h=30.0)
            win.cls(t, "head")

            with win.bx(pd=16.0, gp=8.0, h=300.0) as stage:
                win.cls(stage, "stage")
                win.img(src_bind=src, fit_bind=lambda: fit(), h=220.0)
                win.lb(bind=lambda: f"Кадр {frame_i() + 1} из {total}",
                       h=28.0)

            with win.bx(ax="h", gp=12.0, h=160.0) as row:
                win.cls(row, "clear")
                with win.bx(pd=8.0, w=160.0) as strip:
                    win.cls(strip, "strip")
                    win.cv([], bind=dial, h=140.0)
                with win.bx(pd=10.0, gp=6.0):
                    win.lb("Статус устройства", h=26.0)
                    win.lb(bind=lambda: TITLES[STATES[status()]], h=28.0)
                    win.lb(bind=lambda: f"Код: {STATES[status()]}", h=24.0)
                    win.bt("Следующее состояние", h=42.0,
                           clk=lambda: status.st((status() + 1) % len(STATES)))

        with win.bx(pd=14.0, gp=8.0, w=340.0):
            t2 = win.lb("Управление анимацией", h=30.0)
            win.cls(t2, "head")
            win.sep()

            win.sw("Проигрывание", on=True,
                   clk=lambda v: running.st(v), h=38.0)

            win.lb(bind=lambda: f"Задержка кадра: {delay():.0f} мс", h=24.0)
            win.sl(0.3, ch=lambda v: delay.st(20.0 + v * 200.0), h=32.0)

            win.lb("Режим вписывания:", h=24.0)
            win.dd(FITS, ch=lambda i: fit.st(float(i)), h=44.0)

            win.sep()
            with win.bx(ax="h", gp=8.0, h=52.0) as nav:
                win.cls(nav, "clear")
                win.bt("◀", h=44.0, clk=lambda: step(-1))
                win.bt("В начало", h=44.0, clk=lambda: frame_i.st(0))
                win.bt("▶", h=44.0, clk=lambda: step(1))

            win.lb(bind=lambda: f"Позиция: {frame_i() + 1} / {total}", h=24.0)
            win.pr(0.0, bind=lambda: (frame_i() + 1) / total, h=18.0)

            win.sep()
            t3 = win.lb("Таймер окна", h=28.0)
            win.cls(t3, "head")
            win.lb(bind=lambda: f"Тиков с запуска: {ticks()}", h=26.0)
            win.lb(bind=lambda: f"Время: {clock() / 1000.0:6.1f} с", h=26.0)
            win.lb("Анимация идёт сама по себе: мышь двигать не нужно.",
                   h=48.0, wrap=True)

            win.sep()
            win.lb(f"Файл: {os.path.basename(GIF)}", h=26.0)
            win.lb(bind=lambda: f"Текущий источник: ...|{frame_i()}", h=26.0)

    win.every(20.0, tick)

    win.css(CSS)
    win.go()


if __name__ == "__main__":
    main()