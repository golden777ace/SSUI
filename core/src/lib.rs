//! # ssui-core
//!
//! Нативное ядро SSUI. Пока (Фаза 0) содержит два слоя:
//!
//! - [`platform`] — работа с Windows: окно, цикл сообщений, DPI.
//! - [`render`]   — устройство Direct3D 11, swap chain, очистка экрана.
//!
//! Следующие слои (layout, events, style, memory, виджеты) добавляются
//! по дорожной карте (см. ROADMAP.md).
//!
//! Ядро рассчитано только на Windows. Модули с Win32/D3D скомпилируются
//! лишь под `cfg(windows)`.

#![cfg(windows)]

pub mod platform;
pub mod render;
