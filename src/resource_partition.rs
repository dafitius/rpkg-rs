use std::fs;
use std::fs::File;
use std::io::Cursor;
use std::iter::zip;
use std::path::Path;
use std::collections::HashMap;
use memmap2::Mmap;
use anyhow::{anyhow, Error};
use binrw::BinReaderExt;
use regex::Regex;
use crate::resource::ResourceInfo;
use crate::resource_package::ResourcePackage;
use crate::runtime_resource_id::RuntimeResourceID;


pub struct ResourcePartition {
    pub package_dir: String,
    pub index: u16,
    pub resource_bounds: (usize, usize),
    pub package_bounds: Vec<usize>,
    pub patch_indices: Vec<u16>,
    pub resource_indices: Vec<isize>,
}

impl ResourcePartition {
    pub fn new(runtime_path: String, package_index: u16) -> Self {
        Self {
            package_dir: runtime_path,
            index: package_index,
            patch_indices: vec![],
            resource_bounds: (0, 0),
            package_bounds: vec![],
            resource_indices: vec![],
        }
    }

    pub fn read_patch_indices(&mut self) -> Result<&Self, Error> {
        if !Path::new(format!("{}\\chunk{}.rpkg", self.package_dir, self.index).as_str()).exists() {
            return Err(anyhow!("The base package was not found, stopped attempting to find patches"));
        }

        let regex_str = format!("{}\\chunk{}patch([0-9]+).rpkg", self.package_dir, self.index).as_str().replace('\\', "\\\\");
        let patch_package_re = Regex::new(regex_str.as_str()).unwrap();
        for path_buf in fs::read_dir(self.package_dir.as_str())?.into_iter()
            .filter(|r| r.is_ok())
            .map(|r| r.unwrap().path())
            .filter(|r| r.is_file())
        {
            let path = path_buf.as_path().to_str().unwrap();
            if patch_package_re.is_match(path) {
                let cap = patch_package_re.captures(path).unwrap();
                self.patch_indices.push(cap[1].parse::<u16>()?);
            }
        }
        self.patch_indices.sort();
        Ok(self)
    }

    pub fn read_partition(&mut self, resources: &mut Vec<ResourceInfo>) {
        self.resource_bounds.0 = resources.len();

        //used for fast runtimeResourceID lookup when initializing.
        let mut resource_cache: HashMap<RuntimeResourceID, u64> = HashMap::new();


        let base_package_path = format!("{}\\chunk{}.rpkg", self.package_dir, self.index);
        ResourcePartition::read_package(base_package_path.as_str(), &mut self.resource_indices, resources, &mut resource_cache, false);
        self.package_bounds.push(resources.len());

        for patch_index in self.patch_indices.iter() {
            let patch_package_path = format!("{}\\chunk{}patch{}.rpkg", self.package_dir, self.index, patch_index);
            ResourcePartition::read_package(patch_package_path.as_str(), &mut self.resource_indices, resources, &mut resource_cache, true);
            self.package_bounds.push(resources.len());
        }
        self.resource_indices.retain(|index| {
            let delete = {
                *index == -1
            };
            !delete
        });

        self.resource_bounds.1 = resources.len();

        println!("chunk{} has patch levels: {:?} and Resource index bounds: {:?}", self.index, self.patch_indices, self.resource_bounds);
        println!("rpkg file contains {} Resources", self.resource_indices.len());
    }

    fn read_package(path: &str, resource_indices: &mut Vec<isize>, resources: &mut Vec<ResourceInfo>, cache: &mut HashMap<RuntimeResourceID, u64>, is_patch: bool) {
        //let resource_start: usize = resources.len();

        let file = File::open(path).expect("failed to open the given file");
        let mmap = unsafe { Mmap::map(&file).expect("failed to map the given file") };


        std::println!("Start reading {path}");

        let mut reader = Cursor::new(&mmap[..]);
        let rpkg: ResourcePackage = reader.read_ne_args((is_patch, )).unwrap();

        //remove the deletions if there are any
        if let Some(deletions) = rpkg.deletion_list {
            for deletion in deletions.iter() {
                //delete the deletion from the resource_indices array
                if let Some(index) = cache.get(deletion) {
                    if let Some(idx) = resource_indices.get_mut(*index as usize) {
                        *idx = -1;
                    }
                    cache.remove(deletion);
                }
            }
        }

        for (entry, header) in zip(rpkg.resource_entries, rpkg.resource_metadata) {

            // Determine hash's size and if it is LZ4ed and/or XORed
            let mut xored = false;
            let mut lz4ed = false;
            let mut file_size;
            if header.data_size & 0x3FFFFFFF != 0
            {
                lz4ed = true;
                file_size = header.data_size;

                if header.data_size & 0x80000000 == 0x80000000
                {
                    file_size &= 0x3FFFFFFF;
                    xored = true;
                }
            } else {
                file_size = entry.compressed_size_and_is_scrambled_flag;

                if header.data_size & 0x80000000 == 0x80000000 {
                    xored = true;
                }
            }

            let mut last_index: Option<usize> = None;
            let mut patch_resource: bool = false;

            if is_patch {
                if let Some(index) = cache.get(&entry.runtime_resource_id) {
                    if let Some(r_id) = resource_indices.get(*index as usize) {
                        last_index = Some(*r_id as usize);
                        //resource_indices.remove(*index as usize);
                        resource_indices[*index as usize] = resources.len() as isize;
                        patch_resource = true;
                    }
                };
            }
            cache.insert(entry.runtime_resource_id.clone(), resource_indices.len() as u64);
            resources.push(ResourceInfo { entry, header, size: file_size, is_lz4ed: lz4ed, is_scrambled: xored, last_index });
            if !patch_resource {
                resource_indices.push(resources.len() as isize - 1);
            }
        }
    }

    pub fn get_package_index(&self, resource_index: u64) -> u16 {
        for (i, bound) in self.package_bounds.iter().enumerate() {
            if resource_index < *bound as u64 {
                if i == 0_usize {
                    return i as u16;
                } else {
                    return self.patch_indices[i];
                }
            }
        }
        return *self.patch_indices.last().unwrap();
    }
}