import ssui

CSS = """
.root { padding: 20; gap: 12; radius: 18; }

button { radius: 12; }
button:hover { background: #4b8ff7; }

.cta { gradient: #22c55e #15803d h; color: #ffffff; radius: 14; shadow: 10; }

.card { radius: 14; padding: 16; gap: 12; }

.gv { gradient: #3b82f6 #1e3a8a v;  radius: 14; color: #ffffff; }
.gh { gradient: #22c55e #15803d h;  radius: 14; color: #ffffff; }
.gd { gradient: #f59e0b #7c2d12 d;  radius: 14; color: #ffffff; }
.gu { gradient: #ec4899 #831843 du; radius: 14; color: #ffffff; }

textbox { radius: 12; }
textbox:hover { background: #202a3a; }
textbox:focus { background: #17324f; }

.title { color: #cfe0ff; }
"""

LONG = (
    "Это длинная метка с включённым переносом по словам: текст "
    "разбивается на строки внутри отведённой области, а не обрезается."
)


def grad_panel(win, cls, text):
    with win.bx(rad=14.0, pd=16.0, gp=6.0, h=110.0) as p:
        win.cls(p, cls)
        win.lb(txt=text, h=26.0)


def main():
    vol = ssui.sgnl(0.0)
    name = ssui.sgnl("")
    fruit = ssui.sgnl("яблоко")
    opt = ssui.sgnl(False)
    row = ssui.sgnl("—")
    tabname = ssui.sgnl("Виджеты")
    msg = ssui.sgnl("ПКМ — меню · Space — тема · F12 — инспектор")

    fruits = ["яблоко", "банан", "вишня", "груша"]
    cols = ["Имя", "Роль", "Очки"]
    people = [
        ("Аня", "Дизайн", 128), ("Борис", "Backend", 204), ("Вера", "Frontend", 176),
        ("Глеб", "QA", 97), ("Дина", "PM", 152), ("Егор", "Backend", 143),
        ("Жанна", "Дизайн", 188), ("Зоя", "Data", 211), ("Игорь", "DevOps", 165),
        ("Катя", "Frontend", 133), ("Лев", "QA", 119), ("Мира", "PM", 198),
    ]
    data = [[n, r, str(s)] for (n, r, s) in people]

    win = ssui.W("SSUI · витрина возможностей", 900, 780, thm="drk")
    fx = win.fx()
    dlg = win.dlg()

    win.menu(
        ["Обновить", "Копировать", "Вставить", "Удалить"],
        on_select=lambda i: msg.st(f"Меню: {['Обновить','Копировать','Вставить','Удалить'][i]}"),
    )

    def ask():
        dlg(
            "Удалить запись?",
            "Действие необратимо. Длинное сообщение переносится по словам внутри окна диалога.",
            ["Отмена", "Удалить"],
            on=lambda i: msg.st("Запись удалена" if i == 1 else "Отменено"),
        )

    with win.bx(pd=20.0, gp=12.0) as root:
        win.cls(root, "root")

        title = win.lb(bind=lambda: f"Активная вкладка: {tabname()}", h=30.0)
        win.cls(title, "title")

        with win.tab(
            ["Виджеты", "Стили", "Данные"],
            ch=lambda i: tabname.st(["Виджеты", "Стили", "Данные"][i]),
            h=540.0,
        ):
            with win.bx(rad=14.0, pd=16.0, gp=12.0) as t1:
                win.cls(t1, "card")
                win.lb(bind=lambda: f"Громкость: {int(vol() * 100)}%", h=28.0)
                win.pr(bind=lambda: vol(), h=20.0)
                win.sl(0.0, ch=lambda v: vol.st(v), h=36.0)
                with win.bx(ax="h", gp=12.0, h=48.0):
                    win.bt("В 100%", clk=lambda: fx(vol, 1.0, dur=0.5))
                    win.bt("В 0%", clk=lambda: fx(vol, 0.0, dur=0.5, ease="io"))
                win.ch("Включить опцию", chk=False,
                       clk=lambda: opt.st(not opt()), h=30.0)
                win.lb(bind=lambda: f"Опция: {'вкл' if opt() else 'выкл'} · Имя: {name()}", h=28.0)
                win.tx(sig=name, h=42.0)
                win.dd(fruits, sel=0, ch=lambda i: fruit.st(fruits[i]), h=42.0)

            with win.bx(rad=14.0, pd=16.0, gp=12.0) as t2:
                win.cls(t2, "card")
                with win.bx(ax="h", gp=12.0, h=110.0):
                    grad_panel(win, "gv", "Вертикаль ↓")
                    grad_panel(win, "gh", "Горизонталь →")
                with win.bx(ax="h", gp=12.0, h=110.0):
                    grad_panel(win, "gd", "Диагональ ↘")
                    grad_panel(win, "gu", "Диагональ ↗")
                cta = win.bt("Кнопка с тенью и hover", h=48.0)
                win.cls(cta, "cta")
                win.lb(txt=LONG, wrap=True, h=76.0)

            with win.bx(rad=14.0, pd=16.0, gp=12.0) as t3:
                win.cls(t3, "card")
                win.lb(bind=lambda: f"Выбрано: {row()}", h=28.0)
                win.tbl(cols, data, ch=lambda i: row.st(data[i][0]), h=250.0)
                win.bt("Показать диалог", clk=ask, h=48.0)

        win.lb(bind=lambda: msg(), h=24.0)

    fx(vol, 0.6, dur=1.2, ease="out")

    win.go()


if __name__ == "__main__":
    main()