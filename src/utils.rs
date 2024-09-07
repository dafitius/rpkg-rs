use std::ffi::OsString;
use std::fs;
use std::path::{Component, Path, PathBuf};

pub fn read_file_names(path: &Path) -> Vec<OsString> {
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

pub fn normalize_path(path: &Path) -> PathBuf {
    let mut components = path.components().peekable();
    let mut ret = if let Some(c @ Component::Prefix(..)) = components.peek().cloned() {
        components.next();
        PathBuf::from(c.as_os_str())
    } else {
        PathBuf::new()
    };

    for component in components {
        match component {
            Component::Prefix(..) => unreachable!(),
            Component::RootDir => {
                ret.push(component.as_os_str());
            }
            Component::CurDir => {}
            Component::ParentDir => {
                ret.pop();
            }
            Component::Normal(c) => {
                ret.push(c);
            }
        }
    }
    ret
}

pub(crate) fn uppercase_first_letter(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}