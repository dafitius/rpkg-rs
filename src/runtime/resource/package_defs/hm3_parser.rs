use std::cell::RefCell;
use regex::Regex;
use crate::encryption::xtea::Xtea;
use crate::misc::resource_id::ResourceID;
use crate::runtime::resource::package_defs::{PackageDefinitionError, PartitionInfo, PackageDefinitionParser, PartitionType, PartitionId};

pub struct HM3Parser;

impl PackageDefinitionParser for HM3Parser {
    fn parse(data: &[u8]) -> Result<Vec<PartitionInfo>, PackageDefinitionError> {
        let deciphered_data = match Xtea::is_encrypted_text_file(data) {
            true => Xtea::decrypt_text_file(data, &Xtea::DEFAULT_KEY)?,
            false => match String::from_utf8(data.to_vec()) {
                Ok(v) => v,
                Err(e) => return Err(PackageDefinitionError::TextEncodingError(e)),
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
                        name: (m[1]).parse().ok(),
                        parent: find_parent_id(&partitions, (m[2]).parse().unwrap()),
                        id: PartitionId{
                            part_type: match &m[3]{
                                "standard" => PartitionType::Standard,
                                "addon" => PartitionType::Addon,
                                _ => PartitionType::Standard,
                            },
                            index: partitions.len()
                        },
                        patchlevel: (m[4]).parse().unwrap(),
                        roots: RefCell::new(vec![]),
                    });
                }
            } else if resource_path_regex.is_match(line) {
                if let Some(m) = resource_path_regex.captures_iter(line).next() {
                    partitions.last().unwrap().roots.borrow_mut().push(ResourceID::from_string(format!("{}.pc_{}", &m[1], &m[2]).as_str()),
                    );
                }
            }
        }
        Ok(partitions)
    }
}

fn find_parent_id(partitions: &[PartitionInfo], parent_name: String) -> Option<PartitionId> {
    partitions.iter()
        .find(|&partition| partition.name.as_ref().is_some_and(|s| s == &parent_name))
        .map(|parent_partition| parent_partition.id.clone())
}
