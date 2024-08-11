use rpkg_rs::encryption::xtea::Xtea;

const TEST_STRING: &str = "Lorem ipsum dolor sit amet consectetur adipisicing elit. Maxime mollitia";

#[test]
fn test_xtea_text_encoding() -> Result<(), Box<dyn std::error::Error>> {
    let encrypted = Xtea::encrypt_text_file(TEST_STRING.to_string())?;
    let decrypted = Xtea::decrypt_text_file(encrypted.as_slice())?;

    assert_eq!(TEST_STRING, decrypted);
    Ok(())
}

#[test]
fn test_xtea_string_encoding_default() -> Result<(), Box<dyn std::error::Error>> {
    let encrypted = Xtea::encrypt_string(TEST_STRING.to_string(), &Xtea::DEFAULT_KEY)?;
    let decrypted = Xtea::decrypt_string(encrypted.as_slice(), &Xtea::DEFAULT_KEY)?;

    assert_eq!(TEST_STRING, decrypted);
    Ok(())
}

#[test]
fn test_xtea_string_encoding_locr() -> Result<(), Box<dyn std::error::Error>> {
    let encrypted = Xtea::encrypt_string(TEST_STRING.to_string(), &Xtea::LOCR_KEY)?;
    let decrypted = Xtea::decrypt_string(encrypted.as_slice(), &Xtea::LOCR_KEY)?;

    assert_eq!(TEST_STRING, decrypted);
    Ok(())
}