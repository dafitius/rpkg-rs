use crate::runtime::resource::package_defs::PartitionInfo;
use crate::runtime::resource::partition_manager::PartitionState;
use crate::runtime::resource::resource_info::ResourceInfo;
use crate::{utils, GlacierResource, GlacierResourceError, WoaVersion};
use regex::Regex;
use std::cmp::Ordering;
use std::fmt::Debug;
use std::path::PathBuf;
use std::{collections::HashMap, path::Path};
use std::{fmt, io};
use thiserror::Error;

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

    #[error("Interal resource error: {0}")]
    ResourceError(#[from] GlacierResourceError),
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PatchId {
    Base,
    Patch(usize),
}

impl Ord for PatchId {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (PatchId::Base, PatchId::Base) => Ordering::Equal,
            (PatchId::Base, PatchId::Patch(_)) => Ordering::Less,
            (PatchId::Patch(_), PatchId::Base) => Ordering::Greater,
            (PatchId::Patch(a), PatchId::Patch(b)) => a.cmp(b),
        }
    }
}

impl PartialOrd for PatchId {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub struct ResourcePartition {
    info: PartitionInfo,
    mount_location: Option<PathBuf>,

    packages: HashMap<PatchId, ResourcePackage>,
    resources: HashMap<RuntimeResourceID, PatchId>,
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

    /// search through the package_dir to figure out which patch indices are there.
    /// We have to use this inside of using the patchlevel inside the PartitionInfo.
    fn read_patch_indices(
        &self,
        package_dir: &Path,
    ) -> Result<Vec<PatchId>, ResourcePartitionError> {
        let mut patch_indices = vec![];

        let filename = self.info.filename(&PatchId::Base);
        if !package_dir.join(&filename).exists() {
            return Err(ResourcePartitionError::BasePackageNotFound(filename));
        }

        let regex_str = format!(r"^(?:{}patch([0-9]+).rpkg)$", self.info.id);
        let patch_package_re = Regex::new(regex_str.as_str()).unwrap();

        for file_name in utils::read_file_names(package_dir)
            .iter()
            .flat_map(|file_name| file_name.to_str())
        {
            if let Some(cap) = patch_package_re.captures(file_name) {
                let patch_level = cap[1].parse::<usize>()?;
                if patch_level <= self.info.patch_level {
                    patch_indices.push(PatchId::Patch(patch_level));
                }
            }
        }

        patch_indices.sort();
        Ok(patch_indices)
    }

    pub fn mount_resource_packages_in_partition(
        &mut self,
        runtime_path: &Path,
    ) -> Result<(), ResourcePartitionError> {
        self.mount_resource_packages_in_partition_with_hook(runtime_path, |_| {})
    }

    pub fn mount_resource_packages_in_partition_with_hook<F>(
        &mut self,
        runtime_path: &Path,
        mut progress_callback: F,
    ) -> Result<(), ResourcePartitionError>
    where
        F: FnMut(&PartitionState),
    {
        let mut state = PartitionState {
            installing: true,
            mounted: false,
            install_progress: 0.0,
        };

        //maybe don't error on a missing partition? the game doesn't...
        //let patch_indices = self.read_patch_indices(runtime_path)?;
        let patch_idx_result = self.read_patch_indices(runtime_path);
        if patch_idx_result.is_err() {
            state.installing = false;
            return Ok(());
        }

        let patch_indices = patch_idx_result.unwrap();

        let base_package_path = runtime_path.join(self.info.filename(&PatchId::Base));
        self.mount_package(base_package_path.as_path(), PatchId::Base)?;

        for (index, patch_id) in patch_indices.iter().enumerate() {
            let patch_package_path = runtime_path.join(self.info.filename(patch_id));
            self.mount_package(patch_package_path.as_path(), *patch_id)?;

            state.install_progress = index as f32 / patch_indices.len() as f32;
            progress_callback(&state);
        }
        state.install_progress = 1.0;
        state.installing = false;
        state.mounted = true;
        progress_callback(&state);

        self.mount_location = Some(runtime_path.to_path_buf());

        Ok(())
    }

    fn mount_package(
        &mut self,
        package_path: &Path,
        patch_index: PatchId,
    ) -> Result<(), ResourcePartitionError> {
        let rpkg = ResourcePackage::from_file(package_path).map_err(|e| {
            ResourcePartitionError::ReadResourcePackageError(
                e,
                package_path
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_string(),
            )
        })?;

        //remove the deletions if there are any
        for deletion in rpkg.unneeded_resource_ids().iter() {
            if self.resources.contains_key(deletion) {
                self.resources.remove_entry(deletion);
            }
        }

        for rrid in rpkg.resource_ids().keys() {
            self.resources.insert(*rrid, patch_index);
        }

        self.packages.insert(patch_index, rpkg);
        Ok(())
    }

    pub fn contains(&self, rrid: &RuntimeResourceID) -> bool {
        self.resources.contains_key(rrid)
    }

