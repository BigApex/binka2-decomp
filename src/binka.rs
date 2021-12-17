use std::ffi::c_void;

type FnCbRead = extern "fastcall" fn(
    out: *mut c_void,
    size: usize,
    class: *mut c_void, // sizeof = 0x120 = 288
);

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
);

#[repr(C)]
pub struct CBinkA2 {
    pub idk: u32,
    pub idk2: u32,

    pub parse_metadata: FnParseMetadata,
    pub open_stream: FnOpenStream,
    _unk10: *const c_void,
    _unk18: *const c_void,
    _unk20: *const c_void,
    _unk28: *const c_void,
    _unk30: *const c_void,
    // padding too???
    // _pad: *const c_void,
}

#[derive(Debug)]
pub struct BinkA2Metadata {
    pub channels: u16,
    pub samplerate: u32,
    pub samples_count: u32,

    // Unpacked array...
    pub alloc_size: u32, // "ASI State Block"
    pub unk_c: u32,      // [0xC]+16
    pub frame_len: u32,
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
                data.as_ptr() as *const c_void,
                data.len(),
                (&mut channels) as *mut u16,
                (&mut samplerate) as *mut u32,
                (&mut samples_count) as *mut u32,
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

    // TODO: refactor to Result<_>
    // TODO: finish
    pub fn parse_metadata(&self, data: &[u8]) -> Option<BinkA2Metadata> {
        if data.len() < 24 {
            None
        } else {
            if &data[0..4] != "1FCB".as_bytes() || data[4] > 2 {
                None
            } else {
                let channels = data[5] as u16;
                let samplerate = u16::from_le_bytes(data[6..8].try_into().unwrap()) as u32;
                let samples_count = u32::from_le_bytes(data[8..12].try_into().unwrap());

                // let alloc_size = 0;

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
                let unk_c = 16 + u32::from_le_bytes(data[12..16].try_into().unwrap()); // idk if it's fast or not :/

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
                                2 - if channels.wrapping_sub(chan_id) != 0 { 1 } else { 0 },
                            )
                        })
                        .reduce(|accum: u32, i| accum.wrapping_add(i))
                        .unwrap() // Should never panic?
                } else {
                    0
                };
                println!("{} {}|{}", alloc_size_samples, channels, half_channel);
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

        #[test]
        fn binka2_metadata() {
            const DATA: [u8; 24] = [
                0x31, 0x46, 0x43, 0x42, 0x02, 0x01, 0x80, 0xBB, 0x30, 0xDE, 0x03, 0x00, 0xF0, 0x02,
                0x00, 0x00, 0x7E, 0xF1, 0x00, 0x00, 0x85, 0x00, 0x01, 0x00,
            ];
            if let Some(binka) = crate::util::lla("binkawin64.dll") {
                let decoder =
                    BinkA2::new(unsafe { binka.cast::<u8>().add(0x19000).cast::<CBinkA2>() });
                if let Some(metadata) = decoder.parse_metadata_c(&DATA) {
                    println!("{:#?}", metadata)
                } else {
                    unreachable!("Failed to load parse metadata!")
                }
            } else {
                unreachable!("Failed to load binkawin64.dll!")
            }
        }
    }
}
