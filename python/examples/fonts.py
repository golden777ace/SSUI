"""SSUI — настройка шрифтов.

Два уровня:
1. Весь app сразу — win.font(семейство, размер).
2. На виджет через CSS — font-family, font-size (с наследованием).
"""

import ssui

CSS = """
.card    { background: #171A21F2; radius: 16; padding: 22; gap: 12; }
label    { color: #EEF3FF; }

/* Наследуется всеми потомками блока */
.mono    { font-family: Consolas; }

/* Точечно на метку */
.big     { font-size: 34; }
.small   { font-size: 14; color: #9AA4B2; }
.serif   { font-family: Georgia; font-size: 26; }
"""


def main():
    # Базовый шрифт всего приложения. Меняет каждый виджет.
    win = ssui.W("SSUI · шрифты", 720, 620, thm="drk", glass=True, tint=0.1)
    win.font("Segoe UI", 20.0)  # семейство и базовый размер

    typed = ssui.sgnl("")
    size = ssui.sgnl(20)

    with win.bx(rad=16.0, pd=22.0, gp=12.0) as card:
        win.cls(card, "card")

        top = win.lb("Крупный заголовок", h=48.0)
        win.cls(top, "big")

        win.lb("Обычный текст базовым шрифтом приложения.", h=32.0)

        small = win.lb("Мелкая подпись — вторичный контент.", h=26.0)
        win.cls(small, "small")

        serif = win.lb("Georgia — засечки для акцента.", h=40.0)
        win.cls(serif, "serif")

        # Блок с моноширинным шрифтом — наследуют все дети.
        with win.bx(rad=12.0, pd=14.0, gp=8.0) as block:
            win.cls(block, "mono")
            win.lb("Моноширинный блок (Consolas):", h=28.0)
            win.tx("", h=44.0, sig=typed)
            win.lb(bind=lambda: f"Ввод: {typed()}", h=28.0)

        win.lb(bind=lambda: f"Размер приложения задан: {size()} px", h=26.0)
        win.lb("Шрифт применяется при сборке окна.", h=24.0)

    win.css(CSS)
    win.go()


if __name__ == "__main__":
    main()