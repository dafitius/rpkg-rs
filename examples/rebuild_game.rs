use std::env;
use std::path::PathBuf;

use rpkg_rs::resource::package_builder::PackageBuilder;
use rpkg_rs::resource::partition_manager::PartitionManager;
use rpkg_rs::resource::resource_partition::PatchId;
use rpkg_rs::WoaVersion;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 4 {
        eprintln!("Usage: cargo run --example <example_name> -- <path to a retail directory> <game version (H2016 | HM2 | HM3)> <output path>");
        return;
    }

    let retail_path = PathBuf::from(&args[1]);

    let game_version = match args[2].as_str() {
        "HM2016" => WoaVersion::HM2016,
        "HM2" => WoaVersion::HM2,
        "HM3" => WoaVersion::HM3,
        e => {
            eprintln!("invalid game version: {}", e);
            std::process::exit(0);
        }
    };

    let output_path = PathBuf::from(&args[3]);
    
    // Create output directory if it doesn't exist.
    if !output_path.is_dir() {
        std::fs::create_dir_all(&output_path).unwrap_or_else(|e| {
            eprintln!("failed to create output directory: {}", e);
            std::process::exit(0);
        });
    }

    // Mount the game.
    println!("Mounting game...");
    let package_manager = PartitionManager::mount_game(
        retail_path,
        game_version,
        true,
        |_, _| {},
    ).unwrap_or_else(|e| {
        eprintln!("failed to mount game: {}", e);
        std::process::exit(0);
    });

    println!("Rebuilding game...");

    // Rebuild each package one by one.
    for partition in package_manager.partitions {
        for (patch_id, package) in &partition.packages {
            let output_name = partition.partition_info().filename(*patch_id);
            println!("Rebuilding package '{}'", output_name);

            let mut builder = PackageBuilder::from_resource_package(&package).unwrap_or_else(|e| {
                eprintln!("failed to create package builder for package '{}': {}", output_name, e);
                std::process::exit(0);
            });

            let is_patch = match patch_id {
                PatchId::Patch(id) => {
                    builder.with_patch_id(*id as u8);
                    true
                },
                _ => false,
            };
            
            if !is_patch {
                continue;
            }

            builder.build(
                package.version(), 
                output_path.join(&output_name).as_path(),
                is_patch,
                package.has_legacy_references(),
            ).unwrap_or_else(|e| {
                eprintln!("failed to build package '{}': {}", output_name, e);
                std::process::exit(0);
            });
        }
    }

    println!("Done!");
}