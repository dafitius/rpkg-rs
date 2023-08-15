use anyhow::Error;
use rayon::iter::ParallelIterator;
use rayon::prelude::ParallelBridge;
use std::collections::{HashMap, HashSet};
use std::fs::{self};

use crate::runtime::resource::runtime_resource_id::RuntimeResourceID;
use crate::utils;

use super::resource_id::ResourceID;

#[derive(Default)]
pub struct PathList {
    pub entries: HashMap<RuntimeResourceID, Option<ResourceID>>,
}

impl PathList {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    pub fn parse_into2(&mut self, path: &str) -> Result<&Self, Error> {
        let lines = utils::read_lines(path).ok().unwrap();

        let pairs: Vec<Option<(RuntimeResourceID, Option<ResourceID>)>> = lines
            .par_bridge()
            .map(
                |line_res| -> Option<(RuntimeResourceID, Option<ResourceID>)> {
                    let line = line_res.unwrap();
                    if line.starts_with('#') {
                        return None;
                    };

                    let hash = line.split_once('.').unwrap().0;
                    let path = line.split_once(',');
                    if let Some(path) = path {
                        let rid = ResourceID {
                            uri: (path.1).to_string(),
                        };
                        if rid.is_valid() {
                            return Some((
                                RuntimeResourceID {
                                    id: u64::from_str_radix(hash, 16).unwrap(),
                                },
                                Some(rid),
                            ));
                        }
                    }
                    Some((
                        RuntimeResourceID {
                            id: u64::from_str_radix(hash, 16).unwrap(),
                        },
                        None,
                    ))
                },
            )
            .collect();

        for line_res in pairs.into_iter().flatten() {
            self.entries.insert(line_res.0, line_res.1);
        }
        Ok(self)
    }

    pub fn parse_into(&mut self, path: &str) -> Result<&Self, Error> {
        let mut entries: HashMap<RuntimeResourceID, Option<ResourceID>> = HashMap::new();

        let hash_list = fs::read_to_string(path).ok().unwrap();
        for line in hash_list.lines() {
            if line.starts_with('#') {
                continue;
            }

            let hash = line.split_once('.').unwrap().0;
            let path = line.split_once(',');
            if let Some(path) = path {
                let rid = ResourceID {
                    uri: (path.1).to_string(),
                };
                if rid.is_valid() {
                    entries.insert(
                        RuntimeResourceID {
                            id: u64::from_str_radix(hash, 16).unwrap(),
                        },
                        Some(rid),
                    );
                    continue;
                }
            }
            entries.insert(
                RuntimeResourceID {
                    id: u64::from_str_radix(hash, 16).unwrap(),
                },
                None,
            );
        }
        self.entries = entries;
        Ok(self)
    }

    pub fn get_resource_id(&self, key: &RuntimeResourceID) -> Option<&ResourceID> {
        if let Some(value) = self.entries.get(key) {
            if let Some(path) = value {
                return Some(path);
            }
            return None;
        }
        None
    }

    pub fn get_files(&self, query: &str) -> HashSet<String> {
        let mut result = HashSet::default();
        for path in self.entries.values().flatten() {
            if let Some(path) = path.get_inner_most_resource_path() {
                if path.starts_with(query) {
                    let p: String = path.chars().skip(query.len() + 1).collect();
                    if !p.contains('/') {
                        result.insert(p);
                    }
                }
            }
        }
        result
    }

    pub fn get_folders(&self, query: &str) -> HashSet<String> {
        let mut result = HashSet::default();
        for path in self.entries.values().flatten() {
            if let Some(path) = path.get_inner_most_resource_path() {
                if path.starts_with(query) {
                    let path: String = path.chars().skip(query.len() + 1).collect();
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
}
