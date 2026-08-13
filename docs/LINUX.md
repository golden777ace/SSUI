# SSUI на Linux

Путь в репозитории: `docs/LINUX.md`

Принцип: ядро (дерево, раскладка, CSS, темы, сигналы, виджеты)
остаётся общим. Платформенные слои уходят за трейты.
Windows-код не переписывается — переезжает в свой backend.

---

## 1. Слои

| Слой | Windows (есть) | Linux (план) |
|---|---|---|
| Окно и ввод | Win32 (window.rs) | winit 0.30 (X11 + Wayland) |
| GPU-поверхность | D3D11 + DXGI | OpenGL через glutin 0.32 |
| 2D-рисование | Direct2D | skia-safe 0.80 (feature `gl`) |
| Текст | DirectWrite | SkParagraph (feature `textlayout`) |
| Картинки | WIC | Skia codecs (PNG/JPEG) |
| Прозрачность | DirectComposition | ARGB visual / Wayland alpha |
| Размытие | ACCENT_* | только KDE-протокол, иначе нет |
| Clipboard | Win32 | arboard 3 |
| Диалоги | Win32 dialogs | ashpd 0.9 (xdg-portal), fallback zenity |
| Drag&drop | Win32 | winit `DroppedFile` |
| DPI | per-monitor v2 | winit `scale_factor` |

## 2. Почему Skia

- Качество 2D уровня Direct2D; SkParagraph ≈ DirectWrite
  (шейпинг, переносы, каретка, hit-test) — критично для
  TextArea/Term.
- Один Painter покрывает всё, что уже умеет Canvas:
  rounded rect, ellipse, line, gradient, clip, bitmap, text.
- `skia-binaries` качаются готовыми — Skia не собираем.

Альтернативы и почему нет:
- wgpu + lyon + cosmic-text: всё писать самим (заливки,
  градиенты, кэш глифов), месяцы работы.
- cairo + pango: CPU-рендер, проигрыш по скорости — а цель
  выигрывать бенчмарки.
- femtovg: слабый текст, нет параграфов.

Цена: linux-wheel потяжелеет до ~20 МБ (у Windows 3–5 МБ).

## 3. Архитектура: трейты

`core/src/backend/mod.rs` — интерфейсы;
`backend/win/` — текущий код; `backend/linux/` — новый.

```rust
pub trait Painter {
    fn clear(&mut self, c: Color);
    fn fill_rounded_rect(&mut self, r: Rect, rad: f32, c: Color);
    fn stroke_rect(&mut self, r: Rect, w: f32, c: Color);
    fn fill_ellipse(&mut self, r: Rect, c: Color);
    fn stroke_ellipse(&mut self, r: Rect, w: f32, c: Color);
    fn draw_line(&mut self, a: Point, b: Point, w: f32, c: Color);
    fn fill_rounded_gradient(&mut self, r: Rect, rad: f32,
        c0: Color, c1: Color, dir: u8);
    fn draw_text(&mut self, key: &TextKey, r: Rect, c: Color);
    fn push_clip(&mut self, r: Rect);
    fn pop_clip(&mut self);
    fn draw_bitmap(&mut self, img: ImageId, r: Rect, fit: u8);
}

pub trait TextEngine {
    fn width(&mut self, key: &TextKey) -> f32;
    fn caret(&mut self, key: &TextKey, pos: usize) -> Point;
    fn hit(&mut self, key: &TextKey, p: Point) -> usize;
    fn ranges(&mut self, key: &TextKey, a: usize, b: usize) -> Vec<Rect>;
}

pub trait PlatformWindow {
    fn run(self);                       // блокирующий цикл
    fn request_redraw(&self);
    fn set_title(&self, t: &str);
}

pub enum Event { Mouse(..), Key(..), Char(..), Resize(..),
    Timer, Close, Dropped(PathBuf) }
```

`TextKey { text: Vec<u16>, fmt: u8, width_bits: u32 }` — общий
с кэшем layout'ов из docs/OPTIMIZATION.md (P0-1): оба бэкенда
кэшируют layout (IDWriteTextLayout / skia Paragraph) по одному ключу.

## 4. Cargo

Реализовано: фича `linux-skia` в `core` и `python`, по умолчанию
выключена. Сборка на Linux: `maturin develop --release --features
linux-skia`. На Windows фича не влияет ни на что.

```toml
[features]
default = []
win-native = []
linux-skia = ["winit", "glutin", "skia-safe", "arboard", "ashpd"]

[target.'cfg(windows)'.dependencies]
windows = { version = "0.62", features = [ ... ] }

[target.'cfg(target_os = "linux")'.dependencies]
winit = "0.30"
glutin = "0.32"
skia-safe = { version = "0.80", features = ["gl", "textlayout"] }
arboard = "3"
ashpd = "0.9"
```

