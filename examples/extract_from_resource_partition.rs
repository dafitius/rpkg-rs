use std::{env, io};
use std::io::Write;
use std::path::PathBuf;
use rpkg_rs::misc::resource_id::ResourceID;
use rpkg_rs::runtime::resource::package_defs::{PartitionInfo};
use rpkg_rs::runtime::resource::partition_manager::PartitionState;
use rpkg_rs::runtime::resource::resource_partition::ResourcePartition;
use rpkg_rs::runtime::resource::runtime_resource_id::RuntimeResourceID;

fn main() {

    let args: Vec<String> = env::args().collect();

    if args.len() < 4{
        eprintln!("Usage: cargo run --example <example_name> -- <runtime directory> <partition id (e.g. chunk0, dlc3)> <ResourceId to extract>");
        return;
    }

    //set the args
    let runtime_path = PathBuf::from(&args[1]);
    let rid = ResourceID::from_string(&args[3]);
    let rrid: RuntimeResourceID = RuntimeResourceID::from_resource_id(&rid);

    let partition_info = PartitionInfo::from_id(&args[2]).unwrap_or_else(|e| {
        println!("Failed parse partition id: {:?}", e);
        std::process::exit(0)
    });
    
    let mut partition = ResourcePartition::new(partition_info);
    print!("Mounting partition {} ", &partition.get_partition_info().id);


    let mut progress = 0.0;
    let progress_callback = |state: &PartitionState| {

        let install_progress= (state.install_progress*10.0).ceil()/10.0;

        let chars_to_add = (install_progress*10.0 - progress * 10.0) as usize * 2;
        let chars_to_add = std::cmp::min(chars_to_add, 20);
        print!("{}", "█".repeat(chars_to_add));
        io::stdout().flush().unwrap();

        progress = install_progress;

        if progress == 1.0{
            progress = 0.0;
            println!(" done :)");
        }
    };

    partition.mount_resource_packages_in_partition(&runtime_path, progress_callback).unwrap_or_else(|e|{
        println!("Failed parse resource partition: {:?}", e);
        std::process::exit(0)
    });

    println!("Extracting the resource");
    let file = partition.get_resource(&rrid).unwrap_or_else(|e| {
        println!("Failed extract resource: {:?}", e);
        std::process::exit(0)
    });

    println!("Resource extracted!");
    match std::str::from_utf8(&*file) {
        Ok(s) => {
            println!("{}...", s.chars().take(100).collect::<String>())
        }
        Err(_) => {
            println!("first bytes: {:?}", file.iter().take(50).collect::<Vec<_>>());
        }
    };
}