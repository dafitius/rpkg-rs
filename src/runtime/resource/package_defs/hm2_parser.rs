use std::cell::RefCell;
use regex::Regex;
use crate::encryption::xtea::Xtea;
use crate::misc::resource_id::ResourceID;
use crate::runtime::resource::package_defs::{PackageDefinitionError, PartitionInfo, PackageDefinitionParser, PartitionType, PartitionId};

pub struct HM2Parser;

impl PackageDefinitionParser for HM2Parser {
    fn parse(data: &[u8]) -> Result<Vec<PartitionInfo>, PackageDefinitionError> {
        let deciphered_data = match Xtea::is_encrypted_text_file(data) {
            true => Xtea::decrypt_text_file(data)?,
            false => match String::from_utf8(data.to_vec()) {
                Ok(v) => v,
                Err(e) => return Err(PackageDefinitionError::TextEncodingError(e)),
            },
        };

        let mut partitions: Vec<PartitionInfo> = Vec::new();
        let mut previous_lines: [&str; 2] = ["", ""];

        let partition_regex =
            Regex::new(r"@([A-z]+) patchlevel=([0-9]+)")
                .unwrap();

        let resource_path_regex = Regex::new(r"(\[[a-z]+:/.+?]).([a-z]+)").unwrap();

        for line in deciphered_data.lines() {
            let trimmed_line = line.trim();

            match trimmed_line {
                _ if trimmed_line.starts_with("//") => {}, //comment
                line if partition_regex.is_match(trimmed_line) => {
                    if let Some(m) = partition_regex.captures_iter(line).next() {
                        let part_type = if &m[1] == "chunk" {
                            PartitionType::Standard
                        } else {
                            PartitionType::Dlc
                        };

                        partitions.push(PartitionInfo {
                            name: try_get_partition_name(previous_lines.to_vec()),
                            parent: partitions.iter().map(|p| p.id.clone()).next(),
                            id : PartitionId{
                                part_type: part_type.clone(),
                                index: partitions.iter().filter(|&p| p.id.part_type == part_type).count()
                            },
                            patchlevel: (m[2]).parse().unwrap(),
                            roots: vec![],
                        });
                    }
                }
                line if resource_path_regex.is_match(trimmed_line) => {
                    if let Some(m) = resource_path_regex.captures_iter(line).next() {
                        if let Some(current_partition) = partitions.last_mut(){
                            if let Ok(rid) = ResourceID::from_string(format!("{}.{}", &m[1], &m[2]).as_str()) {
                                current_partition.add_root(rid);
                            }
                        }
                    };
                }
                _ => {}
            }

            previous_lines[0] = previous_lines[1];
            previous_lines[1] = line;
        }

        Ok(partitions)
    }
}

fn try_get_partition_name(lines: Vec<&str>) -> Option<String> {
    let reg =
        Regex::new(r"\/\/ --- (?:DLC|Chunk) \d{2,2} (.*)")
            .unwrap();
    for line in lines {
        if reg.is_match(line) {
            if let Some(m) = reg.captures_iter(line).next() {
                return Some(m[1].to_string());
            }
        }
    }
    None
}