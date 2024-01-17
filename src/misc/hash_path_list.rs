use anyhow::Error;
use std::collections::{HashMap};
use std::fs;

#[derive(Default)]
pub struct PathList {
    pub entries: HashMap<RuntimeResourceID, Option<ResourceID>>,
}

impl PathList {
    pub fn new() -> Self
    {
        Self { entries: HashMap::new() }
    }

    pub fn parse_into(&mut self, path: &str, check_md5: bool) -> Result<&Self, Error> {
        let mut entries: HashMap<RuntimeResourceID, Option<ResourceID>> = HashMap::new();

        let pairs: Vec<Option<(RuntimeResourceID, Option<ResourceID>)>> = lines.par_bridge().map(|line_res| -> Option<(RuntimeResourceID, Option<ResourceID>)> {
            let binding = line_res.unwrap();
            let line = binding.as_str();
            if line.starts_with('#') { return None; };

            let (hash, path) = match line.split_once(',') {
                Some((h, p)) => (h.split_once('.').unwrap().0, Some(p)),
                None => (line, None),
            };

            let id = u64::from_str_radix(hash, 16).unwrap();
            if let Some(path) = path {
                let rid = ResourceID { uri: (path).to_string() };
                if !check_md5 {
                    if rid.is_valid() {
                        return Some((RuntimeResourceID { id }, Some(rid)));
                    }
                }
                else {
                    if rid.is_valid_rrid(RuntimeResourceID { id }) {
                        return Some((RuntimeResourceID { id }, Some(rid)));
                    }
                }

            }
            Some((RuntimeResourceID { id }, None))
        }).collect();

        for pair in pairs {
            if let Some((key, value)) = pair {
                self.entries.insert(key, value);
            }
        }
        self.entries = entries;
        Ok(self)
    }
    pub fn get_resource_id(&self, key: &RuntimeResourceID) -> Option<&ResourceID>
    {
        if let Some(value) = self.entries.get(key){
            if let Some(path) = value{
                return Some(path)
            }
            return None;
        }
        None
    }

    pub fn get_files(&self, query: &str) -> HashSet<String> {
        let mut result = HashSet::default();
        for path in self.entries.values() {
            if let Some(path) = path {
                if let Some(path) = path.get_inner_most_resource_path() {
                    if path.starts_with(query) {
                        let p: String = path.chars().skip(query.len() + 1).collect();
                        if !p.contains('/') {
                            result.insert(p);
                        }
                    }
                }
            }
        }
        result
    }

    pub fn get_folders(&self, query: &str) -> HashSet<String> {
        let mut result = HashSet::default();
        for path in self.entries.values() {
            if let Some(path) = path {
                if let Some(path) = path.get_inner_most_resource_path() {
                    if path.starts_with(query) {
                        let path: String = path.chars().skip(query.len() + 1).collect();
                        if let Some(n) = path.find('/'){
                            let p: String = path.chars().take(n).collect();
                            if !p.contains('.') {
                                result.insert(p);
                            }
                        }
                    }
                }
            }
        }
        result
    }

    pub fn get_all_folders(&self) -> HashSet<String>{
        let mut results: HashSet<String> = HashSet::new();

        for (_, res_id) in self.entries.iter() {
            if let Some(res_id) = res_id{
                if let Some(path) = res_id.get_path(){
                    results.insert(path);
                }
            }
        }
        results
    }
}
