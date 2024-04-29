use rpkg_rs::misc::resource_id::ResourceID;
use rpkg_rs::resource::pdefs::PartitionInfo;
use rpkg_rs::resource::resource_partition::ResourcePartition;
use rpkg_rs::resource::runtime_resource_id::RuntimeResourceID;
use std::env;
use std::path::PathBuf;
use std::str::FromStr;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 4 {
        eprintln!("Usage: cargo run --example <example_name> -- <runtime directory> <partition id (e.g. chunk0, dlc3)> <ResourceId to extract>");
        return;
    }

    //set the args
    let runtime_path = PathBuf::from(&args[1]);
    let rid = ResourceID::from_str(&args[3]).unwrap_or_else(|_| {
        println!("Given ResourceID is invalid");
        std::process::exit(0)
    });

    let rrid: RuntimeResourceID = RuntimeResourceID::from_resource_id(&rid);

    let partition_info = PartitionInfo::from_id(&args[2]).unwrap_or_else(|e| {
        println!("Failed parse partition id: {:?}", e);
        std::process::exit(0)
    });

    let mut partition = ResourcePartition::new(partition_info);
    print!("Mounting partition {} ", &partition.partition_info().id());

    partition
        .mount_resource_packages_in_partition(&runtime_path)
        .unwrap_or_else(|e| {
            println!("Failed parse resource partition: {:?}", e);
            std::process::exit(0)
        });

    println!("Extracting the resource");
    let file = partition.read_resource(&rrid).unwrap_or_else(|e| {
        println!("Failed extract resource: {:?}", e);
        std::process::exit(0)
    });

    println!("Resource extracted!");
    match std::str::from_utf8(&*file) {
        Ok(s) => {
            println!("{}...", s.chars().take(100).collect::<String>())
        }
        Err(_) => {
            println!(
                "first bytes: {:?}",
                file.iter().take(50).collect::<Vec<_>>()
            );
        }
    };
}