    #[allow(dead_code)]
    pub(crate) fn num_patches(&self) -> usize {
        self.packages.len().saturating_sub(1)
    }

    pub fn latest_resources(&self) -> Vec<(&ResourceInfo, &PatchId)> {
        self.resources
            .iter()
            .flat_map(|(rrid, idx)| {
                if let Ok(info) = self.resource_info_from(rrid, idx) {
                    Some((info, idx))
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn read_resource(
        &self,
        rrid: &RuntimeResourceID,
    ) -> Result<Vec<u8>, ResourcePartitionError> {
        let package_index = self
            .resources
            .get(rrid)
            .ok_or(ResourcePartitionError::ResourceNotAvailable)?;
        let rpkg = self
            .packages
            .get(package_index)
            .ok_or(ResourcePartitionError::NotMounted)?;
        let mut package_path = self
            .mount_location
            .clone()
            .ok_or(ResourcePartitionError::NotMounted)?;
        package_path = package_path.join(self.info.filename(package_index));
        rpkg.read_resource(package_path.as_path(), rrid)
            .map_err(|e| {
                ResourcePartitionError::ReadResourcePackageError(
                    e,
                    self.info.filename(package_index),
                )
            })
    }

    pub fn read_glacier_resource<T>(
        &self,
        woa_version: WoaVersion,
        rrid: &RuntimeResourceID,
    ) -> Result<T::Output, ResourcePartitionError>
    where
        T: GlacierResource,
    {
        let package_index = self
            .resources
            .get(rrid)
            .ok_or(ResourcePartitionError::ResourceNotAvailable)?;
        let rpkg = self
            .packages
            .get(package_index)
            .ok_or(ResourcePartitionError::NotMounted)?;
        let mut package_path = self
            .mount_location
            .clone()
            .ok_or(ResourcePartitionError::NotMounted)?;
        package_path = package_path.join(self.info.filename(package_index));
        let bytes = rpkg
            .read_resource(package_path.as_path(), rrid)
            .map_err(|e| {
                ResourcePartitionError::ReadResourcePackageError(
                    e,
                    self.info.filename(package_index),
                )
            })?;
        T::process_data(woa_version, bytes).map_err(ResourcePartitionError::ResourceError)
    }

    pub fn read_resource_from(
        &self,
        rrid: &RuntimeResourceID,
        patch_id: &PatchId,
    ) -> Result<Vec<u8>, ResourcePartitionError> {
        let rpkg = self
            .packages
            .get(patch_id)
            .ok_or(ResourcePartitionError::NotMounted)?;
        let mut package_path = self
            .mount_location
            .clone()
            .ok_or(ResourcePartitionError::NotMounted)?;
        package_path = package_path.join(self.info.filename(patch_id));
        rpkg.read_resource(package_path.as_path(), rrid)
            .map_err(|e| {
                ResourcePartitionError::ReadResourcePackageError(e, self.info.filename(patch_id))
            })
    }

    pub fn resource_info(
        &self,
        rrid: &RuntimeResourceID,
    ) -> Result<&ResourceInfo, ResourcePartitionError> {
        let package_index = self
            .resources
            .get(rrid)
            .ok_or(ResourcePartitionError::ResourceNotAvailable)?;
        let rpkg = self
            .packages
            .get(package_index)
            .ok_or(ResourcePartitionError::NotMounted)?;
        rpkg.resource_info(rrid)
            .ok_or(ResourcePartitionError::ResourceNotAvailable)
    }

    pub fn resource_info_from(
        &self,
        rrid: &RuntimeResourceID,
        patch_id: &PatchId,
    ) -> Result<&ResourceInfo, ResourcePartitionError> {
        let rpkg = self
            .packages
            .get(patch_id)
            .ok_or(ResourcePartitionError::NotMounted)?;
        rpkg.resource_info(rrid)
            .ok_or(ResourcePartitionError::ResourceNotAvailable)
    }

    pub fn partition_info(&self) -> &PartitionInfo {
        &self.info
    }

    pub fn resource_patch_indices(&self, rrid: &RuntimeResourceID) -> Vec<&PatchId> {
        self.packages
            .iter()
            .filter(|(_, package)| package.has_resource(rrid))
            .map(|(id, _)| id)
            .collect::<Vec<&PatchId>>()
    }

    pub fn resource_removal_indices(&self, rrid: &RuntimeResourceID) -> Vec<&PatchId> {
        self.packages
            .iter()
            .filter(|(_, package)| package.removes_resource(rrid))
            .map(|(id, _)| id)
            .collect::<Vec<&PatchId>>()
    }
}

impl Debug for ResourcePartition {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let total = self
            .packages
            .values()
            .map(|v| v.resource_ids().len())
            .sum::<usize>();
        write!(
            f,
            "{{index: {}, name: {}, edge_resources: {}, total_resources: {} }}",
            self.info.filename(&PatchId::Base),
            self.info.name.clone().unwrap_or_default(),
            self.resources.len(),
            total
        )?;

        Ok(())
    }
}
