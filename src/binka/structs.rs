use super::types::*;
use std::ffi::c_void;

#[repr(C)]
pub struct BinkA2Header {
    pub header: u32,
    pub version: u8,
    pub channels: u8,
    pub sample_rate: u16,
    pub samples_count: u32,
    pub max_block_size: u32,
    pub unk10: u32,

    // next two are true only if version is 2
    pub seek_table_size: u16,
    pub unk16: u16,
}

#[repr(C)]
pub struct BinkA2HeaderClass {
    pub header: u32,
    pub version: u8,
    pub channels: u8,
    pub sample_rate: u16,
    pub samples_count: u32,
    pub max_block_size: u32,
    pub unk10: u32,

    // This is already parsed
    pub seek_table_size: u32,
    pub unk16: u32,
}

#[repr(C)]
pub struct CBinkA2 {
    pub idk: u32,
    pub idk2: u32,

    pub parse_metadata: FnParseMetadata,
    pub open_stream: FnOpenStream,
    pub reset_byte_pos: FnResetBytePos, // reset???
    pub get_sample_byte_pos: FnGetSampleBytePos,
    pub get_seek_pos_data: FnGetSeekPosData,
    pub decode: FnDecode,
    pub get_block_size: FnGetBlockSize,
    // padding too???
    // _pad: *const c_void,
}

// TF2's binkawin64's decoder
#[repr(C)]
pub struct CBinkA2_old {
    pub idk: u32,
    pub idk2: u32,

    pub parse_metadata: FnParseMetadata,
    pub open_stream: FnOpenStreamOld,
    pub decode: FnDecoderOld,
    pub reset_byte_pos: FnResetBytePos,
    pub get_sample_byte_pos: FnGetSampleBytePos,
    _unk30: *const c_void,
    // padding too???
    // _pad: *const c_void,
}

// 2019 Apex's binkawin64's decoder
#[repr(C)]
pub struct CBinkA2_2019 {
    pub idk: u32,
    pub idk2: u32,

    pub parse_metadata: FnParseMetadata,
    pub open_stream: FnOpenStream,
    pub decode: FnDecoderApex2019,
    pub reset_byte_pos: FnResetBytePos,
    pub get_sample_byte_pos: FnGetSampleBytePos,
    _unk30: *const c_void,
    // padding too???
    // _pad: *const c_void,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct BinkA2Metadata {
    pub channels: u16,
    pub samplerate: u32,
    pub samples_count: u32,

    // Unpacked array...
    pub alloc_size: u32,      // "ASI State Block"
    pub max_stream_size: u32, // ??? [0xC]+16
    pub frame_len: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(deref_nullptr)]
    mod binka2_header {
        use super::*;

        #[test]
        fn size() {
            assert_eq!(std::mem::size_of::<BinkA2Header>(), 0x18);
        }

        #[test]
        fn header_offset() {
            assert_eq!(
                unsafe { &(*(::std::ptr::null::<BinkA2Header>())).header as *const _ as usize },
                0usize
            );
        }

        #[test]
        fn version_offset() {
            assert_eq!(
                unsafe { &(*(::std::ptr::null::<BinkA2Header>())).version as *const _ as usize },
                4usize
            );
        }

        #[test]
        fn channels_offset() {
            assert_eq!(
                unsafe { &(*(::std::ptr::null::<BinkA2Header>())).channels as *const _ as usize },
                5usize
            );
        }

        #[test]
        fn sample_rate_offset() {
            assert_eq!(
                unsafe {
                    &(*(::std::ptr::null::<BinkA2Header>())).sample_rate as *const _ as usize
                },
                6usize
            );
        }

        #[test]
        fn samples_count_offset() {
            assert_eq!(
                unsafe {
                    &(*(::std::ptr::null::<BinkA2Header>())).samples_count as *const _ as usize
                },
                8usize
            );
        }

        #[test]
        fn max_block_size_offset() {
            assert_eq!(
                unsafe {
                    &(*(::std::ptr::null::<BinkA2Header>())).max_block_size as *const _ as usize
                },
                12usize
            );
        }

        #[test]
        fn unk10_offset() {
            assert_eq!(
                unsafe { &(*(::std::ptr::null::<BinkA2Header>())).unk10 as *const _ as usize },
                0x10usize
            );
        }

        #[test]
        fn seek_table_size_offset() {
            assert_eq!(
                unsafe {
                    &(*(::std::ptr::null::<BinkA2Header>())).seek_table_size as *const _ as usize
                },
                20usize
            );
        }

        #[test]
        fn unk16_offset() {
            assert_eq!(
                unsafe { &(*(::std::ptr::null::<BinkA2Header>())).unk16 as *const _ as usize },
                0x16usize
            );
        }
    }

    #[allow(deref_nullptr)]
    mod binka2_header_class {
        use super::*;

        // TODO
        #[test]
        fn size() {
            // in reality it's more like 128?
            assert_eq!(std::mem::size_of::<BinkA2HeaderClass>(), 0x1C);
        }

        #[test]
        fn header_offset() {
            assert_eq!(
                unsafe {
                    &(*(::std::ptr::null::<BinkA2HeaderClass>())).header as *const _ as usize
                },
                0usize
            );
        }

        #[test]
        fn version_offset() {
            assert_eq!(
                unsafe {
                    &(*(::std::ptr::null::<BinkA2HeaderClass>())).version as *const _ as usize
                },
                4usize
            );
        }

        #[test]
        fn channels_offset() {
            assert_eq!(
                unsafe {
                    &(*(::std::ptr::null::<BinkA2HeaderClass>())).channels as *const _ as usize
                },
                5usize
            );
        }

        #[test]
        fn sample_rate_offset() {
            assert_eq!(
                unsafe {
                    &(*(::std::ptr::null::<BinkA2HeaderClass>())).sample_rate as *const _ as usize
                },
                6usize
            );
        }

        #[test]
        fn samples_count_offset() {
            assert_eq!(
                unsafe {
                    &(*(::std::ptr::null::<BinkA2HeaderClass>())).samples_count as *const _ as usize
                },
                8usize
            );
        }

        #[test]
        fn max_block_size_offset() {
            assert_eq!(
                unsafe {
                    &(*(::std::ptr::null::<BinkA2HeaderClass>())).max_block_size as *const _
                        as usize
                },
                12usize
            );
        }

        #[test]
        fn unk10_offset() {
            assert_eq!(
                unsafe { &(*(::std::ptr::null::<BinkA2HeaderClass>())).unk10 as *const _ as usize },
                0x10usize
            );
        }

        #[test]
        fn seek_table_size_offset() {
            assert_eq!(
                unsafe {
                    &(*(::std::ptr::null::<BinkA2HeaderClass>())).seek_table_size as *const _
                        as usize
                },
                20usize
            );
        }

        #[test]
        fn unk16_offset() {
            assert_eq!(
                unsafe { &(*(::std::ptr::null::<BinkA2HeaderClass>())).unk16 as *const _ as usize },
                24usize
            );
        }
    }

    #[test]
    fn cbinka2_size() {
        assert_eq!(std::mem::size_of::<CBinkA2>(), 4 * 2 + 8 * 7);
    }
}
