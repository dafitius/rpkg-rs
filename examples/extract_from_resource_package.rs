use clap::{Arg, Command};
use rpkg_rs::misc::resource_id::ResourceID;
use rpkg_rs::resource::legacy::Format;
use rpkg_rs::resource::resource_package::ResourcePackage;
use rpkg_rs::resource::runtime_resource_id::RuntimeResourceID;
use std::path::PathBuf;
use std::str::FromStr;

fn main() {
    let matches = Command::new("Extract from rpkg example")
        .about("Extracts a resource from a package using rpkg_rs")
        .arg(
            Arg::new("package")
                .help("Path to the resource package")
                .required(true)
                .value_parser(clap::value_parser!(PathBuf)),
        )
        .arg(
            Arg::new("resource_id")
                .help("ResourceID to extract")
                .required(true),
        )
        .arg(
            Arg::new("legacy")
                .help("Read legacy resource content")
                .long("legacy")
                .short('l')
                .action(clap::ArgAction::SetTrue),
        )
        .get_matches();

    let package_path: PathBuf = matches.get_one::<PathBuf>("package").unwrap().clone();
    let rid_str = matches.get_one::<String>("resource_id").unwrap();
    let legacy = *matches.get_one::<bool>("legacy").unwrap_or(&false);

    let rid = ResourceID::from_str(rid_str).unwrap_or_else(|_| {
        println!("Given ResourceID is invalid");
        std::process::exit(1)
    });

    let rrid: RuntimeResourceID = RuntimeResourceID::from_resource_id(&rid);

    println!("Parsing the resource package at {}", package_path.display());
    let rpkg = if !legacy {
        ResourcePackage::from_file(&package_path)
    } else {
        rpkg_rs::resource::legacy::read_package_from_file(
            Format::CL535848,
            package_path,
        )
    }
    .unwrap_or_else(|e| {
        println!("Failed parse resource package: {}", e);
        std::process::exit(0)
    });

    println!("Extracting the resource");
    let file = rpkg.read_resource(&rrid).unwrap_or_else(|e| {
        println!("Failed extract resource: {}", e);
        std::process::exit(0)
    });

    let resource_info = rpkg.resources().get(&rrid).unwrap_or_else(|| {
        println!("Failed to get resource info.");
        std::process::exit(0)
    });

    println!("Resource extracted!");
    println!("Resource type: {:?}", resource_info.data_type());
    println!("Resource size: {}", resource_info.size());
    println!(
        "System memory requirement: {}",
        resource_info.system_memory_requirement()
    );
    println!(
        "Video memory requirement: {}",
        resource_info.video_memory_requirement()
    );
    println!("References: {}", resource_info.references().len());

    for (rrid, flags) in resource_info.references() {
        println!("[+] Ref {}", rrid);
        println!("    Language code: {:?}", flags.language_code());
        println!("    Is acquired: {}", flags.is_acquired());
        println!("    Reference type: {:?}", flags.reference_type());
    }

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
