use std::path::Path;
use rpkg_rs::misc::hash_path_list::PathList;

fn main() {
    let hash_list_path = Path::new("D:\\David\\Hitman-modding\\Tools\\rpkgTools\\2.25\\hash_list.txt");

    let mut path_list = PathList::new();

    if let Ok(_) = path_list.parse_into(hash_list_path,true){
        println!("{:?}", path_list.get_files("assembly:/_pro/scenes/bricks"));
        println!("{:?}", path_list.get_folders("assembly:/_pro/scenes/bricks"));
    }
}