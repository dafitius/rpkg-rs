use crate::utils;
use anyhow::{anyhow, Error};
use std::collections::HashMap;

#[derive(Default)]
pub struct ThumbsProperty {
    pub key: String,
    pub value: String,
    pub is_command: bool,
}

#[derive(Default)]
pub struct Thumbs {
    pub properties: HashMap<String, Vec<ThumbsProperty>>,
}

fn decipher(data: Box<[u8]>) -> Result<Vec<u8>, Error> {
    let header = Box::new(Vec::from([
        0x22, 0x3d, 0x6f, 0x9a, 0xb3, 0xf8, 0xfe, 0xb6, 0x61, 0xd9, 0xcc, 0x1c, 0x62, 0xde, 0x83,
        0x41,
    ]));
    let key = Box::new(Vec::from([0x30f95282, 0x1f48c419, 0x295f8548, 0x2a78366d]));
    let delta = 0x61c88647;
    let rounds = 32;

    let data_pointer = Box::into_raw(data);
    let unwrapped_data = unsafe { data_pointer.as_ref().expect("data is null") };
    if unwrapped_data.len() < 2 {
        panic!("data is < 2 for some reason");
    }

    let header_pointer = Box::into_raw(header);
    let unwrapped_header = unsafe { header_pointer.as_ref().expect("header is null") };

    let key_pointer = Box::into_raw(key);

    let unwrapped_key = unsafe { key_pointer.as_ref().expect("key is null") };
    if unwrapped_key.len() < 4 {
        panic!("key is < 2 for some reason");
    }

    let res = hitman_xtea::decipher_file(
        unwrapped_data,
        delta,
        unwrapped_header,
        rounds,
        unwrapped_key,
    );
    if res.is_err() {
        return Err(anyhow!("Couldn't decipher data"));
    }

    Ok(res.unwrap())
}

impl Thumbs {
    pub fn new() -> Self {
        Self {
            properties: HashMap::new(),
        }
    }

    pub fn parse_into(&mut self, path: String) -> Result<&Self, Error> {
        if let Ok(bytes) = utils::get_file_as_byte_vec(path.as_str()) {
            let data = bytes.into_boxed_slice();

            if let Ok(deciphered_data) = decipher(data) {
                //convert the byte array to a string
                let s = match std::str::from_utf8(deciphered_data.as_slice()) {
                    Ok(v) => v,
                    Err(e) => return Err(anyhow!("Unable to read deciphered data {}", e)),
                };

                let mut properties: HashMap<String, Vec<ThumbsProperty>> = HashMap::new();
                let mut active_tag: String = "None".to_string();

                for line in s.lines() {
                    match line {
                        _ if line.starts_with('#') => {}
                        _ if line.starts_with('[') => {
                            active_tag = line.replace(['[', ']'], "");
                            properties.insert(active_tag.clone(), vec![]);
                        }
                        _ if line.contains("ConsoleCmd") => {
                            let keyval = line.replace("ConsoleCmd ", "");
                            let (key, val) = keyval.split_once(' ').unwrap();
                            if let Some(props) = properties.get_mut(&active_tag){
                                props.push(ThumbsProperty{key: key.parse()?, value: val.parse()?, is_command: true });
                            }
                        }
                        _ if line.contains('=') => {
                            let (key, val) = line.split_once('=').unwrap();
                            if let Some(props) = properties.get_mut(&active_tag){
                                props.push(ThumbsProperty{key: key.parse()?, value: val.parse()?, is_command: false });
                            }
                        }
                        _ => {}
                    }
                }

                self.properties = properties;
                Ok(self)
            } else {
                Err(anyhow!("Failed to parse given thumbs file"))
            }
        } else {
            Err(anyhow!("Failed to read given file"))
        }
    }

    pub fn get_property(&self, key: &str) -> Option<&String> {
        for (_tag, properties) in self.properties.iter() {
            for property in properties{
                if property.key == key {
                    return Some(&property.value);
                }
            }
        }
        None
    }
}
