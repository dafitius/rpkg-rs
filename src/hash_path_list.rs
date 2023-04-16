use anyhow::Error;
use std::collections::{HashMap};
use std::fs;

#[derive(Default)]
pub struct PathList {
    pub entries: HashMap<u64, Option<String>>,
}

impl PathList {
    pub fn new() -> Self
    {
        Self { entries: HashMap::new() }
    }

    pub fn parse_into(&mut self, path: &str) -> Result<&Self, Error> {
        let mut entries: HashMap<u64, Option<String>> = HashMap::new();

        let hash_list = fs::read_to_string(path).ok().unwrap();
        for line in hash_list.lines(){

            if line.starts_with('#'){ continue; }

            let hash = line.split_once('.').unwrap().0;
            let path = line.split_once(',');
            if let Some(path) = path {
                entries.insert(u64::from_str_radix(hash, 16).unwrap(), Some((path.1).to_string()));
            }
            else{
                entries.insert(u64::from_str_radix(hash, 16).unwrap(), None);
            }
        }
        self.entries = entries;
        Ok(self)
    }

    pub fn get_path(&self, key: &u64) -> Option<&String>
    {
        if let Some(value) = self.entries.get(key){
            if let Some(path) = value{
                return Some(path)
            }
            return None;
        }
        None
    }
}