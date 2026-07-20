"""SSUI — множественные независимые окна.

Модальные и немодальные окна, безрамочный сплэш, центрирование,
кнопки заголовка, прозрачность, возврат данных в родителя.
"""

import ssui

CSS = """
frame { background: #1c244000; radius: 16; }
.clear { background: #00000000; }
label { color: #eef3ff; }
.splash { background: #0d142400; radius: 20; }
.title { color: #3b82f6; }
.ok { background: #2fbf71; color: #ffffff; }
.no { background: #e5484d; color: #ffffff; }
"""

THEMES = ["drk", "blk", "lit", "wht"]


def main():
    win = ssui.W("SSUI · многооконность", 940, 700, thm="drk",
                 glass=True, tint=0.85)

    log = ssui.sgnl("—")
    back = ssui.sgnl("данных нет")
    opened = ssui.sgnl(0)

    theme = ssui.sgnl("drk")
    modal = ssui.sgnl(True)
    center = ssui.sgnl(True)
    resiz = ssui.sgnl(True)
    minb = ssui.sgnl(False)
    maxb = ssui.sgnl(False)
    glassy = ssui.sgnl(True)
    tintv = ssui.sgnl(0.75)
    blurry = ssui.sgnl(False)
    subw = ssui.sgnl(520)
    subh = ssui.sgnl(440)

    kids = []

    def common():
        return dict(
            thm=theme(),
            glass=glassy(),
            tint=float(tintv()),
            blur=blurry(),
        )

    def track(w):
        kids.append(w)
        opened.st(opened() + 1)

    def open_child():
        # Результат копится здесь и уезжает в родителя при закрытии.
        result = {"applied": False, "level": 0.5}
        name = ssui.sgnl("")

        def finish():
            if result["applied"]:
                back.st(
                    f"применено · «{name() or 'без имени'}» · "
                    f"уровень {result['level']:.2f}"
                )
            else:
                back.st("окно закрыто без сохранения")
            opened.st(max(0, opened() - 1))

        sub = win.subwin(
            "Настройка интерфейса",
            int(subw()),
            int(subh()),
            modal=modal(),
            center=center(),
            resizable=resiz(),
            minbox=minb(),
            maxbox=maxb(),
            on_close=finish,
            **common(),
        )
        track(sub)

        def apply():
            result["applied"] = True
            sub.close()

        with sub:
            with sub.bx(pd=16.0, gp=10.0):
                sub.lb("Дочернее окно со своим деревом", h=30.0)
                sub.sep()
                sub.lb(f"Тема: {theme()} · тон: {tintv():.2f}", h=26.0)
                sub.tx("", sig=name, ph="имя профиля", h=44.0)
                sub.sl(0.5, h=36.0,
                       ch=lambda v: result.__setitem__("level", v))
                sub.lst([f"Пункт {i}" for i in range(1, 10)], h=150.0)
                with sub.bx(ax="h", gp=8.0, h=54.0) as row:
                    sub.cls(row, "clear")
                    ok = sub.bt("Применить", h=46.0, clk=apply)
                    no = sub.bt("Отмена", h=46.0, clk=lambda: sub.close())
                    sub.cls(ok, "ok")
                    sub.cls(no, "no")
            sub.css(CSS)

    def open_tool():
        n = opened() + 1
        tw = win.subwin(
            f"Индикатор {n}", 360, 230,
            modal=False, center=False,
            minbox=False, maxbox=False,
            on_close=lambda: (back.st(f"индикатор {n} закрыт"),
                              opened.st(max(0, opened() - 1))),
            **common(),
        )
        track(tw)
        with tw:
            with tw.bx(pd=16.0, gp=10.0):
                tw.lb(f"Немодальное окно #{n}", h=28.0)
                tw.lb("Открой несколько и подвигай.", h=26.0, wrap=True)
                tw.pr(0.65, h=18.0)
                tw.bt("Закрыть окно", h=40.0, clk=lambda: tw.close())
            tw.css(CSS)

    def open_splash():
        sp = win.subwin(
            "", 420, 270,
            frameless=True, topmost=True, center=True,
            resizable=False, closebox=False,
            on_close=lambda: (back.st("сплэш закрыт"),
                              opened.st(max(0, opened() - 1))),
            **common(),
        )
        track(sp)
        with sp:
            with sp.bx(pd=24.0, gp=12.0) as card:
                sp.cls(card, "splash")
                t = sp.lb("SSUI", h=54.0)
                sp.cls(t, "title")
                sp.lb("Загрузка компонентов…", h=28.0)
                sp.spn(h=48.0)
                sp.bt("Закрыть", h=40.0, clk=lambda: sp.close())
            sp.css(CSS)

    with win.bx(pr=win.rt(), ax="h", pd=14.0, gp=14.0) as root:
        win.cls(root, "clear")

        with win.bx(pd=16.0, gp=8.0, w=380.0):
            win.lb("Геометрия и рамка", h=30.0)
            win.sep()

            win.lb(bind=lambda: f"Ширина: {int(subw())}", h=24.0)
            win.sl(0.5, ch=lambda v: subw.st(320 + v * 680), h=32.0)

            win.lb(bind=lambda: f"Высота: {int(subh())}", h=24.0)
            win.sl(0.5, ch=lambda v: subh.st(240 + v * 560), h=32.0)

            win.dd(THEMES, ch=lambda i: theme.st(THEMES[i]), h=44.0)

            win.sw("Модальное", on=True, clk=lambda v: modal.st(v), h=36.0)
            win.sw("По центру родителя", on=True,
                   clk=lambda v: center.st(v), h=36.0)
            win.sw("Изменяемый размер", on=True,
                   clk=lambda v: resiz.st(v), h=36.0)
            win.sw("Кнопка «свернуть»", on=False,
                   clk=lambda v: minb.st(v), h=36.0)
            win.sw("Кнопка «развернуть»", on=False,
                   clk=lambda v: maxb.st(v), h=36.0)

        with win.bx(pd=16.0, gp=8.0):
            win.lb("Прозрачность и наложение", h=30.0)
            win.sep()
            win.sw("Прозрачный фон (glass)", on=True,
                   clk=lambda v: glassy.st(v), h=36.0)
            win.lb(bind=lambda: f"Тон фона: {tintv():.2f}", h=24.0)
            win.sl(0.75, ch=lambda v: tintv.st(v), h=32.0)
            win.sw("Размытие фона (blur)", on=False,
                   clk=lambda v: blurry.st(v), h=36.0)
            win.lb("Значения применяются к следующему окну.",
                   h=40.0, wrap=True)

            win.sep()
            win.bt("Диалог настроек", h=46.0, clk=open_child)
            win.bt("Немодальный индикатор", h=46.0, clk=open_tool)
            win.bt("Безрамочный сплэш", h=46.0, clk=open_splash)

            win.sep()
            win.lb(bind=lambda: f"Открыто дочерних: {opened()}", h=26.0)
            win.lb(bind=lambda: f"Возврат: {back()}", h=46.0, wrap=True)
            win.lb(bind=lambda: f"Событие: {log()}", h=26.0)

    win.css(CSS)
    win.go()


if __name__ == "__main__":
    main()