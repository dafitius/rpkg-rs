use anyhow::{anyhow, Error};
use std::{collections::HashMap, path::Path};

use crate::{encryption::xtea::Xtea, utils};

#[derive(Default, Debug)]
pub struct IniFileSection {
    name: String,
    options: HashMap<String, String>,
}

#[derive(Default)]
pub struct IniFile {
    sections: HashMap<String, IniFileSection>,
    console_cmds: Vec<String>,
    ini_files_loaded: Vec<String>,
}

impl IniFileSection {
    fn new(name: &String) -> Self {
        Self {
            name: name.to_owned(),
            options: HashMap::new(),
        }
    }

    pub fn get_name(&self) -> String {
        self.name.to_owned()
    }

    pub fn get_value(&self, option_name: &str) -> Option<&String> {
        self.options.get(option_name)
    }

    pub fn get_options(&self) -> Vec<&String> {
        self.options.keys().collect()
    }

    fn set_option(&mut self, option_name: &str, value: &str) {
        if let Some(key) = self.options.get_mut(option_name) {
            *key = value.to_string();
        } else {
            self.options
                .insert(option_name.to_string(), value.to_string());
        }
    }
}

impl IniFile {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load(&mut self, path: &str) -> Result<(), anyhow::Error> {
        self.load_internal(path)?;
        self.ini_files_loaded.push(path.to_string());
        Ok(())
    }

    pub fn load_from_string(&mut self, settings: &str) -> Result<(), anyhow::Error> {
        self.load_from_string_internal(settings, "")?;
        Ok(())
    }

    pub fn get_sections(&self) -> Vec<&IniFileSection> {
        self.sections.values().collect()
    }

    pub fn get_section(&self, name: &str) -> Option<&IniFileSection> {
        self.sections.get(name)
    }

    pub fn get_section_mut(&mut self, name: &str) -> Option<&mut IniFileSection> {
        self.sections.get_mut(name)
    }

    pub fn get_value(
        &self,
        section_name: &str,
        option_name: &str,
    ) -> Result<&String, anyhow::Error> {
        match self.sections.get(section_name) {
            Some(v) => match v.options.get(option_name) {
                Some(o) => Ok(o),
                None => Err(anyhow!("Can't find value inside the section")),
            },
            None => Err(anyhow!("Can't find section")),
        }
    }

    pub fn set_value(
        &mut self,
        section_name: &str,
        option_name: &str,
        value: &str,
    ) -> Result<(), anyhow::Error> {
        match self.sections.get_mut(section_name) {
            Some(v) => match v.options.get_mut(option_name) {
                Some(o) => {
                    *o = value.to_string();
                    Ok(())
                }
                None => Err(anyhow!("Can't find value inside the section")),
            },
            None => Err(anyhow!("Can't find section")),
        }
    }

    pub fn push_console_command(&mut self, command: String) {
        self.console_cmds.push(command);
    }

    pub fn get_console_cmds(&self) -> &Vec<String> {
        &self.console_cmds
    }

    pub fn get_ini_files_loaded(&self) -> &Vec<String> {
        &self.ini_files_loaded
    }

    fn load_internal(&mut self, path: &str) -> Result<(), Error> {
        let content = self.load_ini_file_content(path)?;
        if Xtea::is_encrypted_text_file(&content) {
            let content_decrypted = Xtea::decrypt_text_file(&content, &Xtea::DEFAULT_KEY)?;
            self.load_from_string_internal(content_decrypted.as_str(), path)?;
        }
        Ok(())
    }

    pub fn load_ini_file_content(&self, path: &str) -> Result<Vec<u8>, Error> {
        utils::get_file_as_byte_vec(path)
    }

    pub fn generate_hash(&self, s: &str) -> u32 {
        let mut hash_value: u32 = 0xe10f732f; // Initialize the hash value with a constant.
        let length: usize = s.len() & 0x3fffffff; // Get the least significant 30 bits of the string length.

        // Check if the string is not empty.
        if length != 0 {
            for char in s.as_bytes() {
                let curr_char = *char;
                hash_value = (hash_value << 13) | (hash_value >> 19); // Rotate left by 13 bits.
                hash_value ^= u32::from(curr_char); // XOR with the ASCII value of the current character.
            }
        }
        hash_value // Return the final hash value.
    }

    fn load_from_string_internal(
        &mut self,
        ini_file_content: &str,
        path: &str,
    ) -> Result<(), anyhow::Error> {
        let mut active_section: String = "None".to_string();

        for line in ini_file_content.lines() {
            match line {
                _ if line.starts_with('!') => {
                    let keyval = line.split(' ').collect::<Vec<&str>>();
                    let (command, value) = (*keyval.first().unwrap(), *keyval.get(1).unwrap());
                    if command == "!include" {
                        if let Some(new_path) = Self::parse_file_name(path, value) {
                            self.load(new_path.as_str())?;
                        }
                    }
                }
                _ if line.starts_with('#') => {}
                _ if line.starts_with('[') => {
                    active_section = line.replace(['[', ']'], "");
                    if !self.sections.contains_key(&active_section) {
                        self.sections
                            .insert(active_section.clone(), IniFileSection::new(&active_section));
                    }
                }
                _ if line.contains("ConsoleCmd") => {
                    let keyval = line.replace("ConsoleCmd ", "");
                    self.console_cmds.push(keyval)
                }
                _ if line.contains('=') => {
                    let (key, val) = line.split_once('=').unwrap();
                    if let Some(section) = self.sections.get_mut(&active_section) {
                        section.set_option(key, val);
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn parse_file_name(current_file: &str, psz_raw: &str) -> Option<String> {
        let parent_path = Path::new(current_file).parent()?;
        Some(parent_path.join(Path::new(psz_raw)).to_str()?.to_string())
    }
}
