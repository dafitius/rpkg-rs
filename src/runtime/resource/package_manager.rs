use anyhow::{anyhow, Error};
use regex::Regex;
use serde::Serialize;
use std::cell::RefCell;

use crate::{encryption::xtea::Xtea, utils, misc::resource_id::ResourceID};

use super::resource_container::ResourceContainer;

#[derive(Serialize, Clone, Debug)]
pub enum EPartitionType {
    PartitionTypeStandard = 0,
    PartitionTypeAddon = 1,
}

#[derive(Serialize, Clone, Debug)]
pub struct PartitionInfo {
    pub name: String,
    pub parent: String,
    pub part_type: EPartitionType,
    pub index: usize,
    //patchlevel is left out here. We can't use it because of the custom patch levels set by most tools.
    pub roots: RefCell<Vec<ResourceID>>,

    //state bools
    available: bool,
    installing: bool,
    mounted: bool,

    install_progress: f32,
}

pub struct PackageManager {
    pub runtime_dir: String,
    pub partition_infos: Vec<PartitionInfo>,
}

impl PackageManager {
    pub fn new(runtime_path: &str) -> Self {
        let partition_infos = Self::get_partition_infos(
            format!("{}/{}", runtime_path, "packagedefinition.txt").as_str(),
        )
        .unwrap_or(vec![]);

        Self { partition_infos, runtime_dir: runtime_path.to_string() }
    }

    pub fn initialize(&mut self, resource_container: &mut ResourceContainer) -> Result<(), Error>{
        
        for partition in self.partition_infos.iter_mut(){
            if partition.available{
                Self::mount_resource_packages_in_partition(resource_container, partition, &self.runtime_dir)?;
            }
            //set the partition to mounted somewhere down here.
            //also async the shit out of this :)
        };
        Ok(())
    }

    fn mount_resource_packages_in_partition(resource_container: &mut ResourceContainer, partition_info: &PartitionInfo, runtime_path: &str ) -> Result<(), Error>{
        resource_container.mount_partition(partition_info,runtime_path)
    }

    fn get_partition_infos(path: &str) -> Result<Vec<PartitionInfo>, anyhow::Error> {

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
                match line {
                    _ if partition_regex.is_match(line) => {
                        for m in partition_regex.captures_iter(line) {
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
                                available: true,
                                installing: false,
                                mounted: false,
                                install_progress: 0.0,
                            });
                            continue; //we do not want a second match, even if there is one
                        }
                    }

                    _ if resource_path_regex.is_match(line) => {
                        for m in resource_path_regex.captures_iter(line) {
                            partitions.last().unwrap().roots.borrow_mut().push(
                                ResourceID::from_string(format!("{}.pc_{}", &m[1], &m[2]).as_str()),
                            );
                            continue; //we do not want a second match, even if there is one
                        }
                    }
                    _ => {}
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
