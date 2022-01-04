use super::{types::*, BINKA2_BANDS_MAX};
use std::ffi::c_void;

#[repr(C)]
pub struct BinkA2Header {
    pub header: u32,
    pub version: u8,
    pub channels: u8,
    pub sample_rate: u16,
    pub samples_count: u32,
    pub max_block_size: u16,
    pub is_new_codec: u16, // determines BinkA ACTUAL 2.0
    pub total_size: u32,

    // next two are true only if version is 2
    pub seek_table_size: u16,
    pub unk16: u16,
}

#[repr(C)]
pub struct BinkA2ClassHeader {
    pub header: u32,
    pub version: u8,
    pub channels: u8,
    pub sample_rate: u16,
    pub samples_count: u32,
    pub max_block_size: u16,
    pub is_new_codec: u16,
    pub total_size: u32,

    // This is already parsed
    pub seek_table_size: u32,
    pub unk16: u32,

    // Non header data
    pub min_stream_size: u32,

    pub decoders_num: u8,
    pub decoders_byte: [u8; 4],
    _pad: [u8; 3],
    pub decoders: [*mut u8; 4],

    pub seek_table: *mut u32,

    _unk: [u8; 48],
}

#[repr(C)]
pub struct BinkA2DecoderInternal {
    pub ptr: *mut c_void,

    pub transform_size: u32,
    pub transform_ratio: f32, // get ratio'd liberal

    pub unk10: u32,
    pub unk14: u32,

    pub bits_shift: u32,
    pub channels: u32,
    pub start_frame: u32,
    pub bands_num: u32,
    _pad: u32,
    pub flags: u32, // up to 4
    pub size: u32,

    pub bands: [u32; BINKA2_BANDS_MAX + 2], // some bands might be padding actually...
}

#[repr(C)]
pub struct CBinkA2 {
    pub idk: u32,
    pub idk2: u32, // might be audio type

    pub parse_metadata: FnParseMetadata,
    pub open_stream: FnOpenStream,
    pub reset_byte_pos: FnResetBytePos,
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
        fn total_size_offset() {
            assert_eq!(
                unsafe { &(*(::std::ptr::null::<BinkA2Header>())).total_size as *const _ as usize },
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
    mod binka2_class_header {
        use super::*;

        // TODO
        #[test]
        fn size() {
            // in reality it's more like 128?
            assert_eq!(std::mem::size_of::<BinkA2ClassHeader>(), 128);
        }

        #[test]
        fn header_offset() {
            assert_eq!(
                unsafe {
                    &(*(::std::ptr::null::<BinkA2ClassHeader>())).header as *const _ as usize
                },
                0usize
            );
        }

        #[test]
        fn version_offset() {
            assert_eq!(
                unsafe {
                    &(*(::std::ptr::null::<BinkA2ClassHeader>())).version as *const _ as usize
                },
                4usize
            );
        }

        #[test]
        fn channels_offset() {
            assert_eq!(
                unsafe {
                    &(*(::std::ptr::null::<BinkA2ClassHeader>())).channels as *const _ as usize
                },
                5usize
            );
        }

        #[test]
        fn sample_rate_offset() {
            assert_eq!(
                unsafe {
                    &(*(::std::ptr::null::<BinkA2ClassHeader>())).sample_rate as *const _ as usize
                },
                6usize
            );
        }

        #[test]
        fn samples_count_offset() {
            assert_eq!(
                unsafe {
                    &(*(::std::ptr::null::<BinkA2ClassHeader>())).samples_count as *const _ as usize
                },
                8usize
            );
        }

        #[test]
        fn max_block_size_offset() {
            assert_eq!(
                unsafe {
                    &(*(::std::ptr::null::<BinkA2ClassHeader>())).max_block_size as *const _
                        as usize
                },
                12usize
            );
        }

        #[test]
        fn total_size_offset() {
            assert_eq!(
                unsafe {
                    &(*(::std::ptr::null::<BinkA2ClassHeader>())).total_size as *const _ as usize
                },
                0x10usize
            );
        }

        #[test]
        fn seek_table_size_offset() {
            assert_eq!(
                unsafe {
                    &(*(::std::ptr::null::<BinkA2ClassHeader>())).seek_table_size as *const _
                        as usize
                },
                20usize
            );
        }

        #[test]
        fn unk16_offset() {
            assert_eq!(
                unsafe { &(*(::std::ptr::null::<BinkA2ClassHeader>())).unk16 as *const _ as usize },
                24usize
            );
        }

        #[test]
        fn min_stream_size_offset() {
            assert_eq!(
                unsafe {
                    &(*(::std::ptr::null::<BinkA2ClassHeader>())).min_stream_size as *const _
                        as usize
                },
                0x1Cusize
            );
        }

        #[test]
        fn decoders_num_offset() {
            assert_eq!(
                unsafe {
                    &(*(::std::ptr::null::<BinkA2ClassHeader>())).decoders_num as *const _ as usize
                },
                0x20usize
            );
        }

        #[test]
        fn decoders_byte_offset() {
            assert_eq!(
                unsafe {
                    &(*(::std::ptr::null::<BinkA2ClassHeader>())).decoders_byte as *const _ as usize
                },
                0x21usize
            );
        }

        #[test]
        fn decoders_offset() {
            assert_eq!(
                unsafe {
                    &(*(::std::ptr::null::<BinkA2ClassHeader>())).decoders as *const _ as usize
                },
                0x28usize
            );
        }

        #[test]
        fn seek_table_offset() {
            assert_eq!(
                unsafe {
                    &(*(::std::ptr::null::<BinkA2ClassHeader>())).seek_table as *const _ as usize
                },
                0x48usize
            );
        }
    }

    #[test]
    fn cbinka2_size() {
        assert_eq!(std::mem::size_of::<CBinkA2>(), 4 * 2 + 8 * 7);
    }
}
