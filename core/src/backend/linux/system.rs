//! Системные службы Linux-бэкенда: буфер обмена, чтение изображений,
//! файловые диалоги.
//!
//! Заменяет заглушки `render::stub` на рабочие реализации поверх
//! `arboard`, `skia_safe::Image` и `rfd`.
//!
//! Путь в репозитории: core/src/backend/linux/system.rs

use std::path::PathBuf;

use skia_safe::{Data, Image};

/// Кладёт текст в системный буфер обмена.
pub fn clipboard_set(text: &str) {
    if let Ok(mut cb) = arboard::Clipboard::new() {
        let _ = cb.set_text(text.to_string());
    }
}

/// Возвращает текст из системного буфера; переводы строк — `\n`.
pub fn clipboard_get() -> String {
    match arboard::Clipboard::new() {
        Ok(mut cb) => cb.get_text().unwrap_or_default().replace("\r\n", "\n"),
        Err(_) => String::new(),
    }
}

/// Читает изображение с диска; поддерживаются форматы, знакомые Skia.
pub fn load_image(path: &str) -> Option<Image> {
    let bytes = std::fs::read(path).ok()?;
    let data = Data::new_copy(&bytes);
    Image::from_encoded(data)
}

/// Число кадров в файле изображения; анимация пока не разбирается.
pub fn frame_count(_path: &str) -> u32 {
    1
}

/// Спецификация фильтра: подпись и список расширений без точки.
pub type Filter = (String, Vec<String>);

/// Диалог открытия файла; `multi` разрешает выбрать несколько.
pub fn open_file(title: &str, filters: &[Filter], multi: bool) -> Vec<PathBuf> {
    let mut dlg = rfd::FileDialog::new().set_title(title);
    for (name, exts) in filters {
        let list: Vec<&str> = exts.iter().map(|s| s.as_str()).collect();
        dlg = dlg.add_filter(name, &list);
    }
    if multi {
        dlg.pick_files().unwrap_or_default()
    } else {
        dlg.pick_file().map(|p| vec![p]).unwrap_or_default()
    }
}

/// Диалог сохранения файла.
pub fn save_file(title: &str, filters: &[Filter], name: &str) -> Option<PathBuf> {
    let mut dlg = rfd::FileDialog::new().set_title(title).set_file_name(name);
    for (label, exts) in filters {
        let list: Vec<&str> = exts.iter().map(|s| s.as_str()).collect();
        dlg = dlg.add_filter(label, &list);
    }
    dlg.save_file()
}

/// Диалог выбора папки.
pub fn pick_folder(title: &str) -> Option<PathBuf> {
    rfd::FileDialog::new().set_title(title).pick_folder()
}