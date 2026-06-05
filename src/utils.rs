use std::ffi::OsString;
use std::fs;
use std::path::Path;

pub(crate) fn read_file_names(path: &Path) -> Vec<OsString> {
    match fs::read_dir(path) {
        Ok(entries) => entries
            .flatten()
            .filter(|dir_entry| dir_entry.file_type().is_ok_and(|file| file.is_file()))
            .map(|entry| entry.file_name())
            .collect::<Vec<_>>(),
        Err(_) => {
            vec![]
        }
    }
}

pub(crate) fn uppercase_first_letter(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}
