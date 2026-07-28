"""SSUI — двусторонний sig и программное управление вводом.

Демонстрирует:
  • sig= у tx, ta, dd, ch, sw, tgl, sl, rat, rsl, cal, tm, clr;
  • запись в сигнал из колбэка таймера и после показа окна;
  • val, chk, dd_sel, txt, items, cal_set, tm_set, clr_set, rsl_set;
  • шаблон группы радиокнопок через один сигнал и bindv.

Кнопка «Прочитать из устройства» имитирует внешний источник:
заполняет форму целиком, не трогая ни одного колбэка.
"""

import random

import ssui

CSS = """
frame { background: #141c30cc; radius: 14; }
.clear { background: #00000000; }
label { color: #eef3ff; }
.head { color: #3b82f6; }
.dim  { color: #94a3b8; }
.card { background: #1b2540; radius: 10; }
"""

PROFILES = ["HDLC", "Прямой", "Шлюз"]
SPEEDS = ["300", "1200", "9600", "19200", "115200"]
MODES = ["ASCII", "HEX", "BIN"]


def main():
    win = ssui.W("SSUI · двусторонний sig", 1360, 900, thm="drk",
                 glass=True, tint=0.12)

    addr = ssui.sgnl("0x0001")
    passwd = ssui.sgnl("")
    note = ssui.sgnl("")
    profile = ssui.sgnl(0)
    speed = ssui.sgnl(2)
    hidden = ssui.sgnl(False)
    autoconn = ssui.sgnl(True)
    verbose = ssui.sgnl(False)
    level = ssui.sgnl(0.5)
    stars = ssui.sgnl(3)
    window = ssui.sgnl((0.2, 0.8))
    date = ssui.sgnl((2026, 7, 28))
    clock = ssui.sgnl((14, 30))
    tint = ssui.sgnl((0.58, 0.75, 0.96))
    mode = ssui.sgnl(1)
    log = ssui.sgnl("—")
    ticking = ssui.sgnl(False)

    nodes = {}

    def read_device():
        # Внешний источник: пишем только в сигналы.
        addr.st(f"0x{random.randint(1, 0xFFFF):04X}")
        passwd.st("".join(random.choice("0123456789ABCDEF") for _ in range(8)))
        note.st("Прочитано с прибора\nПоверка до 2029 года")
        profile.st(random.randrange(len(PROFILES)))
        speed.st(random.randrange(len(SPEEDS)))
        hidden.st(random.random() < 0.5)
        autoconn.st(random.random() < 0.5)
        verbose.st(random.random() < 0.5)
        level.st(random.random())
        stars.st(random.randint(0, 5))
        lo = random.random() * 0.5
        window.st((lo, lo + 0.3))
        date.st((2026, random.randint(1, 12), random.randint(1, 28)))
        clock.st((random.randint(0, 23), random.randint(0, 59)))
        tint.st((random.random(), 0.7, 0.9))
        mode.st(random.randrange(len(MODES)))
        log.st("форма заполнена из внешнего источника")

    def reset():
        addr.st("0x0001")
        passwd.st("")
        note.st("")
        profile.st(0)
        speed.st(2)
        hidden.st(False)
        autoconn.st(True)
        verbose.st(False)
        level.st(0.5)
        stars.st(3)
        window.st((0.2, 0.8))
        date.st((2026, 7, 28))
        clock.st((14, 30))
        tint.st((0.58, 0.75, 0.96))
        mode.st(1)
        log.st("форма сброшена")

    def tick():
        if not ticking():
            return
        h, m = clock()
        m += 1
        if m >= 60:
            m = 0
            h = (h + 1) % 24
        clock.st((h, m))

    def imperative():
        # Тот же результат без сигналов, прямыми вызовами.
        win.txt(nodes["addr"], "0xDEAD")
        win.items(nodes["speed"], SPEEDS + ["230400"])
        win.dd_sel(nodes["speed"], 5)
        win.chk(nodes["hidden"], True)
        win.val(nodes["level"], 0.15)
        win.val(nodes["spin"], 7)
        win.cal_set(nodes["cal"], 2030, 1, 1)
        win.tm_set(nodes["tm"], 23, 59)
        win.clr_set(nodes["clr"], 0.02, 0.9, 0.95)
        win.rsl_set(nodes["rsl"], 0.05, 0.45)
        log.st("установлено императивно, без сигналов")

    with win.bx(pr=win.rt(), ax="h", pd=14.0, gp=14.0) as root:
        win.cls(root, "clear")

        with win.bx(pd=12.0, gp=10.0) as col1:
            t = win.lb("Параметры связи", h=28.0)
            win.cls(t, "head")

            with win.bx(pd=10.0, gp=8.0) as c1:
                win.cls(c1, "card")
                win.lb("Адрес устройства", h=24.0)
                nodes["addr"] = win.tx(sig=addr, ph="0x0000", h=42.0)
                win.lb("Пароль доступа", h=24.0)
                nodes["pass"] = win.tx(sig=passwd, ph="8 символов", h=42.0)
                win.lb("Профиль", h=24.0)
                nodes["profile"] = win.dd(PROFILES, sig=profile, h=44.0)
                win.lb("Скорость", h=24.0)
                nodes["speed"] = win.dd(SPEEDS, sel=2, sig=speed, h=44.0)

            with win.bx(pd=10.0, gp=8.0) as c2:
                win.cls(c2, "card")
                nodes["hidden"] = win.ch("Скрывать пароль", sig=hidden, h=36.0)
                nodes["auto"] = win.sw("Автоподключение", on=True,
                                       sig=autoconn, h=38.0)
                nodes["verbose"] = win.tgl("Подробный журнал",
                                           sig=verbose, h=42.0)
                win.lb("Формат данных", h=24.0)
                with win.bx(ax="h", gp=8.0, h=44.0) as mrow:
                    win.cls(mrow, "clear")
                    for k, nm in enumerate(MODES):
                        b = win.rd(nm, grp=9, on=(k == 1), h=38.0,
                                   clk=lambda k=k: mode.st(k))
                        win.bindv(b, lambda k=k: float(mode() == k))

            with win.bx(pd=10.0, gp=8.0) as c3:
                win.cls(c3, "card")
                win.lb("Заметка", h=24.0)
                nodes["note"] = win.ta(sig=note, ph="Свободный текст",
                                       h=110.0)

        with win.bx(pd=12.0, gp=10.0) as col2:
            t2 = win.lb("Числовые параметры", h=28.0)
            win.cls(t2, "head")

            with win.bx(pd=10.0, gp=8.0) as c4:
                win.cls(c4, "card")
                win.lb(bind=lambda: f"Уровень сигнала: {level():.2f}",
                       h=24.0)
                nodes["level"] = win.sl(0.5, sig=level, h=34.0)
                win.lb(bind=lambda: f"Окно архива: "
                                    f"{window()[0]:.2f}–{window()[1]:.2f}",
                       h=24.0)
                nodes["rsl"] = win.rsl(0.2, 0.8, sig=window, h=38.0)
                win.lb(bind=lambda: f"Приоритет: {stars()}", h=24.0)
                nodes["rat"] = win.rat(3, sig=stars, h=42.0)
                win.lb("Число попыток", h=24.0)
                nodes["spin"] = win.spin(3, min=1, max=10, step=1, h=44.0,
                                         ch=lambda v: log.st(f"попыток: {v:.0f}"))

            with win.bx(pd=10.0, gp=8.0) as c5:
                win.cls(c5, "card")
                win.lb(bind=lambda: "Дата: %04d-%02d-%02d" % date(), h=24.0)
                nodes["cal"] = win.cal(2026, 7, 28, sig=date, h=270.0)

        with win.bx(pd=12.0, gp=10.0, w=400.0):
            t3 = win.lb("Внешний источник", h=28.0)
            win.cls(t3, "head")
            win.sep()

            win.bt("Прочитать из устройства", h=46.0, clk=read_device)
            win.bt("Сбросить форму", h=44.0, clk=reset)
            win.bt("Установить императивно", h=44.0, clk=imperative)

            win.sep()
            win.lb(bind=lambda: "Часы: %02d:%02d" % clock(), h=24.0)
            nodes["tm"] = win.tm(14, 30, sig=clock, h=110.0)
            win.sw("Тикать раз в секунду", h=38.0,
                   clk=lambda v: (ticking.st(v),
                                  log.st("часы идут" if v else "часы стоят")))

            win.sep()
            win.lb(bind=lambda: "Оттенок: %.2f %.2f %.2f" % tint(), h=24.0)
            nodes["clr"] = win.clr(0.58, 0.75, 0.96, sig=tint, h=200.0)

            win.sep()
            win.lb(bind=lambda: f"Событие: {log()}", h=44.0, wrap=True)
            d = win.lb("Правь виджеты мышью — сигналы идут следом. "
                       "Жми «Прочитать» — виджеты идут за сигналами.",
                       h=60.0, wrap=True)
            win.cls(d, "dim")
            d2 = win.lb("Часы меняются по таймеру, но поле даты и "
                        "текст правятся вручную без помех.",
                        h=60.0, wrap=True)
            win.cls(d2, "dim")

    win.every(1000.0, tick)
    win.css(CSS)
    win.go()


if __name__ == "__main__":
    main()
