use std::time::Instant;
use anyhow::{anyhow, Error};
use rpkg_rs::misc::hash_path_list::PathList;

use rpkg_rs::misc::ini_file::IniFile;
use rpkg_rs::runtime::resource::package_manager::PackageManager;
use rpkg_rs::runtime::resource::resource_container::ResourceContainer;

fn main() -> Result<(), Error> {
    let now = Instant::now();

    let mut path_list = PathList::new();

    match path_list.parse_into(r"D:\David\Hitman-modding\Tools\rpkgTools\2.25\hash_list.txt", true) {
        Ok(_) => {
            println!("{}", now.elapsed().as_nanos());
            for path in path_list.get_all_folders() {
                println!("{}", path);
            };
        }
        Err(_) => {}
    };

    let retail_path = "D:\\Steam\\steamapps\\common\\HITMAN 3\\retail";
    let thumbs_path = format!("{retail_path}\\thumbs.dat");

    let mut thumbs = IniFile::new();
    thumbs.load(thumbs_path.as_str())?;
    std::println!("start reading thumbs {thumbs_path}");

    if let (Ok(proj_path), Ok(relative_runtime_path)) = (thumbs.get_value("application", "PROJECT_PATH"), thumbs.get_value("application", "RUNTIME_PATH")) {

        let runtime_path = format!("{retail_path}\\{proj_path}\\{relative_runtime_path}");
        std::println!("start reading package definitions {runtime_path}");
        let mut package_manager = PackageManager::new(&runtime_path);
        println!("{}", serde_json::to_string_pretty(&package_manager.partition_infos).unwrap());

        let mut resource_container : ResourceContainer = ResourceContainer::default();
        package_manager.initialize(&mut resource_container)?;

        println!("{}", resource_container);
        // println!();
        // let mut resources = vec![];
        // let mut partition_manager = PartitionManager::default();
        // partition_manager.parse_into(&package_definitions, runtime_path.as_str(), &mut resources);
        // print_resource_journey(0x00EE6B9C45CC038F, &partition_manager, &resources);
    } else {
        return Err(anyhow!("Missing required properties inside thumbs.dat: \n\
        PROJECT_PATH: {},\n\
        RUNTIME_PATH: {}", thumbs.get_value("application", "PROJECT_PATH").is_ok(), thumbs.get_value("application","RUNTIME_PATH").is_ok()));
    }
    std::println!("done in {} ms", now.elapsed().as_millis());

    Ok(())
}