use std::io::{Write, Read};
use serde::{Deserialize};

#[derive(Deserialize)]
pub struct Response {
    pub version: ResponseVersion,
    pub players: ResponsePlayers,
    pub description: serde_json::Value,
    pub favicon: Option<String>,

    #[serde(rename = "enforcesSecureChat")]
    pub enforces_secure_chat: Option<bool>,

    #[serde(rename = "previewsChat")]
    pub previews_chat: Option<bool>
}

#[derive(Deserialize)]
pub struct ResponseVersion {
    pub name: String,
    pub protocol: i32
}

#[derive(Deserialize)]
pub struct ResponsePlayers {
    pub max: i32,
    pub online: i32,
}

pub fn write_var_int<T: Write>(output: &mut T, value: i32) {
    // Signed, little-endian, variable-length number. The length varies from 1 to 5 bytes as maximum.
    const CONTINUE_BIT: u8 = 0b10000000;
    const SEGMENT_BITS: u32 = 0b01111111;
    let value: u32 = value as u32;
    for i in 0..5 {
        let next_value = value >> (i*7);
        let segment_data = next_value & SEGMENT_BITS;
        if (next_value & !SEGMENT_BITS) == 0 {
            output.write_all(&[segment_data as u8]).unwrap();
            return;
        } else {
            output.write_all(&[segment_data as u8 | CONTINUE_BIT]).unwrap();
        }
    }

    // We should not reach this point!
    panic!("Attempting to write more than 5 bytes of data for VarInt {value:#x} ({value})")
}

pub fn read_var_int<T: Read>(input: &mut T) -> i32 {
    // Signed, little-endian, variable-length number. The length varies from 1 to 5 bytes as maximum.
    const CONTINUE_BIT: u8 = 0b10000000;
    const SEGMENT_BITS: u8 = 0b01111111;
    let mut num: u32 = 0;

    // Read at most five bytes
    for (i, next) in input.take(5).bytes().enumerate() {
        if let Ok(byte) = next {
            num |= ((byte & SEGMENT_BITS) as u32) << (i*7);
            if byte & CONTINUE_BIT == 0 {
                return num as i32;
            }
        } else {
            panic!("Invalid VarInt. Could not successfully decode the value because there were not enough bytes to read. Could only read {} byte(s).", i+1);
        }
    }

    // Either zero bytes were read or attempted to read more than 5 bytes
    panic!("Invalid VarInt");
}

pub fn write_string<T: Write>(output: &mut T, value: &str) {
    // UTF-8 string prefixed with a size as a VarInt. We will use the built-in String data type as it already supports
    // UTF-8 out of the box.
    let str_len = value.len() as i32;
    write_var_int(output, str_len);
    output.write_all(value.as_bytes()).unwrap();
}

pub fn read_string<T: Read>(input: &mut T) -> String {
    // UTF-8 string prefixed with a size as a VarInt. We will use the built-in String data type as it already supports
    // UTF-8 out of the box.
    let size = read_var_int(input);
    if size < 0 {
        panic!("Invalid String size: {size}");
    }

    // Ensure we read exactly *size* bytes. Panic otherwise.
    let mut utf8_data = vec![0; size as usize];
    input.read_exact(&mut utf8_data).unwrap();
    String::from_utf8(utf8_data).unwrap()
}

pub fn read_long<T: Read>(input: &mut T) -> i64 {
    // Signed, big-endian, 64-bit integer
    let mut bytes = [0; 8];

    // Ensure we read exactly 8 bytes or panic otherwise
    input.read_exact(&mut bytes).unwrap();
    i64::from_be_bytes(bytes)
}

pub fn write_unsigned_short<T: Write>(output: &mut T, value: u16) {
    // Unsigned, big-endian, 16-bit integer
    let value_bytes = value.to_be_bytes();
    output.write_all(&value_bytes).unwrap();
}

pub fn write_long<T: Write>(output: &mut T, value: i64) {
    // Signed, big-endian, 64-bit integer
    let value_bytes = value.to_be_bytes();
    output.write_all(&value_bytes).unwrap();
}

#[cfg(test)]
mod var_int_tests {
    use super::*;

    #[test]
    fn test_write_var_int_0() {
        const WRITE_VALUE: i32 = 0;
        const EXPECTED_VALUE: &[u8] = &[0x0];

        let mut buffer: Vec<u8> = Vec::with_capacity(10);
        write_var_int(&mut buffer, WRITE_VALUE);
        assert_eq!(buffer.len(), EXPECTED_VALUE.len());
        assert_eq!(&buffer, EXPECTED_VALUE);
    }

    #[test]
    fn test_read_var_int_0() {
        let mut bytes: &[u8] = &[0x0];
        const EXPECTED_VALUE: i32 = 0;
        let read_value = read_var_int(&mut bytes);
        assert_eq!(read_value, EXPECTED_VALUE);
    }

    #[test]
    fn test_write_var_int_1() {
        const WRITE_VALUE: i32 = 1;
        const EXPECTED_VALUE: &[u8] = &[0x1];

        let mut buffer: Vec<u8> = Vec::with_capacity(10);
        write_var_int(&mut buffer, WRITE_VALUE);
        assert_eq!(buffer.len(), EXPECTED_VALUE.len());
        assert_eq!(&buffer, EXPECTED_VALUE);
    }

    #[test]
    fn test_read_var_int_1() {
        let mut bytes: &[u8] = &[0x1];
        const EXPECTED_VALUE: i32 = 1;
        let read_value = read_var_int(&mut bytes);
        assert_eq!(read_value, EXPECTED_VALUE);
    }

