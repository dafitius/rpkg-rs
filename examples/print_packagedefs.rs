use std::path::PathBuf;
use rpkg_rs::misc::ini_file_system::{IniFileSystem};
use rpkg_rs::runtime::resource::package_manager::PackageManager;

fn main() {
    let retail_path = PathBuf::from("S:/Steam/steamapps/common/HITMAN 3/retail");

    let thumbs_path = retail_path.join("thumbs.dat");

    let thumbs = IniFileSystem::from(&thumbs_path.as_path()).unwrap_or_else(|err| {
        eprintln!("Error reading thumbs file: {:?}", err);
        std::process::exit(1);
    });

    std::println!("start reading thumbs {:?}", thumbs_path.as_os_str());

    let app_options = &thumbs.get_root().unwrap_or_else(|| {
        eprintln!("Missing root in thumbs file");
        std::process::exit(1);
    })["application"];

    if let (Some(proj_path), Some(relative_runtime_path)) = (app_options.get_option("PROJECT_PATH"), app_options.get_option("RUNTIME_PATH")) {
        let runtime_path = retail_path.join(proj_path).join(relative_runtime_path);
        let package_manager = PackageManager::new(&runtime_path);
        let pretty_json = serde_json::to_string_pretty(&package_manager.partition_infos).unwrap();
        println!("{}", pretty_json);
    } else {
        eprintln!("Missing required properties inside thumbs.dat:\n\
             PROJECT_PATH: {}\n\
             RUNTIME_PATH: {}",
                  app_options.has_option("PROJECT_PATH"),
                  app_options.has_option("RUNTIME_PATH"));
        return;
    }
}