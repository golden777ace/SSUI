"""SSUI — файловые диалоги: открыть, сохранить, папка.

Диалоги нативные (Windows Common Item Dialog). Отмена возвращает
пустую строку. Колбэк приходит после закрытия диалога.

Путь в репозитории: python/examples/file_test.py
"""

import ssui

CSS = """
.clear { background: #00000000; }
.glass { background: #16213ab3; radius: 24; padding: 26; gap: 14; shadow: 30; }
.head  { color: #EEF3FF; font-size: 22; }
.dim   { color: #9AA4B2; }
.ok    { color: #22C55E; }
"""

PATTERNS = [
    ("Конфигурация CSV", "*.csv"),
    ("Текст", "*.txt"),
    ("Все файлы", "*.*"),
]


def main():
    win = ssui.W("SSUI · файловые диалоги", 860, 520, thm="drk", glass=True, tint=0.0)
    fl = win.file()

    path = ssui.sgnl("—")
    kind = ssui.sgnl("нет")

    def result(k):
        def on(p):
            kind.st(k)
            path.st(p if p else "(отменено)")
        return on

    def do_open():
        fl.open(title="Открыть профиль счётчика",
                patterns=PATTERNS, on=result("открыть"))

    def do_save():
        fl.save(title="Сохранить профиль",
                name="profile.csv",
                patterns=PATTERNS, on=result("сохранить"))

    def do_dir():
        fl.dir(title="Папка для выгрузки", on=result("папка"))

    with win.bx(pr=win.rt(), ax="v", pd=16.0, gp=12.0) as root:
        win.cls(root, "clear")

        t = win.lb("Диалоги файловой системы", h=34.0)
        win.cls(t, "head")
        win.lb("Каждая кнопка открывает нативный диалог. "
               "Результат появится ниже после закрытия.",
               h=44.0, wrap=True)

        with win.bx(ax="h", gp=10.0, h=54.0) as bar:
            win.cls(bar, "clear")
            win.bt("Открыть файл", h=50.0, clk=do_open)
            win.bt("Сохранить как", h=50.0, clk=do_save)
            win.bt("Выбрать папку", h=50.0, clk=do_dir)

        win.sep()
        k = win.lb(bind=lambda: f"Тип диалога: {kind()}", h=30.0)
        win.cls(k, "dim")
        p = win.lb(bind=lambda: f"Путь: {path()}", h=80.0, wrap=True)
        win.cls(p, "ok")

        win.sep()
        win.lb("Отмена диалога возвращает пустую строку — "
               "здесь показывается как «(отменено)».",
               h=44.0, wrap=True)

    win.css(CSS)
    win.go()


if __name__ == "__main__":
    main()
