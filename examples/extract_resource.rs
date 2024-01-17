use std::fs::File;
use std::io;
use std::io::{Cursor, Read, Seek};
use std::path::PathBuf;
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
    let rrid : RuntimeResourceID = RuntimeResourceID::from_resource_id(&rid);

    //parse the ResourcePackage
    match File::open(&package_path){
        Ok(file) => {
            let mmap = unsafe { Mmap::map(&file).unwrap() };
            let mut reader = Cursor::new(&mmap[..]);
            let is_patch = package_path.clone().file_name().unwrap().to_str().unwrap().contains("patch");
            let rpkg: ResourcePackage = reader.read_ne_args((is_patch,)).unwrap();

            //try to find the resource inside the package
            if let Some((resource_header, resource_offset_info)) = rpkg
                .resource_entries
                .iter()
                .enumerate()
                .find(|(_, entry)| entry.runtime_resource_id == rrid)
                .map(|(index, entry)| (rpkg.resource_metadata.get(index).unwrap(), entry))
            {
                println!("I want to extract: {}", resource_header);
                println!("Let's extract that from: {}", resource_offset_info);

                let final_size = resource_offset_info.compressed_size_and_is_scrambled_flag & 0x3FFFFFFF;
                let is_lz4ed = final_size != resource_header.data_size;
                let is_scrambled = resource_offset_info.compressed_size_and_is_scrambled_flag & 0x80000000 == 0x80000000;

                // Extract the resource bytes from the resourcePackage
                match File::open(&package_path){
                    Ok(mut file) =>{
                        file.seek(io::SeekFrom::Start(resource_offset_info.data_offset)).unwrap();
                        let mut buffer = vec![0; final_size as usize];
                        file.read_exact(&mut buffer).unwrap();

                        if is_scrambled {
                            let str_xor = vec![0xdc, 0x45, 0xa6, 0x9c, 0xd3, 0x72, 0x4c, 0xab];
                            buffer = buffer
                                .iter()
                                .enumerate()
                                .map(|(index, byte)| byte ^ str_xor[index % str_xor.len()])
                                .collect();
                        }

                        let mut file = vec![0; resource_header.data_size as usize];

                        if is_lz4ed {
                            let size = decompress_to_buffer(&*buffer, Some(resource_header.data_size as i32), &mut *file).expect("TODO: panic message");
                            if size == resource_header.data_size as usize {
                                println!("{:?}", std::str::from_utf8(&*file));
                            }
                        }
                        else{
                            println!("{:?}", buffer.iter());
                        }
                    },
                    Err(e) =>{
                        eprintln!("There was an error opening the file: {}", e);
                    }
                };

            } else {
                eprintln!("Couldn't find the requested resource inside of the given resource package");
            }
        },
        Err(e) =>{
            eprintln!("There was an error opening the file: {}", e);
        }
    }
}