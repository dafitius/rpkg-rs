use crate::encryption::xtea::Xtea;
use crate::misc::resource_id::ResourceID;
use crate::resource::pdefs::{
    PackageDefinitionError, PackageDefinitionParser, PartitionId, PartitionInfo, PartitionType,
};
use lazy_regex::regex;
use std::str::FromStr;

pub struct H2016Parser;

impl PackageDefinitionParser for H2016Parser {
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

        let partition_regex = regex!(r"#([A-z]+) patchlevel=([0-9]+)");

        let langdlc_regex = regex!(r"#langdlc ([A-z]+)");

        let resource_path_regex = regex!(r"(\[[a-z]+:/.+?]).([a-z]+)");

        for line in deciphered_data.lines() {
            let trimmed_line = line.trim();

            match trimmed_line {
                _ if trimmed_line.starts_with("##") => {} //comment
                line if partition_regex.is_match(trimmed_line) => {
                    if let Some(m) = partition_regex.captures_iter(line).next() {
                        let part_type = match &m[1] {
                            "dlc" => PartitionType::Dlc,
                            "chunk" => PartitionType::Standard,
                            _ => PartitionType::Standard,
                        };

                        partitions.push(PartitionInfo {
                            name: try_read_partition_name(previous_lines.to_vec()),
                            parent: partitions.iter().map(|p| p.id.clone()).next(),
                            id: PartitionId {
                                part_type: part_type.clone(),
                                index: partitions
                                    .iter()
                                    .filter(|&p| p.id.part_type == part_type)
                                    .count(),
                            },
                            patch_level: m[2].parse().unwrap(),
                            roots: vec![],
                        });
                    }
                }
                line if langdlc_regex.is_match(trimmed_line) => {
                    if let Some(m) = langdlc_regex.captures_iter(line).next() {
                        let language_code = &m[1];
                        let mut lang_partitions = vec![];
                        for partition in partitions.iter() {
                            lang_partitions.push(PartitionInfo {
                                name: None,
                                parent: Some(partition.id.clone()),
                                id: PartitionId {
                                    part_type: match partition.id.part_type {
                                        PartitionType::Standard => PartitionType::LanguageStandard(
                                            language_code.parse().unwrap(),
                                        ),
                                        PartitionType::Dlc => PartitionType::LanguageDlc(
                                            language_code.parse().unwrap(),
                                        ),
                                        _ => PartitionType::LanguageDlc(
                                            language_code.parse().unwrap(),
                                        ),
                                    },
                                    index: partition.id.index,
                                },
                                patch_level: 0, //doesn't matter, this will be checked later
                                roots: vec![],
                            });
                        }
                        partitions.append(&mut lang_partitions);
                    }
                }
                line if resource_path_regex.is_match(trimmed_line) => {
                    if let Some(m) = resource_path_regex.captures_iter(line).next() {
                        if let Some(current_partition) = partitions.last_mut() {
                            if let Ok(rid) =
                                ResourceID::from_str(format!("{}.{}", &m[1], &m[2]).as_str())
                            {
                                current_partition.roots.push(rid);
                            }
                        }
                    }
                }
                _ => {}
            }

            previous_lines[0] = previous_lines[1];
            previous_lines[1] = line;
        }

        Ok(partitions)
    }
}

fn try_read_partition_name(lines: Vec<&str>) -> Option<String> {
    let reg = regex!(r"## --- +(?:DLC|Chunk )\d{2} (.*)");
    for line in lines {
        if reg.is_match(line) {
            if let Some(m) = reg.captures_iter(line).next() {
                return Some(m[1].to_string());
            }
        }
    }
    None
}
