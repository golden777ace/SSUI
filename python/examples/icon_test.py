"""SSUI — кнопка-иконка с уведомлением о сохранении."""

from pathlib import Path

import ssui

ICON = str(Path(__file__).with_name("save.png"))


def main():
    win = ssui.W("Сохранить", 220, 220, thm="drk")

    with win.bx(pr=win.rt(), pd=16.0):
        win.bt("", icon=ICON, w=100.0, h=100.0,
               tip="Сохранить", toast="Сохранено")

    win.go()


if __name__ == "__main__":
    main()