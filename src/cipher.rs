use crate::apperror::AppError;
use aes::Aes128;
use cipher::consts::U16;
use cipher::{Array, BlockCipherDecrypt, KeyInit};
use log::trace;

pub const NEW_KEY_CORE: [u8; 16] = [
    0x68, 0x7A, 0x48, 0x52, 0x41, 0x6D, 0x73, 0x6F, 0x35, 0x6B, 0x49, 0x6E, 0x62, 0x61, 0x78, 0x57,
];
pub const NEW_KEY_META: [u8; 16] = [
    0x23, 0x31, 0x34, 0x6C, 0x6A, 0x6B, 0x5F, 0x21, 0x5C, 0x5D, 0x26, 0x30, 0x55, 0x3C, 0x27, 0x28,
];

fn convert_to_arrays(input: &[u8]) -> Result<Vec<Array<u8, U16>>, AppError> {
    if input.len() % 16 != 0 {
        return Err(AppError::FileDataError);
    }
    Ok(input
        .chunks(16)
        .map(|chunk| <Array<u8, U16>>::try_from(chunk).map_err(|_| AppError::FileDataError))
        .collect::<Result<Vec<_>, _>>()?)
}

pub fn aes128_to_slice<T: AsRef<[u8]>>(key: &T, blocks: &[u8]) -> Result<Vec<u8>, AppError> {
    trace!("进行AES128解密");
    let key: &Array<u8, U16> = key
        .as_ref()
        .try_into()
        .map_err(|_| AppError::FileDataError)?;

    let mut blocks = convert_to_arrays(blocks)?;

    let cipher = Aes128::new(key);
    cipher.decrypt_blocks(&mut blocks);

    let mut x = Vec::with_capacity(blocks.len() * 16);
    for block in blocks.iter() {
        x.extend_from_slice(block);
    }
    Ok(x)
}

/// 根据 RC4 密钥构建解密表
pub fn build_decrypt_table(key_data: &[u8]) -> [u8; 256] {
    let key_length = key_data.len();
    let mut key_box = (0..=255).collect::<Vec<u8>>();
    let mut last_byte = 0u64;
    let mut key_offset = 0;

    for i in 0..=255 {
        let swap = key_box[i] as u64;
        let temp = (swap + last_byte + key_data[key_offset] as u64) & 0xFF;
        key_offset += 1;
        if key_offset >= key_length {
            key_offset = 0;
        }
        key_box[i] = key_box[temp as usize];
        key_box[temp as usize] = swap as u8;
        last_byte = temp;
    }

    let mut table = [0u8; 256];
    for j in 0..256usize {
        table[j] = key_box
            [(key_box[j] as usize + key_box[(key_box[j] as usize + j) & 0xFF] as usize) & 0xFF];
    }
    table
}

pub fn parse_key(key: &mut [u8]) {
    for item in key.iter_mut() {
        *item ^= 0x64;
    }
}

pub fn unpad(data: &[u8]) -> Vec<u8> {
    data[..data.len() - data[data.len() - 1] as usize].to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unpad_removes_trailing_padding() {
        let data = vec![b'a', b'b', b'c', 0x01];
        assert_eq!(unpad(&data), vec![b'a', b'b', b'c']);
    }

    #[test]
    fn parse_key_xors_with_0x64() {
        let mut key = vec![0x00, 0x64, 0xFF];
        parse_key(&mut key);
        assert_eq!(key, vec![0x64, 0x00, 0x9B]);
    }

    #[test]
    fn build_decrypt_table_is_256_bytes() {
        let table = build_decrypt_table(&[0x01, 0x02, 0x03]);
        assert_eq!(table.len(), 256);
    }

    #[test]
    fn aes128_to_slice_rejects_non_block_aligned_input() {
        assert!(matches!(
            aes128_to_slice(&NEW_KEY_CORE, &[0u8; 15]),
            Err(AppError::FileDataError)
        ));
    }
}