`core/src/lib.rs` выбирает backend по `cfg(target_os)`.
Python-API (`python/src/lib.rs`) не меняется вообще.

## 5. Wheels и CI

Сделано: `.github/workflows/wheels.yml` — матрица
`windows-latest` × `ubuntu-22.04`, Python 3.11–3.13,
тег `manylinux_2_28`, кэш `skia-binaries`.

- maturin, тег `manylinux_2_28` (skia-safe требует свежий clang).
- `skia-binaries` кэшировать в CI (архив ~100 МБ).
- GitHub Actions matrix: `windows-latest`, `ubuntu-22.04`;
  артефакты — wheels обеих платформ.
- README: пометить размер linux-wheel (~20 МБ, Skia).

## 6. Фазы

- **L0 — Трейты.** Выделить Painter/TextEngine/PlatformWindow,
  Windows-код переезжает в `backend/win/` без изменения
  поведения. Критерий: showcase.py на Windows пиксель-в-пиксель,
  бенчмарки не просели.
- **L1 — Окно.** winit + glutin + GL-контекст, Skia surface,
  `clear` работает. Критерий: пустое окно 60 FPS, resize, DPI.
  Сделано: `core/src/backend/linux/mod.rs` — `Window::show()`
  создаёт окно в `resumed`, Skia-поверхность пересоздаётся на
  `Resized`, кадр чистится фоном темы дерева.
- **L2 — Painter.** Все примитивы на Skia. Критерий:
  тест-сцена Canvas (rect/circle/line/gradient) идентична Windows.
  Сделано: `backend/linux/painter.rs` — `SkiaPainter`, формат текста
  `TextFormat`; текст пока однострочный, параграфы придут в L3.
- **L3 — Текст.** SkParagraph + кэш по TextKey; width/caret/
  hit/ranges. Критерий: Label, Button, TextBox, TextArea, Term.
  Сделано: `backend/linux/text.rs` — `SkiaText`, кэш параграфов по
  «текст + формат + ширина + цвет», индексы UTF-16 ↔ UTF-8.
- **L4 — Виджеты.** Делится на шаги: L4-a — абстракция форматов
  (`FormatSource`, сделано: `WinFormats` и `SkiaFormats`); L4-b —
  реализации `TextEngine`: `WinText` (DirectWrite) и `SkiaText`
  (SkParagraph), обе сделаны; далее вызовы в `device.rs` переходят
  на трейт;
  L4-c — обход дерева становится generic по `Painter`; на Linux
  обход уже работает (`backend/linux/render.rs`), раскладка идёт
  через `Tree::layout`. Долг: константы `SCROLLBAR_W`, `ACC_HEADER`,
  `GROUP_HEADER`, `DOCK_HEADER`, `SPLIT_W`, `SPLIT_ARROW`, `BAR_ITEM`
  продублированы в `device.rs` и `backend/linux/render.rs` — свести
  в один модуль при переводе `device.rs` на `paint.rs`. Начат:
  `core/src/render/paint.rs` — `PaintCtx`, `ImageSource`, ветки
  Frame/Image/Label/Toggle. Остальные ветки переносятся по одной.
  Перенос всех веток `NodeKind` из `device.rs`
- **L5 — Интеграции.** Clipboard (arboard), диалоги (portal),
  drag&drop, IME (Wayland text-input-v3; XIM ограниченно).
- **L6 — Выпуск.** Wheels в CI, README/DESC, чек-лист
  паритета, известные ограничения.

## 7. Честные ограничения

- Mica/Acrylic — только Windows. На Linux `glass=True` даёт
  альфа-фон; размытие — только KDE (протокол blur), на GNOME нет.
- Wayland может не дать серверных декораций — рисуем свои
  (кастомный titlebar уже есть, переиспользуем).
- IME: полноценно на Wayland; X11/XIM — базово, поздняя фаза.
- Скринридеры (AT-SPI) — вне рамок этого плана.

## 8. Файлы для L0 (прислать целиком)

- `core/Cargo.toml`, `python/Cargo.toml`
- `core/src/lib.rs`
- `core/src/platform/window.rs`, `core/src/platform/dpi.rs`
- `core/src/render/device.rs`, `core/src/render/canvas.rs`,
  `core/src/render/types.rs`
- `python/src/lib.rs`
- `core/src/tree/mod.rs` — по запросу (большой)

## 9. Строки для DESC.md

```
## Linux
- [ ] L0: трейты Painter/TextEngine/PlatformWindow, backend/win
- [ ] L1: winit + glutin + Skia surface
- [ ] L2: примитивы Painter на Skia
- [ ] L3: SkParagraph + кэш TextKey
- [ ] L4: паритет виджетов, showcase.py на Linux
- [ ] L5: clipboard, порталы, drag&drop, IME
- [ ] L6: manylinux_2_28 wheels в CI, документация
```
