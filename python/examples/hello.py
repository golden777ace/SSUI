import ssui


def main():
    clicks = [0]

    win = ssui.Window("Hello SSUI", 800, 600)
    root = win.root()
    panel = win.frame(root, radius=16.0, padding=24.0, gap=12.0)
    win.label(panel, "Привет из Python!", height=40.0)

    def on_click():
        clicks[0] += 1
        print("Кликов:", clicks[0])

    win.button(panel, "Нажми меня", width=200.0, height=48.0, on_click=on_click)
    win.run()


if __name__ == "__main__":
    main()