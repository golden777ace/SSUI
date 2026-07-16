# Производительность SSUI

Путь в репозитории: `docs/OPTIMIZATION.md`

Цель — выигрывать у Tkinter каждую измеримую дисциплину:
виджеты, таблицы, канвас, тексты, реактивность, память, старт, анимация.

---

## 1. Как измеряем

Эталон — `benchmarks/benchmark.py` (SSUI vs Tkinter, Windows).
Каждый замер — отдельный процесс, окна закрываются автоматически.

| Дисциплина | Что меряется |
|---|---|
| import | импорт библиотеки, мс |
| build | дерево из 1000 виджетов, мс |
| table | таблица 10×300, мс |
| signal | 50 000 обновлений реактивной строки, мс |
| canvas | Canvas из 3000 фигур (rect/circle/line), мс |
| mem | RSS после 2000 виджетов, МБ |
| window | от старта процесса до показа окна, мс |
| anim | CPU % за 3 с анимации `fx` при 400 виджетах + кадры |

Запуск: `python benchmarks/benchmark.py` (опции `--only`, `--repeat`, `--json`).

---

## 2. Что уже хорошо — не трогать

- Одна кэшированная `ID2D1SolidColorBrush` + `SetColor` (canvas.rs).
- Flip-model swap chain `FLIP_DISCARD`, BufferCount=2.
- Событийная модель: `GetMessageW`, `InvalidateRect` только при изменениях.
- Виртуализация строк в List и Term.
- Кэш геометрии раскладки, кэш битмапов WIC.
- `SetDpi(96,96)` — координаты 1:1 с пикселями.

---

## 3. Находки

### P0-1. Кэш текстовых layout'ов — главный выигрыш

`core/src/render/canvas.rs::draw_text` вызывает `rt.DrawText`.
D2D внутри создаёт `IDWriteTextLayout` (шейпинг, метрики, переносы)
на каждый вызов, каждый кадр, для каждого текста. Это основная
стоимость любого D2D-интерфейса с текстом.

Решение: `LayoutCache` в Renderer.

```rust
struct TextKey { text: Vec<u16>, fmt: u8, width_bits: u32 }
cache: HashMap<TextKey, IDWriteTextLayout>
```

- `draw_text` берёт layout из кэша, рисует `DrawTextLayout`.
- Инвалидация: смена текста, ширины, темы, DPI; лимит записей + LRU.
- Туда же перевести `text_width` (hover у Link),
  `wrapped_caret`/`wrapped_ranges` (TextArea) — сейчас они тоже
  строят layout каждый кадр.

Эффект: 3–10× на текстовых сценах (Table, List, Term, формы).
Затрагивает Label, Button, Group, Link, List, Table, Term.

### P0-2. Canvas: линии и круги нативными примитивами

`core/src/render/device.rs`, ветка `NodeKind::Canvas`:

- kind==2 (line): линия рисуется цепочкой `fill_rounded_rect`
  с шагом ~0.4·w — сотни draw-call'ов на одну линию, рваные диагонали.
  Заменить на один `rt.DrawLine` (добавить `Canvas::draw_line`).
- kind==1 (circle) со stroke: используется `stroke_rect` —
  контур получается квадратным. Добавить `Canvas::stroke_ellipse`
  / `fill_ellipse` через `DrawEllipse`/`FillEllipse`.

Эффект: 10–100× на канвас-сценах, плюс корректные круги
и гладкие линии. Дисциплина canvas выигрывается этим пунктом.

### P0-3. Кэш градиентных кистей

`canvas.rs::fill_rounded_gradient` создаёт `GradientStopCollection`
и `LinearGradientBrush` при каждом вызове каждый кадр — это
создание device-объектов в горячем пути.

Решение: `HashMap<(u32,u32), ID2D1LinearGradientBrush>` по паре
цветов; точки задавать через `SetStartPoint`/`SetEndPoint` перед
использованием. Сброс кэша при пересоздании render target.

### P1-4. Отсечение невидимых узлов при обходе

