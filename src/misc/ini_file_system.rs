use crate::encryption::xtea::Xtea;
use crate::encryption::xtea::XteaError;
use crate::utils::normalize_path;
use itertools::Itertools;
use pathdiff::diff_paths;
use std::collections::VecDeque;
use std::io::Write;
use std::ops::{Index, IndexMut};
use std::path::PathBuf;
use std::str::from_utf8;
use std::{collections::HashMap, fs, path::Path};
use thiserror::Error;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Error, Debug)]
pub enum IniFileError {
    #[error("Option ({}) not found", _0)]
    OptionNotFound(String),

    #[error("Can't find section ({})", _0)]
    SectionNotFound(String),

    #[error("An error occurred when parsing: {}", _0)]
    ParsingError(String),

    #[error("An io error occurred: {}", _0)]
    IoError(#[from] std::io::Error),

    #[error("An io error occurred: {}", _0)]
    DecryptionError(#[from] XteaError),

    #[error("The given input was incorrect: {}", _0)]
    InvalidInput(String),
}

#[derive(Default, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct IniFileSection {
    name: String,
    options: HashMap<String, String>,
}

/// Represents a system config file for the Glacier engine
/// ## Example contents
///
/// ```txt
/// [application]
/// ForceVSync=0
/// CapWorkerThreads=1
/// SCENE_FILE=assembly:/path/to/scene.entity
/// ....
///
/// [Hitman5]
/// usegamecontroller=1
/// ConsoleCmd UI_EnableMouseEvents 0
/// ....
/// ```
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct IniFile {
    name: String,
    description: Option<String>,
    includes: Vec<IniFile>,
    sections: HashMap<String, IniFileSection>,
    console_cmds: Vec<String>,
}

/// A hierarchical file system of [IniFile].
///
/// example usage:
/// ```ignore
///  use std::path::PathBuf;
///  use rpkg_rs::misc::ini_file_system::IniFileSystem;
///
///  let retail_path = PathBuf::from("Path to retail folder");
///  let thumbs_path = retail_path.join("thumbs.dat");
///
///  let thumbs = IniFileSystem::from(&thumbs_path.as_path())?;
///
///  let app_options = &thumbs.root()?;
///
///  if let (Some(proj_path), Some(runtime_path)) = (app_options.get("PROJECT_PATH"), app_options.get("RUNTIME_PATH")) {
///     println!("Project path: {}", proj_path);
///     println!("Runtime path: {}", runtime_path);
///  }
/// ```
#[derive(Default, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct IniFileSystem {
    root: IniFile,
}

impl IniFileSection {
    fn new(name: String) -> Self {
        Self {
            name,
            options: HashMap::new(),
        }
    }

    pub fn name(&self) -> String {
        self.name.to_owned()
    }

    pub fn options(&self) -> &HashMap<String, String> {
        &self.options
    }

    pub fn has_option(&self, option_name: &str) -> bool {
        self.options.contains_key(option_name)
    }

    fn set_option(&mut self, option_name: &str, value: &str) {
        if let Some(key) = self.options.get_mut(option_name) {
            *key = value.to_string();
        } else {
            self.options
                .insert(option_name.to_string(), value.to_string());
        }
    }

    pub fn write_section<W: std::fmt::Write>(&self, writer: &mut W) {
        writeln!(writer, "[{}]", self.name).unwrap();
        for (key, value) in &self.options {
            writeln!(writer, "{}={}", key, value).unwrap();
        }
        writeln!(writer).unwrap();
    }
}

impl Index<&str> for IniFileSection {
    type Output = str;

    fn index(&self, option_name: &str) -> &str {
        self.options.get(option_name).expect("Option not found")
    }
}

impl IndexMut<&str> for IniFileSection {
    fn index_mut(&mut self, option_name: &str) -> &mut str {
        self.options.entry(option_name.to_string()).or_default()
    }
}

impl Default for IniFile {
    fn default() -> Self {
        Self {
            name: "thumbs.dat".to_string(),
            description: Some(String::from("System config file for the engine")),
            includes: vec![],
            sections: Default::default(),
            console_cmds: vec![],
        }
    }
}

