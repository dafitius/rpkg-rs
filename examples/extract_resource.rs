use std::fs::File;
use std::io;
use std::io::{Cursor, Read, Seek};
use std::path::PathBuf;
use std::str::Utf8Error;
use binrw::BinReaderExt;
use lz4::block::decompress_to_buffer;
use memmap2::Mmap;
use rpkg_rs::misc::resource_id::ResourceID;
use rpkg_rs::runtime::resource::resource_package::{ResourcePackage};
use rpkg_rs::runtime::resource::runtime_resource_id::RuntimeResourceID;

fn main() {

    //set the args
    let package_path = PathBuf::from("S:/Steam/steamapps/common/Hitman™/Runtime/chunk0.rpkg");
    //let package_path = PathBuf::from("D:/Steam/steamapps/common/HITMAN 3/Runtime/chunk0.rpkg");
    let rid = ResourceID::from_string("[assembly:/repository/pro.repo].pc_repo");
    let rrid: RuntimeResourceID = RuntimeResourceID::from_resource_id(&rid);

    //parse the ResourcePackage
    match File::open(&package_path) {
        Ok(file) => {
            let mmap = unsafe { Mmap::map(&file).unwrap() };
            let mut reader = Cursor::new(&mmap[..]);
            let is_patch = package_path.clone().file_name().unwrap().to_str().unwrap().contains("patch");
            let rpkg: ResourcePackage = reader.read_ne_args((is_patch, )).unwrap_or_else(|e| {
                println!("Failed to parse package: {}", e);
                std::process::exit(1)
            }
            );

            let file = rpkg.get_resource(&package_path, &rrid).unwrap_or_else(|e| {
                println!("Failed extract resource: {}", e);
                std::process::exit(1)
            });

            match std::str::from_utf8(&*file) {
                Ok(s) => {
                    println!("{}...", s.chars().take(100).collect::<String>())
                }
                Err(e) => {
                    println!("first bytes: {:?}", file.iter().take(50).collect::<Vec<_>>());
                }
            };
        }
        Err(e) => {
            eprintln!("There was an error opening the file: {}", e);
        }
    }
}