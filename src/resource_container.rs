use std::collections::HashMap;
use crate::resource::ResourceInfo;
use crate::resource_index::ResourceIndex;
use crate::runtime_resource_id::RuntimeResourceID;

pub struct ResourceContainer{
    resources: Vec<ResourceInfo>,
    old_versions: Vec<ResourceIndex>,
    indices: HashMap<RuntimeResourceID, ResourceIndex>,

}