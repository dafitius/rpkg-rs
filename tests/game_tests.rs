use std::{fs, io};
use std::path::PathBuf;
use md5::{Digest, Md5};
use rpkg_rs::resource::partition_manager::PartitionManager;
use rpkg_rs::WoaVersion;
use rpkg_rs::resource::package_builder::PackageBuilder;
use rpkg_rs::resource::resource_package::ResourcePackageSource;
use rpkg_rs::resource::resource_partition::PatchId;

fn test_game_mounting(path_env_var: &str, game_version: WoaVersion) -> Result<(), Box<dyn std::error::Error>> {
    // Read game path from env variable
    let game_retail_path = match std::env::var(path_env_var) {
        Ok(path) => PathBuf::from(path),
        Err(_) => return Err(format!("{} environment variable not set", path_env_var).into()),
    };

    let package_manager = PartitionManager::mount_game(
        game_retail_path,
        game_version,
        true,
        |_, _| {},
    )?;

    assert!(package_manager.partitions.len() > 0);

    let packages = package_manager.partitions.iter().map(|p| p.packages.len()).sum::<usize>();
    assert!(packages > 0);
    
    Ok(())
}

#[test]
#[ignore]
fn test_hm2016_mounting() -> Result<(), Box<dyn std::error::Error>> {
    test_game_mounting("HM2016_PATH", WoaVersion::HM2016)
}

#[test]
#[ignore]
fn test_hm2_mounting() -> Result<(), Box<dyn std::error::Error>> {
    test_game_mounting("HM2_PATH", WoaVersion::HM2)
}

#[test]
#[ignore]
fn test_hm3_mounting() -> Result<(), Box<dyn std::error::Error>> {
    test_game_mounting("HM3_PATH", WoaVersion::HM3)
}

fn test_game_rebuild(path_env_var: &str, game_version: WoaVersion) -> Result<(), Box<dyn std::error::Error>> {
    // Read game path from env variable
    let game_retail_path = match std::env::var(path_env_var) {
        Ok(path) => PathBuf::from(path),
        Err(_) => return Err(format!("{} environment variable not set", path_env_var).into()),
    };

    // Create a temporary directory to store the output. This is automatically removed when it goes out of scope.
    let temp_dir = tempfile::tempdir()?;
    let output_path = temp_dir.path();
    
    println!("Game path: {:?}", game_retail_path);
    println!("Output path: {:?}", output_path);

    // Mount the game.
    let package_manager = PartitionManager::mount_game(
        game_retail_path,
        game_version,
        true,
        |_, _| {},
    )?;

    // Rebuild each package one by one.
    for partition in package_manager.partitions {
        for (patch_id, package) in &partition.packages {
            let output_name = partition.partition_info().filename(*patch_id);

            println!("Rebuilding package '{}' of game '{:?}'", output_name, game_version);

            // Create a package builder to duplicate the package.
            let mut builder = PackageBuilder::from_resource_package(&package)?;

            // Set the patch ID if it's a patch package.
            let is_patch = match patch_id {
                PatchId::Patch(id) => {
                    builder.with_patch_id(*id as u8);
                    true
                },
                _ => false,
            };

            // And now build it.
            builder.build(
                package.version(),
                output_path.join(&output_name).as_path(),
                is_patch,
                package.has_legacy_references(),
            )?;

            // After it's built, check if the generated file is the same as the original.
            let original_file = match &package.source {
                Some(ResourcePackageSource::File(path)) => path,
                _ => Err(format!("Package '{}' of game '{:?}' has no source", output_name, game_version))?,
            };
            
            let generated_file = output_path.join(&output_name);

            if original_file.metadata()?.len() != generated_file.metadata()?.len() {
                return Err(format!("File size mismatch for package '{}' of game '{:?}'", output_name, game_version).into());
            }

            // Hash the files and compare them.
            let original_hash = {
                let mut file = fs::File::open(original_file)?;
                let mut hasher = Md5::new();
                io::copy(&mut file, &mut hasher)?;
                hasher.finalize()
            };

            let generated_hash = {
                let mut file = fs::File::open(&generated_file)?;
                let mut hasher = Md5::new();
                io::copy(&mut file, &mut hasher)?;
                hasher.finalize()
            };
            
            if original_hash != generated_hash {
                return Err(format!("Hash mismatch for package '{}' of game '{:?}'", output_name, game_version).into());
            }
            
            // Remove the generated file.
            fs::remove_file(generated_file)?;
        }
    }

    Ok(())
}

#[test]
#[ignore] 
fn test_hm2016_rebuild() -> Result<(), Box<dyn std::error::Error>> {
    test_game_rebuild("HM2016_PATH", WoaVersion::HM2016)
}

#[test]
#[ignore] 
fn test_hm2_rebuild() -> Result<(), Box<dyn std::error::Error>> {
    test_game_rebuild("HM2_PATH", WoaVersion::HM2)
}

#[test]
#[ignore]
fn test_hm3_rebuild() -> Result<(), Box<dyn std::error::Error>> {
    test_game_rebuild("HM3_PATH", WoaVersion::HM3)
}