"""SSUI — авто-высота контейнеров по детям.

Демонстрирует:
  • вложенные контейнеры без h внутри scr — высота по содержимому;
  • тело acc с составными строками — секция не режет детей;
  • упаковку pk без h — блок больше не схлопывается в ноль;
  • обычный поток: авто-высота работает как нижняя граница.

Ползунки меняют отступ, зазор и число строк — высота пересчитывается.
"""

import ssui

CSS = """
frame { background: #141c30cc; radius: 14; }
.clear { background: #00000000; }
label { color: #eef3ff; }
.head { color: #3b82f6; }
.dim  { color: #94a3b8; }
.card { background: #1b2540; radius: 10; }
.tag  { background: #24314f; radius: 8; color: #93c5fd; }
accordion { background: #1b2540; color: #eef3ff; radius: 10; }
"""

FIELDS = [
    ("Адрес устройства", "0x0001"),
    ("Пароль доступа", "••••••"),
    ("Скорость обмена", "9600"),
    ("Чётность", "нет"),
    ("Таймаут ответа", "1500 мс"),
    ("Профиль связи", "HDLC"),
    ("Версия прошивки", "2.14.3"),
]


def main():
    win = ssui.W("SSUI · авто-высота", 1340, 860, thm="drk",
                 glass=True, tint=0.12)

    pad = ssui.sgnl(10.0)
    gap = ssui.sgnl(6.0)
    lines = ssui.sgnl(3)
    log = ssui.sgnl("—")

    def card(title, value, extra):
        # Контейнер без h: высота складывается из детей.
        with win.bx(pd=10.0, gp=6.0) as c:
            win.cls(c, "card")
            win.bindb(c, lambda: (pad(), gap()))
            win.lb(title, h=26.0)
            win.lb(f"Значение: {value}", h=24.0)
            for i in range(extra):
                t = win.lb(f"Примечание {i + 1}", h=22.0)
                win.cls(t, "tag")
        return c

    with win.bx(pr=win.rt(), ax="h", pd=14.0, gp=14.0) as root:
        win.cls(root, "clear")

        with win.bx(pd=12.0, gp=10.0) as col1:
            t = win.lb("scr · карточки без h", h=28.0)
            win.cls(t, "head")
            with win.scr(pd=8.0, gp=10.0) as area:
                win.cls(area, "clear")
                for nm, vl in FIELDS:
                    card(nm, vl, 2)

        with win.bx(pd=12.0, gp=10.0, w=380.0) as col2:
            t2 = win.lb("acc · составные строки", h=28.0)
            win.cls(t2, "head")
            for nm, vl in FIELDS[:3]:
                with win.acc(nm, open=True, grp=2, h=None):
                    card("Параметр", vl, 1)
                    card("Дубль", vl, 0)

            win.sep()
            t3 = win.lb("pk · упаковка без h", h=28.0)
            win.cls(t3, "head")
            with win.bx(pd=8.0, gp=6.0, h=240.0) as packed:
                win.cls(packed, "card")
                top = card("Сверху", "прижат к верху", 1)
                win.pk(top, "t")
                bot = card("Снизу", "прижат к низу", 1)
                win.pk(bot, "b")
                mid = win.lb("Середина растянута", h=30.0)
                win.pk(mid, "t", fill="both", exp=True)

        with win.bx(pd=12.0, gp=8.0, w=360.0):
            t4 = win.lb("Параметры", h=28.0)
            win.cls(t4, "head")
            win.sep()

            win.lb(bind=lambda: f"Отступ карточки: {pad():.0f}", h=24.0)
            win.sl(0.42, ch=lambda v: pad.st(v * 24.0), h=32.0)
            win.lb(bind=lambda: f"Зазор карточки: {gap():.0f}", h=24.0)
            win.sl(0.25, ch=lambda v: gap.st(v * 24.0), h=32.0)

            win.lb(bind=lambda: f"Строк в блоке потока: {lines()}", h=24.0)
            win.spin(3, min=1, max=8, step=1, h=44.0,
                     ch=lambda v: (lines.st(int(v)),
                                   log.st(f"строк: {int(v)}")))

            win.sep()
            t5 = win.lb("Поток · нижняя граница", h=28.0)
            win.cls(t5, "head")
            with win.bx(pd=8.0, gp=6.0, h=300.0) as flow:
                win.cls(flow, "card")
                win.lb("Фиксированная шапка", h=40.0)
                # Контейнер без h: пока есть место — тянется,
                # когда места нет — не сжимается ниже содержимого.
                with win.bx(pd=8.0, gp=4.0) as grown:
                    win.cls(grown, "tag")
                    for i in range(8):
                        n = win.lb(f"Строка {i + 1}", h=24.0)
                        win.show(n, i < 3)
                        win.bindz(n, lambda: 0.0)
                win.lb("Фиксированный подвал", h=40.0)

            win.sep()
            win.lb(bind=lambda: f"Событие: {log()}", h=40.0, wrap=True)
            d = win.lb("Раньше карточка в scr получала ровно 40 px, "
                       "и её строки налезали на соседнюю.",
                       h=60.0, wrap=True)
            win.cls(d, "dim")

    win.css(CSS)
    win.go()


if __name__ == "__main__":
    main()
