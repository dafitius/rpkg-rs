use hitman_xtea::{self, CipherError};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum XteaError {
    #[error("Text encoding error: {0}")]
    TextEncodingError(#[from] std::string::FromUtf8Error),

    #[error("An error occurred while trying to decrypt the file: {:?}", _0)]
    DecryptionError(CipherError),

    // Add more error variants as needed
}

//Custom wrapper for the hitman_xtea module
pub struct Xtea;

impl Xtea {
    const DEFAULT_NUMBER_OF_ROUNDS: u32 = 32;
    pub const DEFAULT_KEY: [u32; 4] = [0x30f95282, 0x1f48c419, 0x295f8548, 0x2a78366d];
    pub const LOCR_KEY: [u32; 4] = [0x53527737, 0x7506499E, 0xBD39AEE3, 0xA59E7268];
    const DEFAULT_ENCRYPTED_HEADER: [u8; 0x10] = [
        0x22, 0x3d, 0x6f, 0x9a, 0xb3, 0xf8, 0xfe, 0xb6, 0x61, 0xd9, 0xcc, 0x1c, 0x62, 0xde, 0x83,
        0x41,
    ];
    const DELTA: u32 = 0x61C88647;

    pub fn is_encrypted_text_file(input_buffer: &[u8]) -> bool {
        if input_buffer.len() > 0x13 {
            input_buffer[..0x10] == Self::DEFAULT_ENCRYPTED_HEADER
        } else {
            false
        }
    }

    #[allow(dead_code)]
    fn encipher(data: &mut [u32], key: &[u32; 4]) {
        hitman_xtea::encipher(data, Self::DELTA, Self::DEFAULT_NUMBER_OF_ROUNDS, key)
    }

    #[allow(dead_code)]
    fn decipher(data: &mut [u32], key: &[u32; 4]) {
        hitman_xtea::decipher(data, Self::DELTA, Self::DEFAULT_NUMBER_OF_ROUNDS, key)
    }

    pub fn decrypt_text_file(input_buffer: &[u8], key: &[u32; 4]) -> Result<String, XteaError>{

        let bytes = hitman_xtea::decipher_file(
            input_buffer,
            Self::DELTA,
            Self::DEFAULT_ENCRYPTED_HEADER.as_slice(),
            Self::DEFAULT_NUMBER_OF_ROUNDS,
            key,
        ).map_err(XteaError::DecryptionError)?;

        String::from_utf8(bytes).map_err(XteaError::TextEncodingError)
    }

    pub fn encrypt_text_file(input_string: String, key: &[u32; 4]) -> Result<Vec<u8>, XteaError>{
        hitman_xtea::encipher_file(
            input_string.as_bytes(),
            Self::DELTA,
            Self::DEFAULT_ENCRYPTED_HEADER.as_slice(),
            Self::DEFAULT_NUMBER_OF_ROUNDS,
            key
        ).map_err(XteaError::DecryptionError)
    }

    // pub fn decrypt_string(input_buffer: Vec<u8>, key: &[u32; 4]) {
    //     todo!()
    // }

    // pub fn encrypt_string(input_string: String, key: &[u32; 4]) {
    //     todo!()
    // }

}
