use rpkg_rs::misc::resource_id::ResourceID;
use rpkg_rs::resource::package_builder::{PackageBuilder, PackageResourceBuilder};
use rpkg_rs::resource::resource_package::{
    ChunkType, PackageVersion, ResourcePackage, ResourceReferenceFlags, ResourceReferenceFlagsV1,
    ResourceReferenceFlagsV2,
};
use rpkg_rs::resource::runtime_resource_id::RuntimeResourceID;
use std::str::FromStr;

fn test_package_with_resource(
    compression_level: Option<i32>,
    should_scramble: bool,
    version: PackageVersion,
    is_patch: bool,
    legacy_references: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let resource_id = ResourceID::from_str("[assembly:/res1.brick].pc_entitytype")?;

    let unneeded_resource_ids = vec![
        RuntimeResourceID::from_resource_id(&ResourceID::from_str(
            "[assembly:/res2.brick].pc_entitytype",
        )?),
        RuntimeResourceID::from_resource_id(&ResourceID::from_str(
            "[assembly:/res3.brick].pc_entitytype",
        )?),
    ];

    let references = vec![
        RuntimeResourceID::from_resource_id(&ResourceID::from_str(
            "[assembly:/ref1.brick].pc_entitytype",
        )?),
        RuntimeResourceID::from_resource_id(&ResourceID::from_str(
            "[assembly:/ref2.brick].pc_entitytype",
        )?),
    ];

    let resource_reference_flags = if legacy_references {
        ResourceReferenceFlags::V1(
            ResourceReferenceFlagsV1::new()
                .with_runtime_acquired(true)
                .with_install_dependency(true),
        )
    } else {
        ResourceReferenceFlags::V2(ResourceReferenceFlagsV2::new().with_language_code(0x1F))
    };

    // Start building the package.
    let mut builder = PackageBuilder::new(69, ChunkType::Standard);

    // Create a fake resource id and data for the resource.
    let rrid: RuntimeResourceID = RuntimeResourceID::from_resource_id(&resource_id);
    let fake_data: Vec<u8> = (0..1024).map(|j| j as u8).collect();

    // Create a resource from memory and add it to the package.
    let mut resource = PackageResourceBuilder::from_memory(
        rrid,
        "TEMP",
        fake_data.clone(),
        compression_level,
        should_scramble,
    )?;

    for reference in &references {
        resource.with_reference(*reference, resource_reference_flags.clone());
    }

    builder.with_resource(resource);

    if is_patch {
        builder.with_patch_id(1);

        // Add some unneeded resources.
        for rrid in &unneeded_resource_ids {
            builder.with_unneeded_resource(*rrid);
        }
    }

    // Build the package in memory.
    let package_data = builder.build_in_memory(version, is_patch, legacy_references)?;

    // Now let's try to parse it again.
    let package = ResourcePackage::from_memory(package_data, is_patch)?;

    // Check that its data matches the original.
    let resource_data = package.read_resource(&rrid).unwrap();
    assert_eq!(resource_data, fake_data, "Resource data doesn't match");

    // Check that the references are correct and in the right order.
    let resource_info = package.resources.get(&rrid).unwrap();

    for (i, (rrid, flags)) in resource_info.references().iter().enumerate() {
        let reference = references[i];
        assert_eq!(*rrid, reference, "Reference at index {} doesn't match", i);
        assert_eq!(
            *flags, resource_reference_flags,
            "Reference flags at index {} don't match",
            i
        );
    }

    // Check that the unneeded resources are correct.
    if is_patch {
        let parsed_unneeded_resource_ids = package.unneeded_resource_ids();

        assert_eq!(
            parsed_unneeded_resource_ids.len(),
            unneeded_resource_ids.len(),
            "Number of unneeded resources doesn't match"
        );

        for (i, rrid) in parsed_unneeded_resource_ids.iter().enumerate() {
            let expected_rrid = unneeded_resource_ids[i];
            assert_eq!(
                **rrid, expected_rrid,
                "Unneeded resource at index {} doesn't match",
                i
            );
        }
    }

    Ok(())
}

#[test]
fn test_simple_rpkg_v1() -> Result<(), Box<dyn std::error::Error>> {
    test_package_with_resource(None, false, PackageVersion::RPKGv1, false, false)
}

#[test]
fn test_compression_rpkg_v1() -> Result<(), Box<dyn std::error::Error>> {
    test_package_with_resource(Some(4), false, PackageVersion::RPKGv1, false, false)
}

#[test]
fn test_scrambling_rpkg_v1() -> Result<(), Box<dyn std::error::Error>> {
    test_package_with_resource(None, true, PackageVersion::RPKGv1, false, false)
}

#[test]
fn test_compression_and_scrambling_rpkg_v1() -> Result<(), Box<dyn std::error::Error>> {
    test_package_with_resource(Some(4), true, PackageVersion::RPKGv1, false, false)
}

#[test]
fn test_simple_rpkg_v2() -> Result<(), Box<dyn std::error::Error>> {
    test_package_with_resource(None, false, PackageVersion::RPKGv2, false, false)
}

#[test]
fn test_compression_rpkg_v2() -> Result<(), Box<dyn std::error::Error>> {
    test_package_with_resource(Some(4), false, PackageVersion::RPKGv2, false, false)
}

#[test]
fn test_scrambling_rpkg_v2() -> Result<(), Box<dyn std::error::Error>> {
    test_package_with_resource(None, true, PackageVersion::RPKGv2, false, false)
}

#[test]
fn test_compression_and_scrambling_rpkg_v2() -> Result<(), Box<dyn std::error::Error>> {
    test_package_with_resource(Some(4), true, PackageVersion::RPKGv2, false, false)
}

#[test]
fn test_patch_rpkg_v1() -> Result<(), Box<dyn std::error::Error>> {
    test_package_with_resource(None, false, PackageVersion::RPKGv1, true, false)
}

#[test]
fn test_compressed_and_scrambled_patch_rpkg_v1() -> Result<(), Box<dyn std::error::Error>> {
    test_package_with_resource(Some(4), true, PackageVersion::RPKGv1, true, false)
}

#[test]
fn test_legacy_rpkg_v1() -> Result<(), Box<dyn std::error::Error>> {
    test_package_with_resource(None, false, PackageVersion::RPKGv1, false, true)
}

#[test]
fn test_legacy_patch_rpkg_v1() -> Result<(), Box<dyn std::error::Error>> {
    test_package_with_resource(None, false, PackageVersion::RPKGv1, true, true)
}

#[test]
fn test_legacy_compressed_and_scrambled_patch_rpkg_v1() -> Result<(), Box<dyn std::error::Error>> {
    test_package_with_resource(Some(4), true, PackageVersion::RPKGv1, true, true)
}

#[test]
fn test_legacy_rpkg_v2() -> Result<(), Box<dyn std::error::Error>> {
    test_package_with_resource(None, false, PackageVersion::RPKGv2, false, true)
}

#[test]
fn test_legacy_patch_rpkg_v2() -> Result<(), Box<dyn std::error::Error>> {
    test_package_with_resource(None, false, PackageVersion::RPKGv2, true, true)
}

#[test]
fn test_legacy_compressed_and_scrambled_patch_rpkg_v2() -> Result<(), Box<dyn std::error::Error>> {
    test_package_with_resource(Some(4), true, PackageVersion::RPKGv2, true, true)
}
