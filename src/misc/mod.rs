#[deprecated(
    since = "1.2.0",
    note = "Replaced by dedicated glacier-ini crate")]
pub mod ini_file_system;
pub mod resource_id;

#[cfg(feature = "path-list")]
pub mod hash_path_list;
