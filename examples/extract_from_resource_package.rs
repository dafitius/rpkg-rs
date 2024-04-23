use rpkg_rs::misc::resource_id::ResourceID;
use rpkg_rs::runtime::resource::resource_package::ResourcePackage;
use rpkg_rs::runtime::resource::runtime_resource_id::RuntimeResourceID;
use std::env;
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        eprintln!("Usage: cargo run --example <example_name> -- <path to a package> <ResourceId to extract>");
        return;
    }

    //set the args
    let package_path = PathBuf::from(&args[1]);
    let rid = ResourceID::from_str_checked(&args[2]).unwrap_or_else(|_| {
        println!("Given ResourceID is invalid");
        std::process::exit(0)
    });

    let rrid: RuntimeResourceID = RuntimeResourceID::from_resource_id(&rid);

    println!("Parsing the resource package at {}", package_path.display());
    let rpkg = ResourcePackage::from_file(&package_path).unwrap_or_else(|e| {
        println!("Failed parse resource package: {}", e);
        std::process::exit(0)
    });

    println!("Extracting the resource");
    let file = rpkg
        .read_resource(&package_path, &rrid)
        .unwrap_or_else(|e| {
            println!("Failed extract resource: {}", e);
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
