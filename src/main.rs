use std::time::Instant;
use anyhow::{anyhow, Error};
use rpkg_rs::partition_manager::PartitionManager;
use rpkg_rs::resource::ResourceInfo;
use rpkg_rs::thumbs;
use rpkg_rs::package_definitions::PackageDefinitions;

fn main() -> Result<(), Error> {
    let now = Instant::now();

    // println!("Start reading the hashlist");
    // let hash_list_timer = Instant::now();
    // let path_list_path = "D:\\HitmanProjects\\Tools\\rpkgtools2.24\\hash_list.txt";
    // let mut path_list = PathList::new();
    // path_list.parse_into(path_list_path).unwrap();
    // println!("Read the hash list in: {} ms", hash_list_timer.elapsed().as_millis());

    let retail_path = "D:\\Steam\\steamapps\\common\\HUTMAN 3\\retail";
    let thumbs_path = format!("{retail_path}\\thumbs.dat");

    std::println!("start reading thumbs {thumbs_path}");
    let mut thumbs = thumbs::Thumbs::new();
    thumbs.parse_into(thumbs_path).unwrap();

    if let (Some(proj_path), Some(relative_runtime_path)) = (thumbs.get_property("PROJECT_PATH"), thumbs.get_property("RUNTIME_PATH")) {

        let runtime_path = format!("{retail_path}\\{proj_path}\\{relative_runtime_path}");
        let mut package_definitions = PackageDefinitions::new();
        let package_definitions_path = format!("{runtime_path}\\packagedefinition.txt");
        std::println!("start reading package definitions {package_definitions_path}");
        package_definitions.parse_into(package_definitions_path).unwrap();

        println!();
        let mut resources = vec![];
        let mut partition_manager = PartitionManager::default();
        partition_manager.parse_into(&package_definitions, runtime_path.as_str(), &mut resources);
        print_resource_journey(0x00EE6B9C45CC038F, &partition_manager, &resources);
    } else {
        return Err(anyhow!("Missing properties inside thumbs.dat: \n\
        PROJECT_PATH: {},\n\
        RUNTIME_PATH: {}", thumbs.get_property("PROJECT_PATH").is_some(), thumbs.get_property("RUNTIME_PATH").is_some()));
    }

    std::println!("done in {} ms", now.elapsed().as_millis());


    Ok(())
}

fn print_resource_journey(runtime_resource_id: u64, partition_manager: &PartitionManager, resources: &Vec<ResourceInfo>) {

    println!("\n\n-----------------------");
    println!("Resource journey of {runtime_resource_id:X}\n");
    for resource_partition in partition_manager.partitions.iter() {
        for resource_idx in resource_partition.resource_indices.iter() {
            if let Some(resource) = resources.get(*resource_idx as usize) {
                if resource.entry.runtime_resource_id.id == runtime_resource_id {
                    println!("Found {} inside chunk{}patch{}", resource.entry.runtime_resource_id.to_hex_string(), resource_partition.index, resource_partition.get_package_index(*resource_idx as u64));
                    println!("with {} older versions", probe_old_resource_version(resources, resource, 0));
                    if probe_old_resource_version(resources, resource, 0) > 0 {
                        for _ in 0..probe_old_resource_version(resources, resource, 0) {
                            //let old_resource = resources.get(resource.last_index.unwrap()).unwrap();
                            println!("inside chunk{}patch{}", resource_partition.index, resource_partition.get_package_index(resource.last_index.unwrap() as u64));
                        }
                    }
                }
            }
        }
    }
    println!("-----------------------\n\n");

}

fn probe_old_resource_version(resources: &Vec<ResourceInfo>, resource: &ResourceInfo, depth: u32) -> u32 {
    if let Some(old_resource_index) = resource.last_index {
        if let Some(old_resource) = resources.get(old_resource_index) {
            return probe_old_resource_version(resources, old_resource, depth + 1);
        }
    }

    depth
}