impl IniFile {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            description: None,
            includes: vec![],
            sections: Default::default(),
            console_cmds: vec![],
        }
    }
    pub fn name(&self) -> String {
        self.name.to_string()
    }
    pub fn sections(&self) -> &HashMap<String, IniFileSection> {
        &self.sections
    }

    pub fn includes(&self) -> &Vec<IniFile> {
        &self.includes
    }

    pub fn find_include(&self, include_name: &str) -> Option<&IniFile> {
        self.includes.iter().find(|incl| incl.name == include_name)
    }

    pub fn get_option(
        &self,
        section_name: &str,
        option_name: &str,
    ) -> Result<String, IniFileError> {
        match self.sections.get(section_name) {
            Some(v) => match v.options.get(option_name.to_uppercase().as_str()) {
                Some(o) => Ok(o.clone()),
                None => Err(IniFileError::OptionNotFound(option_name.to_string())),
            },
            None => Err(IniFileError::SectionNotFound(section_name.to_string())),
        }
    }

    pub fn set_value(
        &mut self,
        section_name: &str,
        option_name: &str,
        value: &str,
    ) -> Result<(), IniFileError> {
        match self.sections.get_mut(section_name) {
            Some(v) => match v.options.get_mut(option_name) {
                Some(o) => {
                    *o = value.to_string();
                    Ok(())
                }
                None => Err(IniFileError::OptionNotFound(option_name.to_string())),
            },
            None => Err(IniFileError::SectionNotFound(section_name.to_string())),
        }
    }

    pub fn push_console_command(&mut self, command: String) {
        self.console_cmds.push(command);
    }

    pub fn console_cmds(&self) -> &Vec<String> {
        &self.console_cmds
    }

    pub fn write_ini_file<W: std::fmt::Write>(&self, writer: &mut W) {
        if let Some(description) = &self.description {
            writeln!(writer, "# {}", description).unwrap();
            writeln!(writer, "\n# -----------------------------------------------------------------------------\n", ).unwrap();
        }
        for include in &self.includes {
            writeln!(writer, "!include {}", include.name).unwrap();
        }
        for section_name in self
            .sections
            .keys()
            .sorted_by(|a, b| Ord::cmp(&a.to_lowercase(), &b.to_lowercase()))
        {
            if let Some(section) = self.sections().get(section_name) {
                section.write_section(writer);
            }
        }
        for console_cmd in &self.console_cmds {
            writeln!(writer, "ConsoleCmd {}", console_cmd).unwrap();
        }
    }
}

impl IniFileSystem {
    pub fn new() -> Self {
        Self {
            root: IniFile::new("thumbs.dat"),
        }
    }

    /// Loads an IniFileSystem from the given root file.
    pub fn load(&mut self, root_file: impl AsRef<Path>) -> Result<(), IniFileError> {
        let ini_file = Self::load_from_path(
            root_file.as_ref(),
            PathBuf::from(root_file.as_ref()).parent().unwrap(),
        )?;
        self.root = ini_file;
        Ok(())
    }

    pub fn from(root_file: impl AsRef<Path>) -> Result<Self, IniFileError> {
        let mut ret = Self::new();
        match ret.load(root_file) {
            Ok(_) => Ok(ret),
            Err(e) => Err(e),
        }
    }

    fn load_from_path(path: &Path, working_directory: &Path) -> Result<IniFile, IniFileError> {
        let content = fs::read(path).map_err(IniFileError::IoError)?;
        let mut content_decrypted = from_utf8(content.as_ref()).unwrap_or("").to_string();
        if Xtea::is_encrypted_text_file(&content) {
            content_decrypted =
                Xtea::decrypt_text_file(&content).map_err(IniFileError::DecryptionError)?;
        }

        let ini_file_name = match diff_paths(path, working_directory) {
            Some(relative_path) => relative_path.to_str().unwrap().to_string(),
            None => path.to_str().unwrap().to_string(),
        };
        Self::load_from_string(
            ini_file_name.as_str(),
            content_decrypted.as_str(),
            working_directory,
        )
    }

    fn load_from_string(
        name: &str,
        ini_file_content: &str,
        working_directory: &Path,
    ) -> Result<IniFile, IniFileError> {
        let mut active_section: String = "None".to_string();
        let mut ini_file = IniFile::new(name);

        for line in ini_file_content.lines() {
            if let Some(description) = line.strip_prefix('#') {
                if ini_file_content.starts_with(line) {
                    //I don't really like this, but IOI seems to consistently use the first comment as a description.
                    ini_file.description = Some(description.trim_start().to_string());
                }
            } else if let Some(line) = line.strip_prefix('!') {
                if let Some((command, value)) = line.split_once(' ') {
                    if command == "include" {
                        let include = Self::load_from_path(
                            working_directory.join(value).as_path(),
                            working_directory,
                        )?;
                        ini_file.includes.push(include);
                    }
                }
            } else if let Some(mut section_name) = line.strip_prefix('[') {
                section_name = section_name
                    .strip_suffix(']')
                    .ok_or(IniFileError::ParsingError(
                        "a section should always have a closing ] bracket".to_string(),
                    ))?;
                active_section = section_name.to_string();
                if !ini_file.sections.contains_key(&active_section) {
                    ini_file.sections.insert(
                        active_section.clone(),
                        IniFileSection::new(active_section.clone()),
                    );
                }
            } else if let Some(keyval) = line.strip_prefix("ConsoleCmd ") {
                ini_file.console_cmds.push(keyval.to_string());
            } else if let Some((key, val)) = line.split_once('=') {
                if let Some(section) = ini_file.sections.get_mut(&active_section) {
                    section.set_option(key.to_uppercase().as_str(), val);
                }
            }
        }
        Ok(ini_file)
    }

