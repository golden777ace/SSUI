"""SSUI — оформление рамки окна.

Демонстрирует:
  • W(frameless=...) — окно без рамки и заголовка ОС;
  • topmost / center — поверх всех окон и по центру;
  • minbox / maxbox / closebox — набор кнопок заголовка;
  • win.frame(icon=..., cap=..., cap_txt=..., brd=..., dark=...) —
    иконка в заголовке и точечная перекраска заголовка и рамки.

Перекраска заголовка работает на Windows 11 (сборка 22000+).
На Windows 10 эти вызовы игнорируются, окно рисуется системной темой.

Иконка ожидается в файле logo.ico рядом со скриптом; если его нет,
поле иконки просто не применяется.
"""

import os

import ssui

CSS = """
frame { background: #141c30cc; radius: 14; }
.clear { background: #00000000; }
label { color: #eef3ff; }
.head { color: #3b82f6; }
.splash { background: #0d1424ee; radius: 20; }
.title { color: #3b82f6; }
.sw { background: #1e293b; radius: 10; }
"""

HERE = os.path.dirname(os.path.abspath(__file__))
ICON = os.path.join(HERE, "logo.ico")

PALETTE = [
    ("синий", "#1d4ed8"),
    ("зелёный", "#15803d"),
    ("багровый", "#9f1239"),
    ("графит", "#1f2937"),
    ("песочный", "#b45309"),
]
TEXTS = [("белый", "#ffffff"), ("чёрный", "#000000"),
         ("голубой", "#93c5fd"), ("янтарный", "#fcd34d")]


