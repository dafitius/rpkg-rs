use std::str::FromStr;
use rpkg_rs::misc::resource_id::ResourceID;
use rpkg_rs::resource::package_builder::{PackageBuilder, PackageResourceBuilder};
use rpkg_rs::resource::resource_package::{ChunkType, PackageVersion, ResourcePackage, ResourceReferenceFlags, ResourceReferenceFlagsV2};
use rpkg_rs::resource::runtime_resource_id::RuntimeResourceID;

#[test]
fn test_build_simple_package_v2() -> Result<(), Box<dyn std::error::Error>> {
    let resource_ids = vec![
        ResourceID::from_str("[assembly:/res1.brick].pc_entitytype")?,
        ResourceID::from_str("[assembly:/res2.brick].pc_entitytype")?,
    ];

    // Start building the package.
    let mut builder = PackageBuilder::new(69, ChunkType::Standard);

    for (i, rid) in resource_ids.iter().enumerate() {
        // Create a fake resource id and data for the resource.
        let rrid: RuntimeResourceID = RuntimeResourceID::from_resource_id(&rid);
        let fake_data: Vec<u8> = (0..1024).map(|j| (i * j) as u8).collect();

        // Create a resource from memory and add it to the package.
        let mut resource = PackageResourceBuilder::from_memory(rrid, "TEMP", fake_data, None, false)?;

        // Add references to the resource.
        if i == 0 {
            resource.with_reference(RuntimeResourceID::from(0x00123456789ABCDE), ResourceReferenceFlags::V2(ResourceReferenceFlagsV2::new().with_language_code(0x1F)));
        } else {
            resource.with_reference(RuntimeResourceID::from(0x0069696969696969), ResourceReferenceFlags::V2(ResourceReferenceFlagsV2::new().with_language_code(0x06)));
            resource.with_reference(RuntimeResourceID::from(0x0042042042042042), ResourceReferenceFlags::V2(ResourceReferenceFlagsV2::new().with_language_code(0x09).with_runtime_acquired(true)));
        }

        builder.with_resource(resource);
    }

    // Build the package in memory.
    let package_data = builder.build_in_memory(PackageVersion::RPKGv2, false, false)?;

    // Now let's try to parse it again.
    let package = ResourcePackage::from_memory(package_data, false)?;

    // And check that we found the resources we expected.
    assert_eq!(package.header.file_count, 2);
    assert_eq!(package.resources.len(), 2);

    let first_resource = package.resources.get(&RuntimeResourceID::from_resource_id(&resource_ids[0])).unwrap();
    let second_resource = package.resources.get(&RuntimeResourceID::from_resource_id(&resource_ids[1])).unwrap();

    // And check that we found the correct references in them and that they are in the expected order.
    assert_eq!(first_resource.references().len(), 1);
    assert_eq!(second_resource.references().len(), 2);

    assert_eq!(first_resource.references()[0].1.language_code(), 0x1F);
    assert_eq!(first_resource.references()[0].1.is_acquired(), false);

    let second_resource_ref_1 = second_resource.references()[0];
    assert_eq!(second_resource_ref_1.1.language_code(), 0x06);
    assert_eq!(second_resource_ref_1.1.is_acquired(), false);

    let second_resource_ref_2 = second_resource.references()[1];
    assert_eq!(second_resource_ref_2.1.language_code(), 0x09);
    assert_eq!(second_resource_ref_2.1.is_acquired(), true);

    Ok(())
}

fn test_package_with_resource(compression_level: Option<u32>, should_scramble: bool) -> Result<(), Box<dyn std::error::Error>> {
    let resource_id = ResourceID::from_str("[assembly:/res1.brick].pc_entitytype")?;

    // Start building the package.
    let mut builder = PackageBuilder::new(69, ChunkType::Standard);

    // Create a fake resource id and data for the resource.
    let rrid: RuntimeResourceID = RuntimeResourceID::from_resource_id(&resource_id);
    let fake_data: Vec<u8> = (0..1024).map(|j| (j as u8)).collect();

    // Create a resource from memory and add it to the package.
    let resource = PackageResourceBuilder::from_memory(rrid, "TEMP", fake_data.clone(), compression_level, should_scramble)?;

    builder.with_resource(resource);

    // Build the package in memory.
    let package_data = builder.build_in_memory(PackageVersion::RPKGv2, false, false)?;

    // Print as hex.
    for byte in &package_data {
        print!("{:02X}", *byte);
    }
    println!("");

    // Now let's try to parse it again.
    let package = ResourcePackage::from_memory(package_data, false)?;

    // And check that its data matches the original.
    let resource_data = package.read_resource(&rrid).unwrap();
    assert_eq!(resource_data, fake_data);

    Ok(())
}

#[test]
fn test_compression() -> Result<(), Box<dyn std::error::Error>> {
    test_package_with_resource(Some(4), false)
}

#[test]
fn test_scrambling() -> Result<(), Box<dyn std::error::Error>> {
    test_package_with_resource(None, true)
}

#[test]
fn test_compression_and_scrambling() -> Result<(), Box<dyn std::error::Error>> {
    test_package_with_resource(Some(4), true)
}
