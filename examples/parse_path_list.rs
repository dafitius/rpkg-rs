

#[cfg(feature = "path-list")]
use rpkg_rs::misc::hash_path_list::PathList;

use rpkg_rs::runtime::resource::runtime_resource_id::RuntimeResourceID;
use std::io::{stdin, Write};
use std::path::Path;
use std::{env, io};

#[cfg(feature = "path-list")]
fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: cargo run --example <example_name> -- <path to a hashlist>");
        return;
    }

    let hash_list_path = Path::new(&args[1]);

    let mut path_list = PathList::new();

    path_list
        .parse_into(hash_list_path)
        .expect("Failed to parse path list");

    loop {
        print!("enter a runtimeResourceID > ");
        io::stdout().flush().unwrap();

        let mut input_string = String::new();
        stdin()
            .read_line(&mut input_string)
            .ok()
            .expect("Failed to read line");

        if let Ok(rrid) = RuntimeResourceID::from_hex_string(input_string.as_str().trim_end()) {
            println!("{:?}", path_list.get(&rrid));
        } else {
            println!("Failed to interpret the input")
        }
    }
}
