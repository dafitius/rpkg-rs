use serde::Serialize;
use crate::runtime::resource::runtime_resource_id::RuntimeResourceID;
use crate::encryption::md5_engine::Md5Engine;

#[derive(Serialize, Clone, Debug)]
pub struct ResourceID
{
    pub uri: String,
}

impl ResourceID {
    pub fn from_string(source: &str) -> Self {
        Self {
            uri: source.to_string()
        }
    }

    pub fn create_derived(&self, extension: &str, parameters: &str, platform: &str) -> ResourceID {
        let mut derived = format!("[{}.{}]", self.uri, extension);
        if !platform.is_empty() {
            derived += format!("({})", platform).as_str();
        }
        derived += ".pc_";
        if !parameters.is_empty() {
            derived += parameters;
        }

        ResourceID {
            uri: derived
        }
    }

    pub fn get_inner_most_resource_path(&self) -> Option<String> {
        let open_count = self.uri.chars().filter(|c| *c == '[').count();
        match self.uri.find(']'){
            Some(n) => {
                Some(self.uri.chars().skip(open_count).take(n-1).collect())
            },
            None => {None}
        }
    }

    fn find_matching_parentheses(str: &str, start_index: usize, open: char, close: char) -> Option<usize> {
        let mut open_count = str.chars().skip(start_index).filter(|c| *c == open).count();
        for (i, c) in str.chars().skip(start_index).enumerate() {
            if c == close {
                match open_count == 1 {
                    true => { return Some(i); }
                    false => { open_count -= 1 }
                }
            }
        }
        None
    }

    pub fn get_protocol(&self) -> Option<String> {
        match self.uri.find(':') {
            Some(n) => {
                let protocol: String = self.uri.chars().take(n).collect();
                Some(protocol.replace('[', ""))
            },
            None => {None}
        }
    }

    pub fn get_derived_end_index(&self) -> Option<usize>{
        if let Some(n) = Self::find_matching_parentheses(&self.uri, 0, '[', ']') {
            if self.uri.chars().nth(n + 1).unwrap_or(' ') == '(' {
                if let Some(n2) = Self::find_matching_parentheses(&self.uri, n+1, '(', ')') {
                    return Some(n2);
                }
            }
            else {
                return Some(n);
            }
        }
        None
    }

    pub fn get_path(&self) -> Option<String>{
        let path: String = self.uri.chars().skip(1).collect();
        if let Some(n) = path.rfind('/'){
            let p: String = path.chars().take(n).collect();
            if !p.contains('.') {
                return Some(p);
            }
        }
        return None;
    }

    pub fn is_empty(&self) -> bool {
        self.uri.is_empty()
    }

    pub fn is_valid(&self) -> bool {
        {
            !self.uri.contains("unknown") &&
                !self.uri.contains("*") &&
                self.uri.starts_with('[') &&
                self.uri.contains("].pc_")
        }
    }

    pub fn is_valid_rrid(&self, rrid: RuntimeResourceID) -> bool {
        Md5Engine::compute(&self.uri) == rrid.id
    }
}