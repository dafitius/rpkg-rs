use std::cell::{RefCell};
use regex::Regex;
use anyhow::{anyhow, Error};
use serde::Serialize;
use crate::resource_id::ResourceID;
use crate::utils;

#[derive(Default)]
pub struct PackageDefinitions {
    pub partitions: Vec<PartitionInfo>,
}

#[derive(Serialize, Clone)]
pub enum EPartitionType {
    PartitionTypeStandard = 0,
    PartitionTypeAddon = 1,
}

#[derive(Serialize, Clone)]
pub struct PartitionInfo {
    pub name: String,
    pub parent: String,
    pub part_type: EPartitionType,
    //patchlevel is left out here. We can't use it because of the custom patch levels set by most tools.
    pub roots: RefCell<Vec<ResourceID>>
}

fn decipher(data: Box<[u8]>) -> Result<Vec<u8>, Error> {
    let header = Box::new(Vec::from([0x22, 0x3d, 0x6f, 0x9a, 0xb3, 0xf8, 0xfe, 0xb6, 0x61, 0xd9, 0xcc, 0x1c, 0x62, 0xde, 0x83, 0x41]));
    let key = Box::new(Vec::from([0x30f95282, 0x1f48c419, 0x295f8548, 0x2a78366d]));
    let delta = 0x61c88647;
    let rounds = 32;

    let data_pointer = Box::into_raw(data);
    let unwrapped_data = unsafe { data_pointer.as_ref().expect("data is null") };
    if unwrapped_data.len() < 2 {
        panic!("data is < 2 for some reason");
    }

    let header_pointer = Box::into_raw(header);
    let unwrapped_header = unsafe { header_pointer.as_ref().expect("header is null") };

    let key_pointer = Box::into_raw(key);

    let unwrapped_key = unsafe { key_pointer.as_ref().expect("key is null") };
    if unwrapped_key.len() < 4 {
        panic!("key is < 2 for some reason");
    }

    let res = hitman_xtea::decipher_file(
        unwrapped_data,
        delta,
        unwrapped_header,
        rounds,
        unwrapped_key,
    );
    if res.is_err() {
        return Err(anyhow!("Couldn't decipher data"));
    }

    Ok(res.unwrap())
}

impl PackageDefinitions {
    pub fn new() -> Self
    {
        Self { partitions: vec![] }
    }

    pub fn parse_into(&mut self, path: String) -> Result<&Self, Error> {
        if let Ok(bytes) = utils::get_file_as_byte_vec(path.as_str()){
            let data = bytes.into_boxed_slice();

            if let Ok(deciphered_data) = decipher(data) {
    
                //convert the byte array to a string
                let s = match std::str::from_utf8(deciphered_data.as_slice()) {
                    Ok(v) => v,
                    Err(e) => return Err(anyhow!("Unable to read deciphered data: {}", e)),
                };
    
                let mut partitions: Vec<PartitionInfo> = vec![];
    
                //define the regex
                let partition_regex = Regex::new(r"@partition name=(.+?) parent=(.+?) type=(.+?) patchlevel=(.+?)").unwrap();
                let resource_path_regex = Regex::new(r"(\[[a-z]+:/.+?]).([a-z]+)").unwrap();
    
                //try to match the regex on a per-line basis
                for line in s.split("\r\n").collect::<Vec<&str>>() {
    
                    match line {
                        _ if partition_regex.is_match(line) => {
                            for m in partition_regex.captures_iter(line) {
                                partitions.push(PartitionInfo {
                                    name: (m[1]).parse().unwrap(),
                                    parent: (m[2]).parse().unwrap(),
                                    part_type:
                                    if &m[3] == "standard" {
                                        EPartitionType::PartitionTypeStandard
                                    } else {
                                        EPartitionType::PartitionTypeAddon
                                    },
                                    roots: RefCell::new(vec![]),
                                });
                                continue; //we do not want a second match, even if there is one
                            }
                        }
    
                        _ if resource_path_regex.is_match(line) => {
                            for m in resource_path_regex.captures_iter(line) {
                                partitions.last().unwrap().roots.borrow_mut().push(
                                    ResourceID::from_string( format!("{}.pc_{}", &m[1], &m[2]).as_str())
                                );
                                continue; //we do not want a second match, even if there is one
                            }
                        }
                        _ => {}
                    }
                }
                self.partitions = partitions;
                Ok(self)
    
            } else {
                Err(anyhow!("Failed to parse given packagedefinition file"))
            }
        } else {
            Err(anyhow!("Failed to find the given packagedefinition file"))
        }
    }
}