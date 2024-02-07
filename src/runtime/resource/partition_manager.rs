use std::path::{PathBuf};
use thiserror::Error;

use crate::runtime::resource::package_defs::{PackageDefinitionError, PackageDefinitionSource, PartitionInfo};
use crate::runtime::resource::resource_info::ResourceInfo;
use crate::runtime::resource::runtime_resource_id::{RuntimeResourceID};
use super::resource_partition::{ResourcePartition, ResourcePartitionError};

#[derive(Debug, Error)]
pub enum PackageManagerError {
    #[error("Cannot use packagedefinition config: {0}")]
    PackageDefinitionError(#[from] PackageDefinitionError),

    #[error("partition error: {0}")]
    PartitionError(#[from] ResourcePartitionError),

    #[error("partition {0} could not be found")]
    PartitionNotFound(String),
}

#[allow(dead_code)]
#[derive(Clone, Debug, Copy)]
pub struct PartitionState {
    pub(crate) installing: bool,
    pub(crate) mounted: bool,
    pub install_progress: f32,
}

pub struct PartitionManager {
    runtime_directory: PathBuf,
    partitions: Vec<ResourcePartition>,
}

impl PartitionManager {
    pub fn new(runtime_directory: PathBuf) -> Self {
        Self { runtime_directory, partitions: vec![] }
    }

    pub fn mount_partitions<F>(&mut self, partition_source: PackageDefinitionSource, mut progress_callback: F) -> Result<(), PackageManagerError>
        where
            F: FnMut(usize, &PartitionState),
    {
        let partitions = partition_source.read().map_err(PackageManagerError::PackageDefinitionError)?;

        for (index, partition) in partitions.into_iter().enumerate() {
            let callback = |state: &_| {
                progress_callback(index + 1, state);
            };

            self.mount_partition(partition, callback)?;
        }
        Ok(())
    }

    pub fn mount_partition<F>(&mut self, partition_info: PartitionInfo, mut progress_callback: F) -> Result<(), PackageManagerError>
        where
            F: FnMut(&PartitionState),
    {
        let mut partition = ResourcePartition::new(partition_info);
        let mut state_result : PartitionState = PartitionState{
            installing: false,
            mounted: false,
            install_progress: 0.0,
        };

        let callback = |state: &_| {
            progress_callback(state);
            state_result = *state;
        };

        partition.mount_resource_packages_in_partition(&self.runtime_directory, callback)?;

        if state_result.mounted {
            self.partitions.push(partition);
        }

        Ok(())
    }

    pub fn get_resource_from(&self, partition_index: usize, rrid: RuntimeResourceID) -> Result<Vec<u8>, PackageManagerError>{
        let partition = self.partitions.iter().find(|partition| partition.info.id.index == partition_index);
        if let Some(partition) = partition{
           match partition.get_resource(&rrid){
               Ok(data) => { Ok(data)}
               Err(e) => {Err(PackageManagerError::PartitionError(e))}
           }
        }else{
            Err(PackageManagerError::PartitionNotFound(partition_index.to_string()))
        }
    }

    pub fn get_resource_info_from(&self, partition_index: usize, rrid: RuntimeResourceID) -> Result<&ResourceInfo, PackageManagerError>{
        let partition = self.partitions.iter().find(|partition| partition.info.id.index == partition_index);
        if let Some(partition) = partition{
            match partition.get_resource_info(&rrid){
                Ok(info) => { Ok(info)}
                Err(e) => {Err(PackageManagerError::PartitionError(e))}
            }
        }else{
            Err(PackageManagerError::PartitionNotFound(partition_index.to_string()))
        }
    }

    pub fn print_resource_changelog(&self, partition_index: usize, rrid: RuntimeResourceID) -> Result<Vec<u8>, PackageManagerError>{
        let partition = self.partitions.iter().find(|partition| partition.info.id.index == partition_index);
        if let Some(partition) = partition{
            match partition.get_resource(&rrid){
                Ok(data) => { Ok(data)}
                Err(e) => {Err(PackageManagerError::PartitionError(e))}
            }
        }else{
            Err(PackageManagerError::PartitionNotFound(partition_index.to_string()))
        }
    }
}
