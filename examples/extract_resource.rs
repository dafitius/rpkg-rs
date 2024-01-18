use std::path::PathBuf;
use rpkg_rs::misc::resource_id::ResourceID;
use rpkg_rs::runtime::resource::resource_package::{ResourcePackage};
use rpkg_rs::runtime::resource::runtime_resource_id::RuntimeResourceID;

fn main() {

    //set the args
    let package_path = PathBuf::from("S:/Steam/steamapps/common/Hitman™/Runtime/chunk0.rpkg");
    let rid = ResourceID::from_string("[assembly:/repository/pro.repo].pc_repo");
    let rrid: RuntimeResourceID = RuntimeResourceID::from_resource_id(&rid);

    //parse the ResourcePackage
    let rpkg = ResourcePackage::from_file(&package_path).unwrap_or_else(|e|{
        println!("Failed parse resource package: {}", e);
        std::process::exit(1)
    });

    //extract the resource
    let file = rpkg.get_resource(&package_path, &rrid).unwrap_or_else(|e| {
        println!("Failed extract resource: {}", e);
        std::process::exit(1)
    });

    //print the resource
    match std::str::from_utf8(&*file) {
        Ok(s) => {
            println!("{}...", s.chars().take(100).collect::<String>())
        }
        Err(_) => {
            println!("first bytes: {:?}", file.iter().take(50).collect::<Vec<_>>());
        }
    };
}