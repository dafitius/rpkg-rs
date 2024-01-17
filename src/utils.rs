use std::fs;
use std::fs::File;
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use anyhow::Error;

pub fn get_file_as_byte_vec(filename: &Path) -> Result<Vec<u8>, Error> {
    if let Ok(mut f) = File::open(filename) {
        if let Ok(metadata) = fs::metadata(filename){
            let mut buffer = vec![0; metadata.len() as usize];
            if f.read(&mut buffer).is_err(){
                return Err(anyhow::anyhow!("buffer overflow"));
            }
            Ok(buffer)
        }
        else {
            Err(anyhow::anyhow!("unable to read metadata"))
        }
    }
    else {
        Err(anyhow::anyhow!("{:?} does not exist", filename))
    }
    
}

pub fn normalize_path(path: &Path) -> PathBuf {
    let mut components = path.components().peekable();
    let mut ret = if let Some(c @ Component::Prefix(..)) = components.peek().cloned() {
        components.next();
        PathBuf::from(c.as_os_str())
    } else {
        PathBuf::new()
    };

    for component in components {
        match component {
            Component::Prefix(..) => unreachable!(),
            Component::RootDir => {
                ret.push(component.as_os_str());
            }
            Component::CurDir => {}
            Component::ParentDir => {
                ret.pop();
            }
            Component::Normal(c) => {
                ret.push(c);
            }
        }
    }
    ret
}