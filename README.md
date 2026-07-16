# SSUI

GUI-библиотека для Windows на Rust с публичным API на Python.

## Требования

- Windows 10/11
- Python 3.11+

## Установка

```bash
pip install ssui
```

## Пример

```python
import ssui

def main():
    win = ssui.W("Привет, SSUI", 640, 480, thm="drk")

    with win.bx(pd=20.0, gp=12.0):
        win.lb("Привет, мир")
        win.bt("Кнопка", clk=lambda: print("клик"))

    win.go()

if __name__ == "__main__":
    main()
```

## Сборка из исходников

```bash
python -m venv .venv
.venv\Scripts\activate
pip install maturin
maturin develop
python python/examples/showcase.py
```

## Документация

Полный список виджетов и статус разработки — [DESC.md](./DESC.md).

## Лицензия

MIT © Sergievskiy Sergey