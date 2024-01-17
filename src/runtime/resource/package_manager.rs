use anyhow::{anyhow, Error};
use regex::Regex;
use std::cell::RefCell;
use std::path::{Path, PathBuf};

use crate::{encryption::xtea::Xtea, utils, misc::resource_id::ResourceID};
use super::resource_container::ResourceContainer;

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum EPartitionType {
    PartitionTypeStandard = 0,
    PartitionTypeAddon = 1,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
struct PartitionState {
    available: bool,
    installing: bool,
    mounted: bool,
    install_progress: f32,
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct PartitionInfo {
    pub name: String,
    pub parent: String,
    pub part_type: EPartitionType,
    pub index: usize,
    //patchlevel is left out here. We can't use it because of the custom patch levels set by most tools.
    pub roots: RefCell<Vec<ResourceID>>,

    #[cfg_attr(feature = "serde", serde(skip_serializing))]
    state: PartitionState,
}

pub struct PackageManager {
    pub runtime_dir: PathBuf,
    pub partition_infos: Vec<PartitionInfo>,
}

impl PackageManager {
    pub fn new(runtime_path: &impl AsRef<Path>) -> Self {
        let partition_infos = Self::get_partition_infos(
            PathBuf::from(runtime_path.as_ref()).join("packagedefinition.txt").as_path()
        ).unwrap_or(vec![]);

        Self { partition_infos, runtime_dir: PathBuf::from(runtime_path.as_ref()) }
    }

    pub fn initialize(&mut self, resource_container: &mut ResourceContainer) -> Result<(), Error> {
        for partition in self.partition_infos.iter_mut() {
            if partition.state.available {
                Self::mount_resource_packages_in_partition(resource_container, partition, &self.runtime_dir)?;
            }
            //set the partition to mounted somewhere down here.
            //also async the shit out of this :)
        };
        Ok(())
    }

    fn mount_resource_packages_in_partition(resource_container: &mut ResourceContainer, partition_info: &PartitionInfo, runtime_path: &PathBuf) -> Result<(), Error> {
        resource_container.mount_partition(partition_info, runtime_path)
    }

    fn get_partition_infos(path: &Path) -> Result<Vec<PartitionInfo>, anyhow::Error> {
        if let Ok(bytes) = utils::get_file_as_byte_vec(path) {
            let deciphered_data = match Xtea::is_encrypted_text_file(&bytes) {
                true => Xtea::decrypt_text_file(&bytes, &Xtea::DEFAULT_KEY)?,
                false => match String::from_utf8(bytes) {
                    Ok(v) => v,
                    Err(e) => return Err(anyhow!("Text encoding error: {}", e)),
                },
            };

            let mut partitions: Vec<PartitionInfo> = vec![];

            //define the regex
            let partition_regex =
                Regex::new(r"@partition name=(.+?) parent=(.+?) type=(.+?) patchlevel=(.+?)")
                    .unwrap();
            let resource_path_regex = Regex::new(r"(\[[a-z]+:/.+?]).([a-z]+)").unwrap();

            //try to match the regex on a per-line basis
            for line in deciphered_data.split("\r\n").collect::<Vec<&str>>() {
                if partition_regex.is_match(line) {
                    if let Some(m) = partition_regex.captures_iter(line).next() {
                        partitions.push(PartitionInfo {
                            name: (m[1]).parse().unwrap(),
                            parent: (m[2]).parse().unwrap(),
                            part_type: if &m[3] == "standard" {
                                EPartitionType::PartitionTypeStandard
                            } else {
                                EPartitionType::PartitionTypeAddon
                            },
                            roots: RefCell::new(vec![]),
                            index: partitions.len(),
                            state: PartitionState {
                                available: true,
                                installing: false,
                                mounted: false,
                                install_progress: 0.0,
                            },
                        });
                    }
                } else if resource_path_regex.is_match(line) {
                    if let Some(m) = resource_path_regex.captures_iter(line).next(){
                        partitions.last().unwrap().roots.borrow_mut().push( //TODO: fix risky assumption
                                                                            ResourceID::from_string(format!("{}.pc_{}", &m[1], &m[2]).as_str()),
                        );
                    }
                }
            }
            Ok(partitions)
        } else {
            Err(anyhow!("Failed to find the given packagedefinition file"))
        }
    }

    // fn get_partitions_for_roots(&mut self,
    //     root_resources: &Vec<ResourceID>,
    //     found_unknown_root: &mut bool,
    // ) -> Vec<&PartitionInfo> {
    //     unimplemented!()
    // }

    // fn mount_partitions_for_roots(&mut self, root_resources: &Vec<ResourceID>) {
    //     unimplemented!()
    // }
}