    #[test]
    fn test_write_var_int_127() {
        const WRITE_VALUE: i32 = 127;
        const EXPECTED_VALUE: &[u8] = &[0x7F];

        let mut buffer: Vec<u8> = Vec::with_capacity(10);
        write_var_int(&mut buffer, WRITE_VALUE);
        assert_eq!(buffer.len(), EXPECTED_VALUE.len());
        assert_eq!(&buffer, EXPECTED_VALUE);
    }

    #[test]
    fn test_read_var_int_127() {
        let mut bytes: &[u8] = &[0x7F];
        const EXPECTED_VALUE: i32 = 127;
        let read_value = read_var_int(&mut bytes);
        assert_eq!(read_value, EXPECTED_VALUE);
    }

    #[test]
    fn test_write_var_int_128() {
        const WRITE_VALUE: i32 = 128;
        const EXPECTED_VALUE: &[u8] = &[0x80, 0x01];

        let mut buffer: Vec<u8> = Vec::with_capacity(10);
        write_var_int(&mut buffer, WRITE_VALUE);
        assert_eq!(buffer.len(), EXPECTED_VALUE.len());
        assert_eq!(&buffer, EXPECTED_VALUE);
    }

    #[test]
    fn test_read_var_int_128() {
        let mut bytes: &[u8] = &[0x80, 0x01];
        const EXPECTED_VALUE: i32 = 128;
        let read_value = read_var_int(&mut bytes);
        assert_eq!(read_value, EXPECTED_VALUE);
    }

    #[test]
    fn test_write_var_int_25565() {
        const WRITE_VALUE: i32 = 25565;
        const EXPECTED_VALUE: &[u8] = &[0xDD, 0xC7, 0x01];

        let mut buffer: Vec<u8> = Vec::with_capacity(10);
        write_var_int(&mut buffer, WRITE_VALUE);
        assert_eq!(buffer.len(), EXPECTED_VALUE.len());
        assert_eq!(&buffer, EXPECTED_VALUE);
    }

    #[test]
    fn test_read_var_int_25565() {
        let mut bytes: &[u8] = &[0xDD, 0xC7, 0x01];
        const EXPECTED_VALUE: i32 = 25565;
        let read_value = read_var_int(&mut bytes);
        assert_eq!(read_value, EXPECTED_VALUE);
    }

    #[test]
    fn test_write_var_int_2147483647() {
        const WRITE_VALUE: i32 = 2147483647;
        const EXPECTED_VALUE: &[u8] = &[0xFF, 0xFF, 0xFF, 0xFF, 0x07];

        let mut buffer: Vec<u8> = Vec::with_capacity(10);
        write_var_int(&mut buffer, WRITE_VALUE);
        assert_eq!(buffer.len(), EXPECTED_VALUE.len());
        assert_eq!(&buffer, EXPECTED_VALUE);
    }

    #[test]
    fn test_read_var_int_2147483647() {
        let mut bytes: &[u8] = &[0xFF, 0xFF, 0xFF, 0xFF, 0x07];
        const EXPECTED_VALUE: i32 = 2147483647;
        let read_value = read_var_int(&mut bytes);
        assert_eq!(read_value, EXPECTED_VALUE);
    }

    #[test]
    fn test_write_var_int_negative_1() {
        const WRITE_VALUE: i32 = -1;
        const EXPECTED_VALUE: &[u8] = &[0xFF, 0xFF, 0xFF, 0xFF, 0x0F];

        let mut buffer: Vec<u8> = Vec::with_capacity(10);
        write_var_int(&mut buffer, WRITE_VALUE);
        assert_eq!(buffer.len(), EXPECTED_VALUE.len());
        assert_eq!(&buffer, EXPECTED_VALUE);
    }

    #[test]
    fn test_read_var_int_negative_1() {
        let mut bytes: &[u8] = &[0xFF, 0xFF, 0xFF, 0xFF, 0x0F];
        const EXPECTED_VALUE: i32 = -1;
        let read_value = read_var_int(&mut bytes);
        assert_eq!(read_value, EXPECTED_VALUE);
    }

    #[test]
    fn test_write_var_int_negative_2147483648() {
        const WRITE_VALUE: i32 = -2147483648;
        const EXPECTED_VALUE: &[u8] = &[0x80, 0x80, 0x80, 0x80, 0x08];

        let mut buffer: Vec<u8> = Vec::with_capacity(10);
        write_var_int(&mut buffer, WRITE_VALUE);
        assert_eq!(buffer.len(), EXPECTED_VALUE.len());
        assert_eq!(&buffer, EXPECTED_VALUE);
    }

    #[test]
    fn test_read_var_int_negative_2147483648() {
        let mut bytes: &[u8] = &[0x80, 0x80, 0x80, 0x80, 0x08];
        const EXPECTED_VALUE: i32 = -2147483648;
        let read_value = read_var_int(&mut bytes);
        assert_eq!(read_value, EXPECTED_VALUE);
    }

    #[test]
    #[should_panic]
    fn test_invalid_var_int_too_long() {
        let mut bytes: &[u8] = &[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF];
        read_var_int(&mut bytes);
    }

    #[test]
    #[should_panic]
    fn test_invalid_var_int_insufficient_data() {
        let mut bytes: &[u8] = &[0xFF];
        read_var_int(&mut bytes);
    }
}

#[cfg(test)]
mod string_tests {
    use super::*;

    #[test]
    fn test_write_empty_string() {
        let string = "";
        let expected = &[0x0];

        let mut buffer: Vec<u8> = vec![];
        write_string(&mut buffer, string);
        assert_eq!(buffer, expected);
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_write_some_ASCII_characters() {
        let string = "abcd";
        let expected = &[0x4, 0x61, 0x62, 0x63, 0x64];

        let mut buffer: Vec<u8> = vec![];
        write_string(&mut buffer, string);
        assert_eq!(buffer, expected);
    }
}