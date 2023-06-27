fn get_le_u32(bytes: [u8; 4]) -> u32 {
    (bytes[0] as u32) | (bytes[1] as u32) << 8 | (bytes[2] as u32) << 16 | (bytes[3] as u32) << 24
}

fn get_le_u16(bytes: [u8; 2]) -> u32 {
    (bytes[0] as u32) | (bytes[1] as u32) << 8
}

pub struct BmpImg {
    pub rows: usize,
    pub cols: usize,
    pub data: Vec<u8>,
}
impl BmpImg {
    pub fn from_bytes(bytes: &[u8]) -> Self {
        assert!(
            !(bytes[0] != b'B' || bytes[1] != b'M' && bytes[1] != b'A'),
            "Not a supported BMP format"
        );

        let offset = get_le_u32(bytes[10..14].try_into().expect("BMP header error")) as usize;
        let header_size = get_le_u32(bytes[14..18].try_into().expect("BMP header error")) as usize;
        let cols = if header_size == 12 {
            get_le_u16(bytes[18..20].try_into().expect("BMP DIB header error")) as usize
        } else {
            get_le_u32(bytes[18..22].try_into().expect("BMP DIB header error")) as usize
        };
        let rows = if header_size == 12 {
            get_le_u16(bytes[20..22].try_into().expect("BMP DIB header error")) as usize
        } else {
            get_le_u32(bytes[22..26].try_into().expect("BMP DIB header error")) as usize
        };
        let row_size = ((8 * cols + 31) / 32) * 4;
        let size = row_size * rows;
        let mut data = vec![0; size];
        for i in 0..rows {
            let row_offset = offset + (rows - i - 1) * row_size;
            data[i * cols..(i + 1) * cols].copy_from_slice(&bytes[row_offset..row_offset + cols]);
        }
        Self { rows, cols, data }
    }
}
