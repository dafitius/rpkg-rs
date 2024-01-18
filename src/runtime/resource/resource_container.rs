use regex::Regex;
use std::fmt;
use std::iter::zip;
use std::{collections::HashMap, fs, path::Path};
use std::path::PathBuf;
use itertools::Itertools;
use thiserror::Error;

use crate::runtime::resource::resource_package::{ResourcePackage, ResourcePackageError};

use super::package_manager::PartitionInfo;
use super::resource_info::ResourceInfo;
use super::resource_index::ResourceIndex;
use super::runtime_resource_id::RuntimeResourceID;


#[derive(Debug, Error)]
pub enum ResourceContainerError {
    #[error("Failed to open file: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Error while reading ResourcePackage({1}): {0}")]
    ReadResourcePackageError(ResourcePackageError, String),

    #[error("Failed to parse patch index as u16: {0}")]
    ParsePatchIndexError(#[from] std::num::ParseIntError),

    #[error("Base package not found: {0}")]
    BasePackageNotFound(String),

    #[error("Failed to read package: {0}")]
    ReadPackageError(String),

    // Add more error variants as needed
}

#[derive(Default)]
pub struct ResourceContainer {
    pub resources: Vec<ResourceInfo>,
    pub old_versions: Vec<ResourceIndex>,
    pub indices: HashMap<RuntimeResourceID, ResourceIndex>,
    //dynamic_resources: Vec<RuntimeResourceID>,
}

impl ResourceContainer {

    pub fn get_patch_indices(package_dir: &PathBuf, index: usize) -> Result<Vec<u16>, ResourceContainerError> {
        let mut patch_indices = vec![];

        if !package_dir.join(format!("chunk{}.rpkg", index)).exists() {
            return Err(ResourceContainerError::BasePackageNotFound(format!("chunk{}.rpkg", index)));
        }

        let regex_str = package_dir.join(format!("chunk{}patch([0-9]+).rpkg", index))
            .as_os_str().to_str().unwrap_or("")
            .replace('\\', "\\\\");
        let patch_package_re = Regex::new(regex_str.as_str()).unwrap();
        for path_buf in fs::read_dir(package_dir)?
            .filter(|r| r.is_ok())
            .map(|r| r.unwrap().path())
            .filter(|r| r.is_file())
        {
            let path = path_buf.as_path().to_str().unwrap();
            if patch_package_re.is_match(path) {
                let cap = patch_package_re.captures(path).unwrap();
                patch_indices.push(cap[1].parse::<u16>()?);
            }
        }
        patch_indices.sort();
        Ok(patch_indices)
    }

    pub fn mount_partition(
        &mut self,
        partition_info: &PartitionInfo,
        runtime_path: &PathBuf,
    ) -> Result<(), ResourceContainerError> {
        let patch_indices = Self::get_patch_indices(runtime_path, partition_info.index)?;

        let base_package_path = runtime_path.join(format!("chunk{}.rpkg", partition_info.index));
        self.mount_package(base_package_path.as_path(), false)?;

        for patch_index in patch_indices.iter() {
            let patch_package_path = runtime_path.join(format!(
                "chunk{}patch{}.rpkg", partition_info.index, patch_index)
            );
            self.mount_package(patch_package_path.as_path(), true)?;
        }

        println!(
            "chunk{} has patch levels: {:?}",
            partition_info.index, patch_indices
        );
        println!("rpkg file contains {} Resources", self.indices.len());
        Ok(())
    }

    fn mount_package(&mut self, package_path: &Path, is_patch: bool) -> Result<(), ResourceContainerError> {

        let rpkg = ResourcePackage::from_file(package_path).map_err(|e| ResourceContainerError::ReadResourcePackageError(e, package_path.file_name().unwrap().to_str().unwrap().to_string()))?;

        //remove the deletions if there are any
        if let Some(deletions) = rpkg.unneeded_resources {
            for deletion in deletions.iter() {
                //This doesn't fix the next_newest_index breaking yet.
                if let Some(idx) = self.indices.get(deletion){
                    self.old_versions.push(idx.clone());
                }
                self.indices.remove(deletion);
            }
        }

        for (entry, header) in zip(rpkg.resource_entries, rpkg.resource_metadata) {
            // Determine hash's size and if it is LZ4ed and/or XORed
            let mut xored = false;
            let mut lz4ed = false;
            let mut file_size;
            if header.data_size & 0x3FFFFFFF != 0 {
                lz4ed = true;
                file_size = header.data_size;

                if header.data_size & 0x80000000 == 0x80000000 {
                    file_size &= 0x3FFFFFFF;
                    xored = true;
                }
            } else {
                file_size = entry.compressed_size_and_is_scrambled_flag;

                if header.data_size & 0x80000000 == 0x80000000 {
                    xored = true;
                }
            }

            self.resources.push(ResourceInfo {
                entry,
                header,
                size: file_size,
                is_lz4ed: lz4ed,
                is_scrambled: xored,
                next_newest_index: None,
            });
            let old_val = self.indices.insert(
                self.resources.last().unwrap().entry.runtime_resource_id,
                ResourceIndex::from(self.resources.len()),
            );

            if let Some(old_index) = old_val {
                self.old_versions.push(old_index.clone());
                if is_patch {
                    let resource_count = self.resources.len();
                    if let Some(resource) = self.resources.get_mut(old_index.val as usize) {
                        resource.next_newest_index =
                            Some(ResourceIndex::from(resource_count));
                    }
                }
            }
        }
        Ok(())
    }

    #[allow(dead_code)]
    fn is_resource_mounted(&self, r_index: ResourceIndex) -> bool {
        self.resources.get(r_index.val as usize).is_some()
    }
}

impl fmt::Display for ResourceContainer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "There are {} resources", self.resources.len())?;
        writeln!(f, "There are {} old resources", self.old_versions.len())?;
        writeln!(f, "There are {} indices", self.indices.len())?;
        Ok(())
    }
}
