# SSUI

GUI-библиотека для Windows на Rust с публичным API на Python.
Аппаратное ускорение через Direct3D 11 / Direct2D / DirectWrite,
реактивные сигналы, более 60 виджетов, четыре темы и CSS-подмножество.

## Возможности

- Retained-mode дерево виджетов, раскладка `grow`/`align`/`pin`
- Реактивность на сигналах: `bind`, `bindv`, `bindl`, `bindt`
- Канва с событиями мыши, прокруткой и панорамированием
- Дерево с колонками, множественным выбором и цветом ячеек
- Горячие клавиши, буфер обмена, нативные файловые диалоги
- Работа из фоновых потоков через `win.post()`
- Подсказки типов: `.pyi` и маркер `py.typed`

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

## Документация

Полный список виджетов, параметров и статус разработки —
[DESC.md](./DESC.md). Примеры — в каталоге `python/examples`.

## Лицензия

MIT © 2026 Sergievskiy Sergey. Полный текст — [LICENSE](./LICENSE).