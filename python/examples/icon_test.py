"""SSUI — кнопка-иконка с уведомлением о сохранении."""

from pathlib import Path

import ssui

ICON = str(Path(__file__).with_name("save.png"))


def main():
    win = ssui.W("Сохранить", 1220, 620, thm="drk")

    with win.bx(pr=win.rt(), pd=16.0):
        win.bt("", icon=ICON, w=320.0, h=320.0,
               tip="Сохранить и сохранить и сохранить и сохранить и сохранить\n и сохранить и сохранить и сохранить и сохранить и сохранить", toast="Сохранено и сохранено и сохранено и сохранено и сохранено и сохранено")

    win.go()


if __name__ == "__main__":
    main()