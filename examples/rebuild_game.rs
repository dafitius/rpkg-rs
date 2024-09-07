use md5::{Digest, Md5};
use rpkg_rs::resource::package_builder::PackageBuilder;
use rpkg_rs::resource::partition_manager::PartitionManager;
use rpkg_rs::resource::resource_package::ResourcePackageSource;
use rpkg_rs::WoaVersion;
use std::path::PathBuf;
use std::{env, fs, io};

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
    let package_manager = PartitionManager::from_game(retail_path, game_version, true)
        .unwrap_or_else(|e| {
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
                eprintln!(
                    "failed to create package builder for package '{}': {}",
                    output_name, e
                );
                std::process::exit(0);
            });

            builder.with_patch_id(patch_id);

            if package.has_legacy_references() {
                builder.use_legacy_references();
            }

            builder
                .build(package.version(), output_path.join(&output_name).as_path())
                .unwrap_or_else(|e| {
                    eprintln!("failed to build package '{}': {}", output_name, e);
                    std::process::exit(0);
                });

            // After it's built, check if the generated file is the same as the original.
            let original_file = match &package.source() {
                Some(ResourcePackageSource::File(path)) => path,
                _ => panic!(
                    "Package '{}' of game '{:?}' has no source",
                    output_name, game_version
                ),
            };

            let generated_file = output_path.join(&output_name);

            if original_file.metadata().unwrap().len() != generated_file.metadata().unwrap().len() {
                panic!(
                    "File size mismatch for package '{}' of game '{:?}'",
                    output_name, game_version
                );
            }

            // Hash the files and compare them.
            let original_hash = {
                let mut file = fs::File::open(original_file).unwrap();
                let mut hasher = Md5::new();
                io::copy(&mut file, &mut hasher).unwrap();
                hasher.finalize()
            };

            let generated_hash = {
                let mut file = fs::File::open(&generated_file).unwrap();
                let mut hasher = Md5::new();
                io::copy(&mut file, &mut hasher).unwrap();
                hasher.finalize()
            };

            if original_hash != generated_hash {
                panic!(
                    "Hash mismatch for package '{}' of game '{:?}'",
                    output_name, game_version
                );
            }
        }
    }

    println!("Done!");
}
