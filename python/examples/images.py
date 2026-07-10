import os

import ssui


def find_image():
    candidates = [
        "logo.png",
        "test.png",
        "test.jpg",
        os.path.expanduser("~/Pictures/test.png"),
    ]
    for p in candidates:
        if os.path.exists(p):
            return p
    return "logo.png"


def main():
    img_path = find_image()
    names = ["contain", "cover", "fill", "center"]
    fit = ssui.sgnl(0.0)

    win = ssui.W("SSUI — изображения", 720, 600, thm="drk")

    with win.bx(rad=16.0, pd=16.0, gp=12.0):
        win.lb(bind=lambda: f"Файл: {img_path}", h=24.0)
        win.lb(bind=lambda: f"Режим: {names[int(fit()) & 3]}", h=24.0)

        with win.bx(rad=12.0, pd=8.0, h=340.0):
            win.img(img_path, fit_bind=lambda: fit(), h=320.0)

        with win.bx(rad=12.0, ax="h", gp=8.0, h=52.0):
            for i, nm in enumerate(names):
                win.bt(nm, h=44.0, clk=lambda i=i: fit.st(float(i)))

        win.lb("Слайдер тоже меняет режим:", h=22.0)
        win.sl(0.0, ch=lambda v: fit.st(float(round(v * 3.0))), h=36.0)
        win.bt("Тест", icon=r"C:\Users\sergism\Desktop\SSUI\test.png", h=44.0)

    win.go()


if __name__ == "__main__":
    main()