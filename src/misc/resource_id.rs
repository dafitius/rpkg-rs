use regex::Regex;
use serde::Serialize;

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
        self.uri.find(']').map(|n| self.uri.chars().skip(open_count).take(n-1).collect())
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

    pub fn is_empty(&self) -> bool {
        self.uri.is_empty()
    }

    pub fn is_valid(&self) -> bool {
        let re = Regex::new(r"\[([A-z:/.0-9?]+)]([\([A-z,0-9]+\)]+)?.pc_[A-z]+").unwrap();
        re.is_match(&self.uri)
    }
}