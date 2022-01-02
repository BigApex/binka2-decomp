use std::ffi::c_void;

pub type FnCbRead = extern "fastcall" fn(
    out: *mut c_void,
    size: usize,
    class: *mut c_void, // sizeof = 0x120 = 288
) -> usize;

pub type FnCbReadOld =
    extern "fastcall" fn(class: *mut c_void, out: *mut c_void, size: usize) -> usize;

type FnParseMetadata = extern "fastcall" fn(
    data: *const c_void,
    size: usize,
    out_channels: *mut u16,
    out_samplerate: *mut u32,
    out_samples_count: *mut u32,
    adw4: *mut u32,
) -> u64;
type FnOpenStream = extern "fastcall" fn(
    data: *mut c_void,
    _aqw4: *mut u64, // unused in BinkA2?
    callback: FnCbRead,
    class: *mut u64, // sizeof = 0x120 = 288
) -> i64;
type FnResetBytePos = extern "fastcall" fn(data: *mut c_void) -> u8;
type FnGetSampleBytePos = extern "fastcall" fn(
    data: *mut c_void, // might be const
    a2: u32,
    a3: *mut u32,
    a4: *mut u32,
) -> u32;
type FnGetSeekPosData = extern "fastcall" fn(
    header: *const c_void,
    header_size: usize,
    a3: u32,
    out1: *mut u32,
    out2: *mut u32,
) -> u64;
type FnDecode = extern "fastcall" fn(
    allocd: *mut c_void,
    stream_data: *const c_void, // yeah, in theory shouldn't get mutated
    stream_data_size: usize,    // u32 in reality, idk
    out_data: *mut c_void,      // idk how size is calculated, amma be real
    a5: u32,                    // or i32, idk, might be the size of the out_data
    consumed: *mut u32,
    out2: *mut u32, // if(out2) -> assert(consumed <= reported (aka prev_out2))
) -> usize; // might be void
type FnGetBlockSize = extern "fastcall" fn(
    allocd: *mut c_void,
    stream_data: *const c_void,
    stream_data_size: usize, // u32 in reality, idk
    out1: *mut u32,
    reported_block_size: *mut u32,
    out3: *mut u32,
);

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

type FnOpenStreamOld = extern "fastcall" fn(
    class: *mut u64,
    data: *mut c_void,
    _aqw4: *mut u64, // unused in BinkA2?
    callback: FnCbReadOld,
) -> i64;
type FnDecoderOld = extern "fastcall" fn(
    class: *mut c_void,
    data: *mut c_void,
    decoded: *mut f32,
    size: usize,
    cb: FnCbReadOld,
) -> usize;

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

