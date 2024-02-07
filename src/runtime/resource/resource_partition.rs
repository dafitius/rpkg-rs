use regex::Regex;
use std::{fmt, io};
use std::{collections::HashMap, fs, path::Path};
use std::fmt::Debug;
use std::path::PathBuf;
use thiserror::Error;
use crate::runtime::resource::package_defs::PartitionInfo;
use crate::runtime::resource::partition_manager::PartitionState;
use crate::runtime::resource::resource_info::ResourceInfo;

use crate::runtime::resource::resource_package::{ResourcePackage, ResourcePackageError};

use super::runtime_resource_id::RuntimeResourceID;


#[derive(Debug, Error)]
pub enum ResourcePartitionError {
    #[error("Failed to open file: {0}")]
    IoError(#[from] io::Error),

    #[error("Error while reading ResourcePackage({1}): {0}")]
    ReadResourcePackageError(ResourcePackageError, String),

    #[error("Failed to parse patch index as u16: {0}")]
    ParsePatchIndexError(#[from] std::num::ParseIntError),

    #[error("Base package not found: {0}")]
    BasePackageNotFound(String),

    #[error("Failed to read package: {0}")]
    ReadPackageError(String),

    #[error("No partition mounted")]
    NotMounted,

    #[error("Resource not available")]
    ResourceNotAvailable,
}

pub struct ResourcePartition {
    pub(crate) info: PartitionInfo,
    mount_location: Option<PathBuf>,

    packages: HashMap<Option<usize>, ResourcePackage>,
    resources: HashMap<RuntimeResourceID, Option<usize>>,
}

impl ResourcePartition {
    pub fn new(info: PartitionInfo) -> Self {
        Self {
            info,
            mount_location: None,
            packages: Default::default(),
            resources: Default::default(),
        }
    }

    fn get_patch_indices(&self, package_dir: &PathBuf) -> Result<Vec<usize>, ResourcePartitionError> {
        let mut patch_indices = vec![];

        let filename = self.info.get_filename(None);
        if !package_dir.join(&filename).exists() {
            return Err(ResourcePartitionError::BasePackageNotFound(filename));
        }

        let regex_str = package_dir.join(format!("{}patch([0-9]+).rpkg", self.info.id))
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
                patch_indices.push(cap[1].parse::<usize>()?);
            }
        }
        patch_indices.sort();
        Ok(patch_indices)
    }

    pub fn mount_resource_packages_in_partition<F>(&mut self, runtime_path: &PathBuf, mut progress_callback: F,
    ) -> Result<(), ResourcePartitionError>
        where
            F: FnMut(&PartitionState), {
        let mut state = PartitionState {
            installing: true,
            mounted: false,
            install_progress: 0.0,
        };

        //maybe don't error on a missing partition? the game doesn't...
        //let patch_indices = self.get_patch_indices(runtime_path)?;

        let patch_idx_result = self.get_patch_indices(runtime_path);
        if patch_idx_result.is_err() {
            state.installing = false;
            return Ok(());
        }

        let patch_indices = patch_idx_result.unwrap();

        let base_package_path = runtime_path.join(self.info.get_filename(None));
        self.mount_package(base_package_path.as_path(), None)?;

        for (index, patch_index) in patch_indices.iter().enumerate() {
            let patch_package_path = runtime_path.join(
                self.info.get_filename(Some(*patch_index))
            );
            self.mount_package(patch_package_path.as_path(), Some(*patch_index))?;

            state.install_progress = index as f32 / patch_indices.len() as f32;
            progress_callback(&state);
        }
        state.install_progress = 1.0;
        state.installing = false;
        state.mounted = true;
        progress_callback(&state);

        self.mount_location = Some(runtime_path.clone());

        Ok(())
    }

    fn mount_package(&mut self, package_path: &Path, patch_index: Option<usize>) -> Result<(), ResourcePartitionError> {
        let rpkg = ResourcePackage::from_file(package_path).map_err(|e| ResourcePartitionError::ReadResourcePackageError(e, package_path.file_name().unwrap().to_str().unwrap().to_string()))?;

        //remove the deletions if there are any
        for deletion in rpkg.get_unneeded_resource_ids().iter() {
            if self.resources.contains_key(deletion) {
                self.resources.remove_entry(deletion);
            }
        }

        for rrid in rpkg.get_resource_ids().keys() {
            self.resources.insert(*rrid, patch_index);
        }

        self.packages.insert(patch_index, rpkg);
        Ok(())
    }

    #[allow(dead_code)]
    fn is_resource_mounted(&self, rrid: &RuntimeResourceID) -> bool {
        self.resources.contains_key(rrid)
    }

    pub fn get_resource(&self, rrid: &RuntimeResourceID) -> Result<Vec<u8>, ResourcePartitionError> {
        let package_index = self.resources.get(rrid).ok_or(ResourcePartitionError::ResourceNotAvailable)?;
        let rpkg = self.packages.get(package_index).ok_or(ResourcePartitionError::NotMounted)?;
        let mut package_path = self.mount_location.clone().ok_or(ResourcePartitionError::NotMounted)?;
        package_path = package_path.join(self.info.get_filename(*package_index));
        rpkg.get_resource(package_path.as_path(), rrid).map_err(|e| ResourcePartitionError::ReadResourcePackageError(e, self.info.get_filename(*package_index)))
    }

    pub fn get_resource_from(&self, rrid: &RuntimeResourceID, patch_id: Option<usize>) -> Result<Vec<u8>, ResourcePartitionError> {
        let rpkg = self.packages.get(&patch_id).ok_or(ResourcePartitionError::NotMounted)?;
        let mut package_path = self.mount_location.clone().ok_or(ResourcePartitionError::NotMounted)?;
        package_path = package_path.join(self.info.get_filename(patch_id));
        rpkg.get_resource(package_path.as_path(), rrid).map_err(|e| ResourcePartitionError::ReadResourcePackageError(e, self.info.get_filename(patch_id)))
    }

    pub fn get_resource_info(&self, rrid: &RuntimeResourceID) -> Result<&ResourceInfo, ResourcePartitionError> {
        let package_index = self.resources.get(rrid).ok_or(ResourcePartitionError::ResourceNotAvailable)?;
        let rpkg = self.packages.get(package_index).ok_or(ResourcePartitionError::NotMounted)?;
        rpkg.get_resource_info(rrid).ok_or(ResourcePartitionError::ResourceNotAvailable)
    }

    pub fn get_partition_info(&self) -> &PartitionInfo {
        &self.info
    }

    pub fn print_resource_changelog(&self, rrid: &RuntimeResourceID) -> Result<Vec<u8>, ResourcePartitionError> {
        let mut versions = vec![];


        if let Some(resource_info) = self.resources.get(rrid) {
            versions.push(resource_info);
        };

        Err(ResourcePartitionError::ResourceNotAvailable)
    }
}

impl Debug for ResourcePartition {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let total = self.packages.values().map(|v| v.get_resource_ids().len()).sum::<usize>();
        write!(f, "{{index: {}, name: {}, edge_resources: {}, total_resources: {} }}", self.info.get_filename(None), self.info.name.clone().unwrap_or(String::new()), self.resources.len(), total)?;

        Ok(())
    }
}
