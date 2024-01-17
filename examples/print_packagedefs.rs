use std::path::PathBuf;
use anyhow::{anyhow, Error};
use rpkg_rs::misc::ini_file_system::{IniFileSystem};
use rpkg_rs::runtime::resource::package_manager::PackageManager;

fn main() -> Result<(), Error> {
    let retail_path = PathBuf::from("S:/Steam/steamapps/common/HITMAN 3/retail");

    let thumbs_path = retail_path.join("thumbs.dat");

    let thumbs = IniFileSystem::from(&thumbs_path.as_path())?;
    std::println!("start reading thumbs {:?}", thumbs_path.as_os_str());

    let app_options = &thumbs.get_root().unwrap()["application"];

    if let (Some(proj_path), Some(relative_runtime_path)) = (app_options.get_option("PROJECT_PATH"), app_options.get_option("RUNTIME_PATH")) {
        let runtime_path = retail_path.join(proj_path).join(relative_runtime_path);
        let package_manager = PackageManager::new(&runtime_path);
        let pretty_json = serde_json::to_string_pretty(&package_manager.partition_infos)?;
        println!("{}", pretty_json);
    } else {
        return Err(anyhow!(
            "Missing required properties inside thumbs.dat:\n\
             PROJECT_PATH: {}\n\
             RUNTIME_PATH: {}",
            app_options.has_option("PROJECT_PATH"),
            app_options.has_option("RUNTIME_PATH")
        ));
    }

    Ok(())
}