type FnDecoderApex2019 = extern "fastcall" fn(
    data: *mut c_void,
    decoded: *mut f32,
    size: usize,
    size2: usize,
    cb: FnCbRead,
    class: *mut c_void,
) -> usize;

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
    pub alloc_size: u32, // "ASI State Block"
    pub unk_c: u32,      // [0xC]+16
    pub frame_len: u32,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct BinkA2Block {
    pub consumed: u32,
    pub reported_block_size: u32,
    pub required_for_next_call: u32,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct BinkA2Decode {
    pub consumed: u32,
    pub samples: u32,
}

#[derive(Debug)]
pub struct BinkA2 {
    binka: *const CBinkA2,
}
impl BinkA2 {
    pub fn new(binka: *const CBinkA2) -> Self {
        Self { binka }
    }

    // data must be AT LEAST 24(0x18) bytes
    pub fn parse_metadata_c(&self, data: &[u8]) -> Option<BinkA2Metadata> {
        let mut channels = 0u16;
        let mut samplerate = 0u32;
        let mut samples_count = 0u32;
        let mut adw4 = [0u32; 4];
        unsafe {
            if ((*self.binka).parse_metadata)(
                data.as_ptr() as *const _,
                data.len(),
                (&mut channels) as *mut _,
                (&mut samplerate) as *mut _,
                (&mut samples_count) as *mut _,
                adw4.as_mut_ptr(),
            ) == 0
            {
                None
            } else {
                Some(BinkA2Metadata {
                    channels,
                    samplerate,
                    samples_count,
                    //
                    alloc_size: adw4[0],
                    unk_c: adw4[1],
                    frame_len: adw4[2],
                })
            }
        }
    }

    pub fn open_stream_c(&self, data: &mut [u8], cb: FnCbRead, class: *mut c_void) -> i64 {
        unsafe {
            ((*self.binka).open_stream)(
                data.as_mut_ptr() as *mut _,
                std::ptr::null_mut(), // unused
                cb,
                class as *mut u64,
            )
        }
    }

    pub fn get_sample_byte_pos_c(&self, data: &mut [u8], a2: u32) -> (u32, u32, u32) {
        unsafe {
            let mut a3 = 0u32;
            let mut a4 = 0u32;
            let ret = ((*self.binka).get_sample_byte_pos)(
                data.as_mut_ptr() as *mut _,
                a2,
                (&mut a3) as *mut _,
                (&mut a4) as *mut _,
            );
            (ret, a3, a4)
        }
    }

    pub fn reset_byte_pos_c(&self, data: &mut [u8]) -> u8 {
        unsafe { ((*self.binka).reset_byte_pos)(data.as_mut_ptr() as *mut _) }
    }

    pub fn get_seek_pos_data_c(&self, header: &[u8], a3: u32) -> (u64, u32, u32) {
        unsafe {
            let mut out = [0u32; 2];
            let ret = ((*self.binka).get_seek_pos_data)(
                header.as_ptr() as *const _,
                header.len(),
                a3,
                &mut out[0] as *mut _,
                &mut out[1] as *mut _,
            );
            (ret, out[0], out[1])
        }
    }

    pub fn decode_c(
        &self,
        allocd: &mut [u8],
        streaming_data: &[u8],
        out_data: &mut [u16],
    ) -> BinkA2Decode {
        unsafe {
            let mut out = [0u32; 2];
            ((*self.binka).decode)(
                allocd.as_mut_ptr() as *mut _,
                streaming_data.as_ptr() as *const _,
                streaming_data.len(),
                out_data.as_mut_ptr() as *mut _,
                out_data.len() as u32,
                &mut out[0] as *mut _,
                &mut out[1] as *mut _,
            );
            BinkA2Decode {
                consumed: out[0],
                samples: out[1],
            }
        }
    }

    pub fn get_block_size_c(&self, allocd: &mut [u8], streaming_data: &[u8]) -> BinkA2Block {
        unsafe {
            let mut out = [0u32; 3];
            ((*self.binka).get_block_size)(
                allocd.as_mut_ptr() as *mut _,
                streaming_data.as_ptr() as *const _,
                streaming_data.len(),
                &mut out[0] as *mut _,
                &mut out[1] as *mut _, // reported block size...
                &mut out[2] as *mut _,
            );
            BinkA2Block {
                consumed: out[0],
                reported_block_size: out[1],
                required_for_next_call: out[2],
            }
        }
    }

    // TODO: refactor to Result<_>
    // TODO: finish
    pub fn parse_metadata(&self, data: &[u8]) -> Option<BinkA2Metadata> {
        if data.len() < 24 || (&data[0..4] != "1FCB".as_bytes() || data[4] > 2) {
            None
        } else {
            let channels = data[5] as u16;
            let samplerate = u16::from_le_bytes(data[6..8].try_into().unwrap()) as u32;
            let samples_count = u32::from_le_bytes(data[8..12].try_into().unwrap());

            // let alloc_size = 0;

            // ??? number of frames in seek table (C) Kostya's Boring Codec World
            let prepend_array_size = if data[4] == 2 {
                u16::from_le_bytes(data[20..22].try_into().unwrap()) as u32
            } else {
                u32::from_le_bytes(data[20..24].try_into().unwrap())
            };

            /*
                movzx   r13d, word ptr [rsp+0x3C]
                add     r13d, 10h
            */
            // God knows what it means
            let unk_c = 16 + u16::from_le_bytes(data[12..14].try_into().unwrap()) as u32; // idk if it's fast or not :/

            // This is BinkA1 stuff?
            let frame_len = if samplerate < 44100 {
                if samplerate >= 22050 {
                    1024
                } else {
                    512
                }
            } else {
                2048
            };

            let int_calc = |idk: u32| -> u32 {
                if samplerate >= 44100 {
                    (idk.wrapping_mul(1 << 8) & 0xFFFFFFF).wrapping_add(175) & 0xFFFFFFF0
                } else {
                    let mul = if samplerate >= 22050 { 1024 } else { 512 };
                    (idk.wrapping_mul(2).wrapping_mul(mul) >> 4).wrapping_add(175) & 0xFFFFFFF0
                }
            };

            let half_channel = (channels + 1) / 2;
            let alloc_size_samples = if half_channel != 0 {
                (0..half_channel)
                    .map(|i| {
                        // TODO: are names even correct?
                        let chan_id = i * 2;
                        int_calc(
                            2 - if channels.wrapping_sub(chan_id) != 0 {
                                1
                            } else {
                                0
                            },
                        )
                    })
                    .reduce(|accum: u32, i| accum.wrapping_add(i))
                    .unwrap() // Should never panic?
            } else {
                0
            };
            // println!("{} {}|{}", alloc_size_samples, channels, half_channel);
            // fancy round to 64?
            let alloc_size_samples_round = (alloc_size_samples + 128 + 63) & 0xFFFFFFC0;
            let alloc_size =
                (alloc_size_samples_round + (4 * prepend_array_size) + 4 + 63) & 0xFFFFFFC0;

            Some(BinkA2Metadata {
                channels,
                samplerate,
                samples_count,
                //
                alloc_size,
                unk_c,
                frame_len,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    mod cbinka2 {
        use super::super::CBinkA2;

        #[test]
        fn cbinka2_size() {
            assert_eq!(std::mem::size_of::<CBinkA2>(), 4 * 2 + 8 * 7);
        }
    }

    mod binka2 {
        use super::super::BinkA2;
        use super::super::CBinkA2;

        // TODO: more test vectors?
        const DATA: [u8; 24] = [
            0x31, 0x46, 0x43, 0x42, 0x02, 0x01, 0x80, 0xBB, 0x30, 0xDE, 0x03, 0x00, 0xF0, 0x02,
            0x00, 0x00, 0x7E, 0xF1, 0x00, 0x00, 0x85, 0x00, 0x01, 0x00,
        ];

        #[test]
        fn binka2_metadata() {
            if let Some(binka) = crate::util::lla("binkawin64.dll") {
                let decoder =
                    BinkA2::new(crate::util::get_decoder(binka).unwrap().cast::<CBinkA2>());
                if let Some(metadata) = decoder.parse_metadata_c(&DATA) {
                    println!("{:#?}", metadata)
                } else {
                    unreachable!("Failed to load parse metadata!")
                }
            } else {
                unreachable!("Failed to load binkawin64.dll!")
            }
        }

        #[test]
        fn binka2_decomp_metadata() {
            if let Some(binka) = crate::util::lla("binkawin64.dll") {
                let decoder =
                    BinkA2::new(crate::util::get_decoder(binka).unwrap().cast::<CBinkA2>());
                if let Some(metadata) = decoder.parse_metadata_c(&DATA) {
                    assert_eq!(metadata, decoder.parse_metadata(&DATA).unwrap());
                } else {
                    unreachable!("Failed to load parse metadata!")
                }
            } else {
                unreachable!("Failed to load binkawin64.dll!")
            }
        }
    }
}