    pub fn write_to_folder(&self, path: &Path) -> Result<(), IniFileError> {
        let mut folder = path;
        if folder.is_file() {
            folder = path.parent().ok_or(IniFileError::InvalidInput(
                "The export path cannot be empty".to_string(),
            ))?;
        }
        fn write_children_to_folder(path: &Path, ini_file: &IniFile) -> Result<(), IniFileError> {
            let mut file_path = path.join(&ini_file.name);
            file_path = normalize_path(&file_path);

            let parent_dir = file_path.parent().ok_or(IniFileError::InvalidInput(
                "Invalid export path given".to_string(),
            ))?;
            fs::create_dir_all(parent_dir)?;

            let mut writer = fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&file_path)?;
            let mut contents = String::new();
            ini_file.write_ini_file(&mut contents);
            let _ = writer.write_all(contents.as_bytes());

            for include in ini_file.includes.iter() {
                match write_children_to_folder(parent_dir, include) {
                    Ok(_) => {}
                    Err(e) => return Err(e),
                };
            }
            Ok(())
        }

        write_children_to_folder(folder, &self.root)
    }

    /// Normalizes the IniFileSystem by merging sections and console commands from included files into the root file.
    pub fn normalize(&mut self) {
        let mut queue: VecDeque<IniFile> = VecDeque::new();
        for include in self.root.includes.drain(0..) {
            queue.push_back(include);
        }

        while let Some(mut current_file) = queue.pop_front() {
            let root_sections = &mut self.root.sections;

            for (section_key, section) in current_file.sections.drain() {
                if !root_sections.contains_key(&section_key) {
                    root_sections.insert(section_key.clone(), section);
                } else {
                    let root_section = root_sections.get_mut(&section_key).unwrap();
                    for (key, value) in section.options {
                        if !root_section.has_option(&key) {
                            root_section.set_option(&key, &value);
                        } else {
                            root_section.set_option(&key, value.as_str());
                        }
                    }
                }
            }

            for console_cmd in current_file.console_cmds.drain(..) {
                if !self.root.console_cmds.contains(&console_cmd) {
                    self.root.console_cmds.push(console_cmd);
                }
            }
            for include in current_file.includes.drain(0..) {
                queue.push_back(include);
            }
        }
    }

    /// Retrieves all console commands from the IniFileSystem, including those from included files.
    pub fn console_cmds(&self) -> Vec<String> {
        let mut cmds: Vec<String> = vec![];

        // Helper function to traverse the includes recursively
        fn traverse_includes(ini_file: &IniFile, cmds: &mut Vec<String>) {
            for include in &ini_file.includes {
                cmds.extend_from_slice(&include.console_cmds);
                traverse_includes(include, cmds);
            }
        }

        cmds.extend_from_slice(&self.root.console_cmds);
        traverse_includes(&self.root, &mut cmds);

        cmds
    }

    /// Retrieves the value of an option in a section from the IniFileSystem, including values from included files.
    pub fn option(&self, section_name: &str, option_name: &str) -> Result<String, IniFileError> {
        let mut queue: VecDeque<&IniFile> = VecDeque::new();
        queue.push_back(&self.root);
        let mut latest_value: Option<String> = None;

        while let Some(current_file) = queue.pop_front() {
            if let Ok(value) = current_file.get_option(section_name, option_name) {
                // Update the latest value found
                latest_value = Some(value.clone());
            }
            for include in &current_file.includes {
                queue.push_back(include);
            }
        }

        // Return the latest value found or an error if none
        latest_value.ok_or_else(|| IniFileError::OptionNotFound(option_name.to_string()))
    }

    /// Retrieves a reference to the root IniFile of the IniFileSystem.
    pub fn root(&self) -> &IniFile {
        &self.root
    }
}

impl Index<&str> for IniFile {
    type Output = IniFileSection;

    fn index(&self, section_name: &str) -> &IniFileSection {
        self.sections.get(section_name).expect("Section not found")
    }
}

impl IndexMut<&str> for IniFile {
    fn index_mut(&mut self, section_name: &str) -> &mut IniFileSection {
        self.sections
            .entry(section_name.to_string())
            .or_insert(IniFileSection::new(section_name.to_string()))
    }
}
