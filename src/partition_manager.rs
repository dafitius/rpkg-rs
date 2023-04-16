use crate::package_definitions::PackageDefinitions;
use crate::resource_partition::ResourcePartition;
use crate::resource::ResourceInfo;

#[derive(Default)]
pub struct PartitionManager {
    pub partitions: Vec<ResourcePartition>
}

impl PartitionManager {
    pub fn parse_into(&mut self, package_defs: &PackageDefinitions, runtime_path: &str, resources: &mut Vec<ResourceInfo>) {

        let mut partitions :Vec<ResourcePartition> = Vec::new();
        for i in 0..package_defs.partitions.len() {
            let mut partition : ResourcePartition = ResourcePartition::new(runtime_path.to_string(), i as u16);
            partition.read_patch_indices().expect("Reading the runtime folder");
            partition.read_partition(resources);
            partitions.push(partition);
        }
        self.partitions = partitions
    }

    pub fn find_partition(&self, r_index: usize) -> Option<&ResourcePartition>{
        self.partitions.iter().find(|&partition| partition.resource_bounds.0 < r_index && partition.resource_bounds.1 > r_index)
    }
}