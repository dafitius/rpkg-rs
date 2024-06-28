use std::str::FromStr;
use rpkg_rs::misc::resource_id::ResourceID;
use rpkg_rs::resource::package_builder::{PackageBuilder, PackageResourceBuilder, PackageVersion};
use rpkg_rs::resource::resource_package::{ChunkType, ResourcePackage, ResourceReferenceFlagsV2};
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
        let mut resource = PackageResourceBuilder::from_memory(rrid, "TEMP", fake_data)?;

        // Add references to the resource.
        if i == 0 {
            resource.with_reference(RuntimeResourceID::from(0x00123456789ABCDE), ResourceReferenceFlagsV2::new().with_language_code(0x1F));
        } else {
            resource.with_reference(RuntimeResourceID::from(0x0069696969696969), ResourceReferenceFlagsV2::new().with_language_code(0x06));
            resource.with_reference(RuntimeResourceID::from(0x0042042042042042), ResourceReferenceFlagsV2::new().with_language_code(0x09).with_runtime_acquired(true));
        }

        builder.with_resource(resource);
    }

    // Build the package in memory.
    let package_data = builder.build_in_memory(PackageVersion::RPKGv2, false)?;

    // Now let's try to parse it again.
    let package = ResourcePackage::from_memory(package_data, false)?;

    // And check that we found the resources we expected.
    assert_eq!(package.header.file_count, 2);
    assert_eq!(package.resources.len(), 2);

    let first_resource = package.resources.get(&RuntimeResourceID::from_resource_id(&resource_ids[0])).unwrap();
    let second_resource = package.resources.get(&RuntimeResourceID::from_resource_id(&resource_ids[1])).unwrap();

    assert_eq!(first_resource.references().len(), 1);
    assert_eq!(second_resource.references().len(), 2);

    assert_eq!(first_resource.references().first().unwrap().1.language_code(), 0x1F);
    assert_eq!(first_resource.references().first().unwrap().1.is_acquired(), false);

    let second_resource_ref_1 = second_resource.references().iter().find(|(rid, _)| rid == &RuntimeResourceID::from(0x0069696969696969)).unwrap();
    assert_eq!(second_resource_ref_1.1.language_code(), 0x06);
    assert_eq!(second_resource_ref_1.1.is_acquired(), false);

    let second_resource_ref_2 = second_resource.references().iter().find(|(rid, _)| rid == &RuntimeResourceID::from(0x0042042042042042)).unwrap();
    assert_eq!(second_resource_ref_2.1.language_code(), 0x09);
    assert_eq!(second_resource_ref_2.1.is_acquired(), true);

    Ok(())
}