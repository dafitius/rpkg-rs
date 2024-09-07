use std::io::{stdin, Write};
use std::path::PathBuf;
use std::str::FromStr;
use std::{env, io};
use itertools::Itertools;

use rpkg_rs::misc::resource_id::ResourceID;
use rpkg_rs::resource::partition_manager::{PartitionManager, PartitionState};
use rpkg_rs::resource::pdefs::{GamePaths, PackageDefinitionSource};
use rpkg_rs::resource::resource_info::ResourceInfo;
use rpkg_rs::resource::resource_partition::PatchId;
use rpkg_rs::resource::runtime_resource_id::RuntimeResourceID;
use rpkg_rs::WoaVersion;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        eprintln!("Usage: cargo run --example <example_name> -- <path to a retail directory> <game version (H2016 | HM2 | HM3)>");
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

    // Discover the game paths.
    let game_paths = GamePaths::from_retail_directory(retail_path.clone()).unwrap_or_else(|e| {
        eprintln!("failed to discover game paths: {}", e);
        std::process::exit(0);
    });

    // Read and parse the package definition.
    let package_definition_source =
        PackageDefinitionSource::from_file(game_paths.package_definition_path, game_version)
            .unwrap_or_else(|e| {
                eprintln!("failed to parse package definition: {}", e);
                std::process::exit(0);
            });

    let mut partition_infos = package_definition_source.read().unwrap_or_else(|e| {
        eprintln!("failed to read package definition: {}", e);
        std::process::exit(0);
    });

    // Ignore modded patches.
    for partition in partition_infos.iter_mut() {
        partition.set_max_patch_level(9);
    }

    let mut package_manager =
        PartitionManager::new(game_paths.runtime_path, &package_definition_source).unwrap_or_else(
            |e| {
                eprintln!("failed to init package manager: {}", e);
                std::process::exit(0);
            },
        );

        //read the packagedefs here
        let mut last_index = 0;
        let mut progress = 0.0;
        let progress_callback = |current, state: &PartitionState| {
            if current != last_index {
                last_index = current;
                print!("Mounting partition {} ", current);
            }
            if !state.installing && !state.mounted {
                println!("[Failed to mount this partition. Is it installed?]");
            }
            let install_progress = (state.install_progress * 10.0).ceil() / 10.0;

        let chars_to_add = (install_progress * 10.0 - progress * 10.0) as usize * 2;
        let chars_to_add = std::cmp::min(chars_to_add, 20);
        print!("{}", "█".repeat(chars_to_add));
        io::stdout().flush().unwrap();

        progress = install_progress;

        if progress == 1.0 {
            progress = 0.0;

            if state.mounted {
                println!(" done :)");
            } else {
                println!(" failed :(");
            }
        }
    };

    package_manager
        .mount_partitions(progress_callback)
        .unwrap_or_else(|e| {
            eprintln!("failed to mount partitions: {}", e);
            std::process::exit(0);
        });

    loop {
        print!("enter a ResourceID > ");
        io::stdout().flush().unwrap();

        let mut input_string = String::new();
        stdin()
            .read_line(&mut input_string)
            .ok()
            .expect("Failed to read line");

        let rid = ResourceID::from_str(input_string.as_str()).unwrap_or_else(|_| {
            println!("Given ResourceID is invalid");
            std::process::exit(0)
        });

        let rrid = RuntimeResourceID::from_resource_id(&rid);
        println!("Try to find {}", rrid);
        println!("Resource: {rrid}");

        for partition in &package_manager.partitions {
            let mut last_occurence: Option<&ResourceInfo> = None;

            let size =
                |info: &ResourceInfo| info.compressed_size().unwrap_or(info.size());

            let changes = partition.resource_patch_indices(&rrid);
            let deletions = partition.resource_removal_indices(&rrid);
            let occurrences = changes
                .clone()
                .into_iter()
                .chain(deletions.clone().into_iter())
                .collect::<Vec<PatchId>>();

            for occurence in occurrences.iter().sorted() {
                println!(
                    "{}: {}",
                    match occurence {
                        PatchId::Base => {
                            "Base"
                        }
                        PatchId::Patch(_) => {
                            "Patch"
                        }
                    },
                    partition.partition_info().filename(*occurence)
                );

                if deletions.contains(occurence) {
                    println!("\t- Removal: resource deleted");
                    last_occurence = None;
                }

                if changes.contains(occurence) {
                    if let Ok(info) = partition.resource_info_from(&rrid, *occurence) {
                        if let Some(last_info) = last_occurence {
                            println!(
                                "\t- Modification: Size changed from {} to {}",
                                size(last_info),
                                size(info)
                            );
                        } else {
                            println!("\t- Addition: New occurrence, Size {} bytes", size(info))
                        }
                        last_occurence = Some(info);
                    }
                }
            }
        }
    }
}