def main():
    win = ssui.W("SSUI · оформление рамки", 1060, 760, thm="drk",
                 glass=True, tint=0.9)

    # Оформление главного окна задаётся до показа.
    win.frame(
        icon=ICON if os.path.exists(ICON) else None,
        cap="#111827",
        cap_txt="#93c5fd",
        brd="#3b82f6",
        dark=True,
    )

    cap = ssui.sgnl(0)
    txt = ssui.sgnl(0)
    brd = ssui.sgnl(0)
    dark = ssui.sgnl(True)
    use_icon = ssui.sgnl(os.path.exists(ICON))

    minb = ssui.sgnl(True)
    maxb = ssui.sgnl(True)
    closeb = ssui.sgnl(True)
    resiz = ssui.sgnl(True)
    center = ssui.sgnl(True)
    top = ssui.sgnl(False)
    frameless = ssui.sgnl(False)

    log = ssui.sgnl("—")
    opened = ssui.sgnl(0)
    kids = []

    def deco(sub):
        sub.frame(
            icon=ICON if (use_icon() and os.path.exists(ICON)) else None,
            cap=PALETTE[cap()][1],
            cap_txt=TEXTS[txt()][1],
            brd=PALETTE[brd()][1],
            dark=dark(),
        )

    def close_one(w):
        w.close()

    def open_window():
        n = opened() + 1
        sub = win.subwin(
            f"Окно {n}", 560, 380,
            modal=False,
            center=center(),
            topmost=top(),
            frameless=frameless(),
            resizable=resiz(),
            minbox=minb(),
            maxbox=maxb(),
            closebox=closeb(),
            glass=True,
            tint=0.92,
            on_close=lambda: (opened.st(max(0, opened() - 1)),
                              log.st(f"окно {n} закрыто")),
        )
        deco(sub)
        kids.append(sub)
        opened.st(n)
        log.st(f"окно {n} открыто")

        with sub:
            with sub.bx(pd=18.0, gp=10.0) as card:
                if frameless():
                    sub.cls(card, "splash")
                    t = sub.lb("SSUI", h=48.0)
                    sub.cls(t, "title")
                    sub.lb("Безрамочное окно поверх остальных.",
                           h=28.0, wrap=True)
                    sub.spn(h=44.0)
                else:
                    sub.lb(f"Окно {n}", h=32.0)
                    sub.sep()
                    sub.lb(f"Заголовок: {PALETTE[cap()][0]}", h=26.0)
                    sub.lb(f"Текст заголовка: {TEXTS[txt()][0]}", h=26.0)
                    sub.lb(f"Рамка: {PALETTE[brd()][0]}", h=26.0)
                    sub.lb(f"Тёмный режим: {'да' if dark() else 'нет'}",
                           h=26.0)
                    sub.lb("Потяни за край, сверни, разверни — "
                           "проверь набор кнопок.", h=48.0, wrap=True)
                sub.bt("Закрыть", h=44.0, clk=lambda: close_one(sub))
            sub.css(CSS)

    def repaint_main():
        # Перекрасить уже показанное окно нельзя: оформление
        # применяется при создании HWND. Поэтому меняем только
        # заготовку для следующих окон и сообщаем об этом.
        log.st("оформление применится к следующему окну")

    with win.bx(pr=win.rt(), ax="h", pd=14.0, gp=14.0) as root:
        win.cls(root, "clear")

        with win.bx(pd=14.0, gp=8.0, w=340.0):
            t = win.lb("Цвета рамки", h=30.0)
            win.cls(t, "head")
            win.sep()

            win.lb("Цвет заголовка:", h=24.0)
            win.dd([p[0] for p in PALETTE],
                   ch=lambda i: (cap.st(i), repaint_main()), h=44.0)

            win.lb("Цвет текста заголовка:", h=24.0)
            win.dd([t[0] for t in TEXTS],
                   ch=lambda i: (txt.st(i), repaint_main()), h=44.0)

            win.lb("Цвет рамки:", h=24.0)
            win.dd([p[0] for p in PALETTE],
                   ch=lambda i: (brd.st(i), repaint_main()), h=44.0)

            win.sw("Тёмный режим заголовка", on=True,
                   clk=lambda v: dark.st(v), h=38.0)
            win.sw("Иконка в заголовке", on=os.path.exists(ICON),
                   clk=lambda v: use_icon.st(v), h=38.0)

            win.sep()
            win.lb(bind=lambda: (
                f"Иконка: {os.path.basename(ICON)}"
                if os.path.exists(ICON) else "Файл logo.ico не найден"
            ), h=44.0, wrap=True)

        with win.bx(pd=14.0, gp=8.0):
            t2 = win.lb("Кнопки и режим окна", h=30.0)
            win.cls(t2, "head")
            win.sep()

            win.sw("Кнопка «свернуть»", on=True,
                   clk=lambda v: minb.st(v), h=38.0)
            win.sw("Кнопка «развернуть»", on=True,
                   clk=lambda v: maxb.st(v), h=38.0)
            win.sw("Кнопка «закрыть»", on=True,
                   clk=lambda v: closeb.st(v), h=38.0)
            win.sw("Изменяемый размер", on=True,
                   clk=lambda v: resiz.st(v), h=38.0)

            win.sep()
            win.sw("Без рамки (frameless)", on=False,
                   clk=lambda v: frameless.st(v), h=38.0)
            win.sw("Поверх всех окон (topmost)", on=False,
                   clk=lambda v: top.st(v), h=38.0)
            win.sw("По центру родителя", on=True,
                   clk=lambda v: center.st(v), h=38.0)

            win.sep()
            win.bt("Открыть окно с этими настройками", h=48.0,
                   clk=open_window)
            win.lb(bind=lambda: f"Открыто окон: {opened()}", h=26.0)
            win.lb(bind=lambda: f"Событие: {log()}", h=26.0)

            win.sep()
            win.lb("Оформление применяется в момент создания окна, "
                   "поэтому меняй настройки до нажатия кнопки.",
                   h=56.0, wrap=True)
            win.lb("Перекраска заголовка требует Windows 11. "
                   "На Windows 10 работают иконка и набор кнопок.",
                   h=56.0, wrap=True)

    win.css(CSS)
    win.go()


if __name__ == "__main__":
    main()
