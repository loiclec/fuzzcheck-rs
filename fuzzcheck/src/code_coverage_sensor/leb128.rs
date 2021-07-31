#![macro_use]

macro_rules! impl_read_unsigned_leb128 {
    ($fn_name:ident, $int_ty:ty) => {
        #[inline]
        #[no_coverage]
        pub fn $fn_name(slice: &[u8]) -> ($int_ty, usize) {
            let mut result = 0;
            let mut shift = 0;
            let mut position = 0;
            loop {
                let byte = slice[position];
                position += 1;
                if (byte & 0x80) == 0 {
                    result |= (byte as $int_ty) << shift;
                    return (result, position);
                } else {
                    result |= ((byte & 0x7F) as $int_ty) << shift;
                }
                shift += 7;
            }
        }
    };
}

impl_read_unsigned_leb128!(read_u64_leb128, u64);