Клип прячет пиксели, но CPU обходит всё дерево и шлёт draw-call'ы.
В обходе отрисовки пропускать узел (и поддерево), если
`node.rect` не пересекает видимую область (с запасом на тень/elev).
List и Term уже виртуализованы — распространить принцип на
Scroll Area с сотнями детей.

### P1-5. Динамический таймер вместо вечных 60 Гц

`core/src/platform/window.rs`: `WM_TIMER` живёт всегда,
даже в простое — лишние пробуждения CPU и расход батареи.
Считать «нужен ли тик» (fx-анимации, Spinner, Toast, каретка):
если нечего анимировать N тиков подряд — `KillTimer`;
`SetTimer` при вводе, старте fx, появлении Spinner/Toast.
Цель — 0% CPU в простое.

### P1-6. Waitable swap chain

`DXGI_SWAP_CHAIN_FLAG_FRAME_LATENCY_WAITABLE_OBJECT` +
`SetMaximumFrameLatency(1)`. Убирает ~кадр задержки ввода —
слайдеры и drag перестают «липнуть».

### P1-7. Точечный refresh вместо refresh_all

`python/src/lib.rs::refresh_all` пересчитывает ВСЕ биндинги
(тексты, значения, canvas-bind) на любое событие — O(всё дерево)
Python-вызовов на клик. Решение: карта сигнал→подписчики,
`sig.st(v)` будит только своих. Дисциплина signal и отзывчивость
больших форм зависят от этого пункта.

### P2-8. ClearType для непрозрачных окон

Сейчас grayscale AA (нужен для glass). Для окон без
прозрачности включать `D2D1_TEXT_ANTIALIAS_MODE_CLEARTYPE` —
мелкий текст заметно чётче, чем у Tkinter.

### P2-9. Парсинг цвета фигур один раз

Убедиться, что hex-цвет `ShapeSpec` парсится в `make_shapes`,
а не в цикле отрисовки (`Color::hexa` на фигуру на кадр).
Плюс переиспользовать scratch-буферы вместо аллокаций на кадр.

---

## 4. Целевые цифры (после P0–P1)

| Дисциплина | Tkinter (типично) | Цель SSUI |
|---|---|---|
| import | 50–120 мс | ≤ 30 мс |
| build 1000 | 300–900 мс | ≤ 40 мс |
| table 10×300 | 150–400 мс | ≤ 20 мс |
| signal 50k | 200–600 мс | ≤ 150 мс |
| canvas 3000 | 100–500 мс | ≤ 30 мс |
| mem 2000 | 25–40 МБ | ≤ 35 МБ |
| window | 150–350 мс | ≤ 80 мс |
| anim CPU | 15–40 % | ≤ 5 % |

Числа Tkinter зависят от машины — сначала снять базу
бенчмарком на целевой машине, потом сравнивать с собой.

---

## 5. Порядок работ

1. Снять базу: `python benchmarks/benchmark.py --json base.json`.
2. Батч P0 (1–3) одним заходом, прогнать бенчмарк, сравнить.
3. P1 (4–7) по одному, замер после каждого.
4. P2 (8–9) — полировка.
5. Обновить `showcase.py` не требуется: API не меняется.

## 6. Файлы для точных патчей (Было/Стало)

- `core/src/render/canvas.rs`
- `core/src/render/device.rs`
- `core/src/platform/window.rs`
- `core/src/tree/mod.rs`
- `python/src/lib.rs`

## 7. Строки для DESC.md

```
## Производительность
- [ ] Кэш IDWriteTextLayout (draw_text, text_width, wrapped_*)
- [ ] Canvas: DrawLine / DrawEllipse вместо цепочек прямоугольников
- [ ] Кэш градиентных кистей
- [ ] Culling невидимых узлов при отрисовке
- [ ] Динамический таймер (0% CPU в простое)
- [ ] Waitable swap chain, латентность 1 кадр
- [ ] Точечный refresh по подписчикам сигнала
- [ ] ClearType для непрозрачных окон
- [ ] benchmarks/benchmark.py в CI-прогон вручную
```
