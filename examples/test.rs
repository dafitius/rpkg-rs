use rpkg_rs::runtime::resource::runtime_resource_id::RuntimeResourceID;

pub fn main(){
    let resource_id = RuntimeResourceID::from(0x00097AA87B144150);

    let rrid_str = serde_json::to_string(&resource_id);
    println!("{:?}", rrid_str);
}