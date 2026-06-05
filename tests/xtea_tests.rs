use glacier_base::encryption::xtea::{Xtea, XteaConfig};

const TEST_STRING: &str =
    "Lorem ipsum dolor sit amet consectetur adipisicing elit. Maxime mollitia";

const XTEA_TEST_CONFIG: XteaConfig = XteaConfig::Custom {
    key: [0x10020, 0x12031, 0x12391, 0x9134],
    header: [
        0x10, 0x20, 0x20, 0x30, 0x30, 0x30, 0x40, 0x40, 0x40, 0x40, 0x50, 0x50, 0x50, 0x50, 0x50,
        0x60,
    ],
    l10n_key: [0x10020, 0x12031, 0x12391, 0x9134],
};

#[test]
fn test_xtea_text_encoding() -> Result<(), Box<dyn std::error::Error>> {
    let xtea = Xtea::new(XTEA_TEST_CONFIG);
    let encrypted = xtea.encrypt_text_file(TEST_STRING.to_string())?;
    let decrypted = xtea.decrypt_text_file(encrypted.as_slice())?;

    assert_eq!(TEST_STRING, decrypted);
    Ok(())
}

#[test]
fn test_xtea_string_encoding_locr() -> Result<(), Box<dyn std::error::Error>> {
    let xtea = Xtea::new(XTEA_TEST_CONFIG);

    let encrypted = xtea.encrypt_string(TEST_STRING.to_string())?;
    let decrypted = xtea.decrypt_string(encrypted.as_slice())?;

    assert_eq!(TEST_STRING, decrypted);
    Ok(())
}
