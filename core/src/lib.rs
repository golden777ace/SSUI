//! Ядро рассчитано только на Windows. Модули с Win32/D3D скомпилируются
//! лишь под `cfg(windows)`.

#![cfg(windows)]

pub mod platform;
pub mod render;
pub mod tree;
