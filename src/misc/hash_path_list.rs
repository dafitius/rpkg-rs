use crate::misc::resource_id::ResourceID;
use crate::runtime::resource::runtime_resource_id::RuntimeResourceID;
use rayon::iter::ParallelIterator;
use rayon::prelude::IntoParallelIterator;
use std::collections::HashMap;
use std::fs::read_to_string;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PathListError {
    #[error("{0}")]
    IoError(#[from] std::io::Error),

    #[error("Invalid RuntimeResourceID entry")]
    InvalidRuntimeResourceID,
}

/// A rainbow table of hashed paths with associated paths.
#[derive(Default)]
pub struct PathList {
    entries: HashMap<RuntimeResourceID, Option<ResourceID>>,
}

impl PathList {
    /// Creates a new empty PathList.
    pub fn new() -> Self {
        Self::default()
    }

    /// Parses a file into the PathList.
    ///
    /// Example of an input file:
    /// ```txt
    /// #comments will be ingored!
    /// 00546F0BD4E80484.GFXI,[assembly:/any/path/here/file.jpg].pc_gfx
    /// 0023456789ABCDEF.WWEM, this_will_fail_the_md5_validation
    /// 003456789ABCDEF0.FXAS,[assembly:/lorem/ipsum/dolor_sit/amet/ai/consectetur/adipisicing.animset].pc_animset
    /// ....
    /// ```
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the file to parse.
    pub fn parse_into(&mut self, path: &Path) -> Result<&Self, PathListError> {
        let file_as_string = read_to_string(path).map_err(PathListError::IoError)?;
        let lines: Vec<_> = file_as_string.lines().map(String::from).collect();

        let lines_par = lines.into_par_iter();

        self.entries = lines_par
            .filter_map(|line_res| {
                if line_res.starts_with('#') {
                    return None;
                };

                let (hash, path) = match line_res.split_once(',') {
                    Some((h, p)) => (h.split_once('.').unwrap().0, Some(p)),
                    None => (line_res.as_str(), None),
                };

                if let Ok(id) = u64::from_str_radix(hash, 16) {
                    if let Some(path) = path {
                        if let Ok(rid) = ResourceID::from_string(path) {
                            if rid.is_valid() {
                                return Some((RuntimeResourceID::from(id), Some(rid)));
                            }
                        }
                    }
                    Some((RuntimeResourceID::from(id), None))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .into_iter()
            .collect();

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
}
