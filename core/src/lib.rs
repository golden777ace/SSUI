//! Ядро кроссплатформенное: дерево, раскладка, CSS, темы и сигналы общие.
//! Платформенное скрыто за трейтами `backend`; Windows-реализация —
//! `platform`/`render`, Linux — `backend::linux`.

pub mod backend;
#[cfg(windows)]
pub mod platform;
pub mod render;
pub mod theme;
pub mod tree;