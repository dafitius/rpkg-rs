use crate::encryption::xtea::Xtea;
use crate::misc::resource_id::ResourceID;
use crate::resource::pdefs::{
    PackageDefinitionError, PackageDefinitionParser, PartitionId, PartitionInfo, PartitionType,
};
use lazy_regex::regex;
use std::str::FromStr;

pub struct HM3Parser;

impl PackageDefinitionParser for HM3Parser {
    fn parse(data: &[u8]) -> Result<Vec<PartitionInfo>, PackageDefinitionError> {
        let deciphered_data = match Xtea::is_encrypted_text_file(data) {
            true => Xtea::decrypt_text_file(data)?,
            false => match String::from_utf8(data.to_vec()) {
                Ok(v) => v,
                Err(e) => return Err(PackageDefinitionError::TextEncodingError(e)),
            },
        };

        let mut partitions: Vec<PartitionInfo> = vec![];

        //define the regex
        let partition_regex =
            regex!(r"@partition name=(.+?) parent=(.+?) type=(.+?) patchlevel=(.\d*)");
        let resource_path_regex = regex!(r"(\[[a-z]+:/.+?]).([a-z]+)");

        //try to match the regex on a per-line basis
        for line in deciphered_data.lines() {
            if partition_regex.is_match(line) {
                if let Some(m) = partition_regex.captures_iter(line).next() {
                    partitions.push(PartitionInfo {
                        name: m[1].parse().ok(),
                        parent: find_parent_id(&partitions, m[2].parse().unwrap()),
                        id: PartitionId {
                            part_type: match &m[3] {
                                "standard" => PartitionType::Standard,
                                "addon" => PartitionType::Addon,
                                _ => PartitionType::Standard,
                            },
                            index: partitions.len(),
                        },
                        patch_level: m[4].parse().unwrap(),
                        roots: vec![],
                    });
                }
            } else if resource_path_regex.is_match(line) {
                if let Some(m) = resource_path_regex.captures_iter(line).next() {
                    if let Some(current_partition) = partitions.last_mut() {
                        if let Ok(rid) =
                            ResourceID::from_str(format!("{}.{}", &m[1], &m[2]).as_str())
                        {
                            current_partition.add_root(rid);
                        }
                    } else {
                        return Err(PackageDefinitionError::UnexpectedFormat("ResourceID defined before partition, are you using the correct game version?".to_string()));
                    }
                }
            }
        }
        Ok(partitions)
    }
}

fn find_parent_id(partitions: &[PartitionInfo], parent_name: String) -> Option<PartitionId> {
    partitions
        .iter()
        .find(|&partition| partition.name.as_ref().is_some_and(|s| s == &parent_name))
        .map(|parent_partition| parent_partition.id.clone())
}
