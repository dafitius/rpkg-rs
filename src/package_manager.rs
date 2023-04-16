use crate::package_definitions::{EPartitionType, PackageDefinitions};
use crate::resource_container::ResourceContainer;


struct PartitionInfo{
    e_type: EPartitionType,
    index: u32,
    partition_id: String,
    mount_path: String,

    //state bools
    ready: bool,
    installing: bool,
    mounted: bool,

    install_progress: f32,
}

pub struct PackageManager{
    definitions: PackageDefinitions,
}

impl PackageManager{
    pub fn new(runtime_path: String) -> Self
    {
        let mut definitions = PackageDefinitions::new();
        let package_definitions_path = format!("{runtime_path}\\packagedefinition.txt");
        std::println!("start reading package definitions {package_definitions_path}");
        definitions.parse_into(package_definitions_path).unwrap();
        Self{
            definitions
        }
    }

    pub fn mount_partitions_for_roots(&self, _resouce_container: &ResourceContainer) {

    }
}
