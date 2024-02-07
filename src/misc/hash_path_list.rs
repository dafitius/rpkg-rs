use std::collections::{HashMap, HashSet};
use std::fs::read_to_string;
use std::path::Path;
use rayon::prelude::{IntoParallelIterator};
use rayon::iter::ParallelIterator;
use thiserror::Error;
use crate::misc::resource_id::ResourceID;
use crate::runtime::resource::runtime_resource_id::RuntimeResourceID;

#[derive(Debug, Error)]
pub enum PathListError {
    #[error("{0}")]
    IoError(#[from] std::io::Error),

    #[error("Invalid RuntimeResourceID entry")]
    InvalidRuntimeResourceID,
}

#[derive(Default)]
pub struct PathList {
    pub entries: HashMap<RuntimeResourceID, Option<ResourceID>>,
}

impl PathList {
    pub fn new() -> Self
    {
        Self { entries: HashMap::new() }
    }

    pub fn parse_into(&mut self, path: &Path, check_md5: bool) -> Result<&Self, PathListError> {
        let file_as_string = read_to_string(path).map_err(PathListError::IoError)?;
        let lines: Vec<_> = file_as_string
            .lines()
            .map(String::from)
            .collect();

        let lines_par = lines.into_par_iter();

        self.entries = lines_par.filter_map(|line_res| {
            if line_res.starts_with('#') { return None; };

            let (hash, path) = match line_res.split_once(',') {
                Some((h, p)) => (h.split_once('.').unwrap().0, Some(p)),
                None => (line_res.as_str(), None),
            };

            if let Ok(id) = u64::from_str_radix(hash, 16) {
                if let Some(path) = path {
                    let rid = ResourceID { uri: (path).to_string() };
                    if !check_md5 {
                        if rid.is_valid() {
                            return Some((RuntimeResourceID { id }, Some(rid)));
                        }
                    } else if id == RuntimeResourceID::from_resource_id(&rid).id {
                        return Some((RuntimeResourceID { id }, Some(rid)));
                    }
                }
                Some((RuntimeResourceID { id }, None))
            } else { None }
        }).collect::<Vec<_>>().into_iter().collect();

        Ok(self)
    }
    pub fn get_resource_id(&self, key: &RuntimeResourceID) -> Option<&ResourceID>
    {
        if let Some(value) = self.entries.get(key) {
            if let Some(path) = value {
                return Some(path);
            }
            return None;
        }
        None
    }

    pub fn get_files(&self, query: &str) -> HashSet<String> {
        let mut query_filtered = query.to_ascii_lowercase();
        query_filtered.retain(|c| c as u8 > 0x1F);
        let mut result = HashSet::default();
        for path in self.entries.values().flatten() {
            if let Some(path) = path.get_inner_most_resource_path() {
                if path.starts_with(&query_filtered) {
                    let p: String = path.chars().skip(query_filtered.len() + 1).collect();
                    if !p.contains('/') {
                        result.insert(p);
                    }
                }
            }
        }
        result
    }

    pub fn get_folders(&self, query: &str) -> HashSet<String> {
        let mut query_filtered = query.to_ascii_lowercase();
        query_filtered.retain(|c| c as u8 > 0x1F);
        let mut result = HashSet::default();
        for path in self.entries.values().flatten() {
            if let Some(path) = path.get_inner_most_resource_path() {
                if path.starts_with(&query_filtered) {
                    let path: String = path.chars().skip(query_filtered.len() + 1).collect();
                    if let Some(n) = path.find('/') {
                        let p: String = path.chars().take(n).collect();
                        if !p.contains('.') {
                            result.insert(p);
                        }
                    }
                }
            }
        }
        result
    }

    pub fn get_all_folders(&self) -> HashSet<String> {
        let mut results: HashSet<String> = HashSet::new();

        for (_, res_id) in self.entries.iter() {
            if let Some(res_id) = res_id {
                if let Some(path) = res_id.get_path() {
                    results.insert(path);
                }
            }
        }
        results
    }
}
