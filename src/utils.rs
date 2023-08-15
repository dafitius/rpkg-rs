use std::{fs, io};
use std::fs::File;
use std::io::{BufRead, Read};
use std::path::Path;
use anyhow::Error;

pub fn get_file_as_byte_vec(filename: &str) -> Result<Vec<u8>, Error> {
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
        Err(anyhow::anyhow!("no file found"))
    }
}

pub fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
    where P: AsRef<Path>, {
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}