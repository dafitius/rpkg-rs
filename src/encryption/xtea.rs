use std::io::{Cursor};
use extended_tea::{XTEA};
use thiserror::Error;
use crate::encryption::xtea::XteaError::InvalidInput;

/// Errors that can occur during XTEA encryption or decryption.
#[derive(Debug, Error)]
pub enum XteaError {
    #[error("Text encoding error: {0}")]
    TextEncodingError(#[from] std::string::FromUtf8Error),

    #[error("An error occurred while trying to decrypt the file: {:?}", _0)]
    CipherError(#[from] std::io::Error),

    #[error("Unexpected input: {:?}", _0)]
    InvalidInput(String),
}

/// Implementation of XTEA encryption and decryption methods.
pub struct Xtea;

impl Xtea {

    /// Default XTEA key, used in thumbs.dat and packagedefinition.txt
    pub const DEFAULT_KEY: [u32; 4] = [0x30f95282, 0x1f48c419, 0x295f8548, 0x2a78366d];

    /// LOCR/TEXTLIST XTEA key, used for localization
    pub const LOCR_KEY: [u32; 4] = [0x53527737, 0x7506499E, 0xBD39AEE3, 0xA59E7268];

    /// Default header for encrypted files.
    const DEFAULT_ENCRYPTED_HEADER: [u8; 0x10] = [
        0x22, 0x3d, 0x6f, 0x9a, 0xb3, 0xf8, 0xfe, 0xb6, 0x61, 0xd9, 0xcc, 0x1c, 0x62, 0xde, 0x83,
        0x41,
    ];

    /// Checks if a given buffer represents an encrypted text file.
    /// This function will check for the presence of a default header in the text file.
    pub fn is_encrypted_text_file(input_buffer: &[u8]) -> bool {
        input_buffer.starts_with(&Self::DEFAULT_ENCRYPTED_HEADER)
    }

    /// Decrypts a text file given its buffer, uses the default xtea key.
    pub fn decrypt_text_file(input_buffer: &[u8]) -> Result<String, XteaError>{

        let payload_start = Self::DEFAULT_ENCRYPTED_HEADER.len() + 4;

        if input_buffer.len() < payload_start {
            return Err(InvalidInput("Input too short".to_string()));
        }

        if !input_buffer.starts_with(&Self::DEFAULT_ENCRYPTED_HEADER) {
            return Err(InvalidInput("Header mismatch".to_string()));
        }

        let input = &input_buffer[payload_start..];

        if input.len() % 8 != 0{
            return Err(InvalidInput("Input must be of a length divisible by 8".to_string()))
        }

        let xtea = XTEA::new(&Self::DEFAULT_KEY);
        let mut out_buffer = vec![0u8; input.len()];

        let mut input_reader = Cursor::new(input);
        let mut ouput_writer = Cursor::new(&mut out_buffer);

        xtea.decipher_stream::<byteorder::LittleEndian, _, _>(
            &mut input_reader,
            &mut ouput_writer,
        ).map_err(XteaError::CipherError)?;

        String::from_utf8(ouput_writer.get_mut().to_owned()).map_err(XteaError::TextEncodingError)
    }

    /// Decrypts a string given its buffer and key.
    pub fn decrypt_string(input_buffer: Vec<u8>, key: &[u32; 4]) -> Result<String, XteaError>{

        let input = &input_buffer;

        if input.len() % 8 != 0{
            return Err(InvalidInput("Input must be of a length divisible by 8".to_string()))
        }

        let xtea = XTEA::new(key);
        let mut out_buffer = vec![0u8; input.len()];

        let mut input_reader = Cursor::new(input);
        let mut ouput_writer = Cursor::new(&mut out_buffer);

        xtea.decipher_stream::<byteorder::LittleEndian, _, _>(
            &mut input_reader,
            &mut ouput_writer,
        ).map_err(XteaError::CipherError)?;

        String::from_utf8(ouput_writer.get_mut().to_owned()).map_err(XteaError::TextEncodingError)
    }

    // fn encrypt_text_file(input_string: String, key: &[u32; 4]) -> Result<Vec<u8>, XteaError>{
    //     todo!()
    // }
    //
    // fn encrypt_string(input_string: String, key: &[u32; 4]) {
    //     todo!()
    // }

}
