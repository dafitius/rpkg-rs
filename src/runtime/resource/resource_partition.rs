use regex::Regex;
use std::{fmt, io};
use std::{collections::HashMap, fs, path::Path};
use std::cmp::Ordering;
use std::fmt::Debug;
use std::path::PathBuf;
use thiserror::Error;
use crate::{GlacierResource, GlacierResourceError, WoaVersion};
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

    #[error("Interal resource error: {0}")]
    ResourceError(#[from] GlacierResourceError)
}

#[derive(Clone, Copy, Debug)]
#[derive(Eq, Hash, PartialEq)]
pub enum PatchId{
    Base,
    Patch(usize)
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
    fn get_patch_indices(&self, package_dir: &PathBuf) -> Result<Vec<PatchId>, ResourcePartitionError> {
        let mut patch_indices = vec![];

        let filename = self.info.get_filename(&PatchId::Base);
        if !package_dir.join(&filename).exists() {
            return Err(ResourcePartitionError::BasePackageNotFound(filename));
        }

        let regex_str = package_dir.join(format!("{}patch([0-9]+).rpkg", self.info.id))
            .as_os_str().to_str().unwrap_or("")
            .replace('\\', "/");
        let patch_package_re = Regex::new(regex_str.as_str()).unwrap();
        for path_buf in fs::read_dir(package_dir)?
            .filter(|r| r.is_ok())
            .map(|r| r.unwrap().path())
            .filter(|r| r.is_file())
        {
            let path = path_buf.as_path().to_str().unwrap_or("").replace('\\', "/");
            if patch_package_re.is_match(path.as_str()) {
                let cap = patch_package_re.captures(path.as_str()).unwrap();
                let patch_level = cap[1].parse::<usize>()?;
                if patch_level <= self.info.patch_level {
                    patch_indices.push(PatchId::Patch(patch_level));
                }
            }
        }
        patch_indices.sort();
        Ok(patch_indices)
    }

    pub fn mount_resource_packages_in_partition(&mut self, runtime_path: &PathBuf,
    ) -> Result<(), ResourcePartitionError>{
        self.mount_resource_packages_in_partition_with_hook(runtime_path, |_|{})
    }

    pub fn mount_resource_packages_in_partition_with_hook<F>(&mut self, runtime_path: &PathBuf, mut progress_callback: F,
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

        println!("{}: {:?}", &self.info.id, &patch_idx_result);

        let patch_indices = patch_idx_result.unwrap();

        let base_package_path = runtime_path.join(self.info.get_filename(&PatchId::Base));
        self.mount_package(base_package_path.as_path(), PatchId::Base)?;

        for (index, patch_id) in patch_indices.iter().enumerate() {
            let patch_package_path = runtime_path.join(
                self.info.get_filename(patch_id)
            );
            self.mount_package(patch_package_path.as_path(), *patch_id)?;

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

    fn mount_package(&mut self, package_path: &Path, patch_index: PatchId) -> Result<(), ResourcePartitionError> {
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

    #[allow(dead_code)]
    pub(crate) fn get_num_patches(&self) -> usize{
        self.packages.len().saturating_sub(1)
    }

    pub fn get_latest_resources(&self) -> Result<Vec<(&ResourceInfo, &PatchId)>> {
        self.resources.iter().map(|(rrid, idx)| {
            Ok((self.get_resource_info_from(rrid, idx)?, idx))
        }).collect()
    }

    pub fn get_resource(&self, rrid: &RuntimeResourceID) -> Result<Vec<u8>, ResourcePartitionError> {
        let package_index = self.resources.get(rrid).ok_or(ResourcePartitionError::ResourceNotAvailable)?;
        let rpkg = self.packages.get(package_index).ok_or(ResourcePartitionError::NotMounted)?;
        let mut package_path = self.mount_location.clone().ok_or(ResourcePartitionError::NotMounted)?;
        package_path = package_path.join(self.info.get_filename(package_index));
        rpkg.get_resource(package_path.as_path(), rrid).map_err(|e| ResourcePartitionError::ReadResourcePackageError(e, self.info.get_filename(package_index)))
    }

    pub fn get_glacier_resource<T>(&self, woa_version: WoaVersion, rrid: &RuntimeResourceID) -> Result<T::Output, ResourcePartitionError>
        where T: GlacierResource {
        let package_index = self.resources.get(rrid).ok_or(ResourcePartitionError::ResourceNotAvailable)?;
        let rpkg = self.packages.get(package_index).ok_or(ResourcePartitionError::NotMounted)?;
        let mut package_path = self.mount_location.clone().ok_or(ResourcePartitionError::NotMounted)?;
        package_path = package_path.join(self.info.get_filename(package_index));
        let bytes = rpkg.get_resource(package_path.as_path(), rrid).map_err(|e| ResourcePartitionError::ReadResourcePackageError(e, self.info.get_filename(package_index)))?;
        T::process_data(woa_version, bytes).map_err(ResourcePartitionError::ResourceError)
    }

    pub fn get_resource_from(&self, rrid: &RuntimeResourceID, patch_id: &PatchId) -> Result<Vec<u8>, ResourcePartitionError> {
        let rpkg = self.packages.get(patch_id).ok_or(ResourcePartitionError::NotMounted)?;
        let mut package_path = self.mount_location.clone().ok_or(ResourcePartitionError::NotMounted)?;
        package_path = package_path.join(self.info.get_filename(patch_id));
        rpkg.get_resource(package_path.as_path(), rrid).map_err(|e| ResourcePartitionError::ReadResourcePackageError(e, self.info.get_filename(patch_id)))
    }

    pub fn get_resource_info(&self, rrid: &RuntimeResourceID) -> Result<&ResourceInfo, ResourcePartitionError> {
        let package_index = self.resources.get(rrid).ok_or(ResourcePartitionError::ResourceNotAvailable)?;
        let rpkg = self.packages.get(package_index).ok_or(ResourcePartitionError::NotMounted)?;
        rpkg.get_resource_info(rrid).ok_or(ResourcePartitionError::ResourceNotAvailable)
    }

    pub fn get_resource_info_from(&self, rrid: &RuntimeResourceID, patch_id: &PatchId) -> Result<&ResourceInfo, ResourcePartitionError> {
        let rpkg = self.packages.get(patch_id).ok_or(ResourcePartitionError::NotMounted)?;
        rpkg.get_resource_info(rrid).ok_or(ResourcePartitionError::ResourceNotAvailable)
    }

    pub fn get_partition_info(&self) -> &PartitionInfo {
        &self.info
    }

    pub fn get_resource_patch_indices(&self, rrid: &RuntimeResourceID) -> Vec<&PatchId> {
        self.packages.iter().filter(|(_, package)| package.has_resource(rrid)).map(|(id, _)| id).collect::<Vec<&PatchId>>()
    }

    pub fn get_resource_removal_indices(&self, rrid: &RuntimeResourceID) -> Vec<&PatchId> {
        self.packages.iter().filter(|(_, package)| package.removes_resource(rrid)).map(|(id, _)| id).collect::<Vec<&PatchId>>()
    }
}

impl Debug for ResourcePartition {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let total = self.packages.values().map(|v| v.get_resource_ids().len()).sum::<usize>();
        write!(f, "{{index: {}, name: {}, edge_resources: {}, total_resources: {} }}", self.info.get_filename(&PatchId::Base), self.info.name.clone().unwrap_or_default(), self.resources.len(), total)?;

        Ok(())
    }
}
