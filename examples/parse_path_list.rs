use std::{env, io};
use std::io::{stdin, Write};
use std::path::Path;
use rpkg_rs::misc::hash_path_list::PathList;

fn main() {

    let args: Vec<String> = env::args().collect();

    if args.len() < 2{
        eprintln!("Usage: cargo run --example <example_name> -- <path to a hashlist>");
        return;
    }

    let hash_list_path = Path::new(&args[1]);

    let mut path_list = PathList::new();

    loop {
        print!("enter a folder > ");
        io::stdout().flush().unwrap();

        let mut input_string = String::new();
        stdin().read_line(&mut input_string)
            .ok()
            .expect("Failed to read line");


        if let Ok(_) = path_list.parse_into(hash_list_path,true){
            println!("files: {:?}", path_list.get_files(input_string.as_str()));
            println!("folders: {:?}", path_list.get_folders(input_string.as_str()));
        }
    }


}