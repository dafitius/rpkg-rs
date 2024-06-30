use std::path::PathBuf;
use rpkg_rs::resource::partition_manager::PartitionManager;
use rpkg_rs::WoaVersion;

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