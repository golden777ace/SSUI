"""SSUI — буфер обмена.

Копирование текста из поля в системный буфер и вставка обратно.
Работает с внешними приложениями: скопируй здесь — вставь в блокнот,
и наоборот.

Путь в репозитории: python/examples/clip_test.py
"""

import ssui

CSS = """
.clear { background: #00000000; }
.head  { color: #EEF3FF; font-size: 22; }
.dim   { color: #9AA4B2; }
.ok    { color: #22C55E; }
"""


def main():
    win = ssui.W("SSUI · буфер обмена", 820, 560, thm="drk")
    clip = win.clip()

    note = ssui.sgnl("Серийный номер: 12345678")
    got = ssui.sgnl("—")
    status = ssui.sgnl("готово")

    def copy():
        clip.set(note())
        status.st(f"скопировано {len(note())} симв.")

    def paste():
        text = clip.get()
        got.st(text if text else "(пусто)")
        status.st(f"вставлено {len(text)} симв.")

    def swap():
        # Обмен: текущее поле в буфер, прежний буфер — в поле.
        prev = clip.get()
        clip.set(note())
        note.st(prev if prev else note())
        status.st("обмен выполнен")

    with win.bx(pr=win.rt(), ax="v", pd=16.0, gp=12.0) as root:
        win.cls(root, "clear")

        t = win.lb("Поле — источник для копирования", h=32.0)
        win.cls(t, "head")
        win.tx(sig=note, ph="введите текст", h=48.0)

        with win.bx(ax="h", gp=10.0, h=52.0) as bar:
            win.cls(bar, "clear")
            win.bt("Копировать", h=48.0, clk=copy)
            win.bt("Вставить", h=48.0, clk=paste)
            win.bt("Обмен", h=48.0, clk=swap)

        win.sep()
        win.lb("Из буфера:", h=28.0)
        g = win.lb(bind=lambda: got(), h=60.0, wrap=True)
        win.cls(g, "ok")

        win.sep()
        s = win.lb(bind=lambda: f"Статус: {status()}", h=28.0)
        win.cls(s, "dim")
        win.lb("Проверь связь с внешними программами: скопируй "
               "здесь и вставь в блокнот, затем наоборот.",
               h=48.0, wrap=True)

    win.css(CSS)
    win.go()


if __name__ == "__main__":
    main()
