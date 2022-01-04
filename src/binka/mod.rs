mod c_impl;
pub mod structs;
mod types;

pub use c_impl::*;
pub use structs::*;
pub use types::{FnCbRead, FnCbReadOld};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct BinkA2Block {
    pub consumed: u32,
    pub reported_block_size: u32,
    pub required_for_next_call: u32,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct BinkA2Seek {
    pub pos: u32,
    pub unk: u32,
    pub block_size: u32,
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

    // TODO: refactor to Result<_>
    // TODO: finish
    pub fn parse_metadata(&self, data: &[u8]) -> Option<BinkA2Metadata> {
        if data.len() < 24 || (&data[0..4] != b"1FCB" || data[4] > 2) {
            None
        } else {
            let header = unsafe { &*(data.as_ptr() as *const BinkA2Header) };

            let channels = header.channels as u16; //data[5] as u16;
            let samplerate = header.sample_rate as u32; //u16::from_le_bytes(data[6..8].try_into().unwrap()) as u32;
            let samples_count = header.samples_count; //u32::from_le_bytes(data[8..12].try_into().unwrap());

            // let alloc_size = 0;

            // ??? number of frames in seek table (C) Kostya's Boring Codec World
            let prepend_array_size = if header.version == 2 {
                u16::from_le_bytes(data[20..22].try_into().unwrap()) as u32
            } else {
                u32::from_le_bytes(data[20..24].try_into().unwrap())
            };

            /*
                movzx   r13d, word ptr [rsp+0x3C]
                add     r13d, 10h
            */
            // God knows what it means
            let max_block_size = header.max_block_size; //u16::from_le_bytes(data[12..14].try_into().unwrap()) as u32; // idk if it's fast or not :/
            let max_stream_size = 16 + max_block_size;

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
            // fancy align up to 64?
            let alloc_size_samples_round = (alloc_size_samples + 128 + 63) & 0xFFFFFFC0;
            let alloc_size =
                (alloc_size_samples_round + (4 * prepend_array_size) + 4 + 63) & 0xFFFFFFC0;

            Some(BinkA2Metadata {
                channels,
                samplerate,
                samples_count,
                //
                alloc_size,
                max_stream_size,
                frame_len,
            })
        }
    }

    pub fn reset_byte_pos(&self, allocd: &mut [u8]) {
        let size = allocd[32];
        if size != 0 {
            unsafe {
                let oof = std::slice::from_raw_parts::<*mut u8>(
                    u64::from_le_bytes(allocd[40..48].try_into().unwrap()) as *mut _,
                    size as usize,
                );
                for i in oof {
                    let ptr = *i;
                    if !ptr.is_null() {
                        *ptr.add(32).cast::<u32>() = 1u32;
                    }
                }
            }
        }
    }

    pub fn get_samples_count_in_block(sample_rate: u16) -> u32 {
        if sample_rate <= 44100 {
            if sample_rate >= 22050 {
                960
            } else {
                480
            }
        } else {
            1920
        }
    }

    // TODO: anything else other than 0?
    pub fn get_seek_pos_data(data: &[u8], sample_num: u32) -> BinkA2Seek {
        if data.len() < 24 || (&data[0..4] != b"1FCB" || data[4] > 2) {
            BinkA2Seek {
                pos: u32::MAX,
                unk: 0,
                block_size: 0,
            }
        } else {
            let header = unsafe { &*(data.as_ptr() as *const BinkA2Header) };

            let min_size = (2 * header.seek_table_size as usize) + 24;
            if data.len() < min_size {
                BinkA2Seek {
                    pos: 0,
                    unk: 0,
                    block_size: 0,
                }
            } else {
                let samples_in_block = Self::get_samples_count_in_block(header.sample_rate);
                let some_sample_count = header.unk16 as u32 * samples_in_block;
                let v15 = if sample_num >= some_sample_count {
                    let ret = ((sample_num - some_sample_count) / some_sample_count) + 1;
                    if ret >= (header.seek_table_size as u32 - 1) {
                        header.seek_table_size as u32 - 1
                    } else {
                        ret
                    }
                } else {
                    0
                };

                let unk = some_sample_count * v15;

                let v20 = 0;
                let v18 = 0;
                let v14 = 0;
                let slice = if v15 >= 2 {
                    let slice_size = ((v15 - 2) >> 1) + 1;
                    let _slice = unsafe {
                        std::slice::from_raw_parts::<[u16; 2]>(
                            data[0x18..].as_ptr() as *const _,
                            slice_size as usize,
                        )
                    };

                    todo!();
                } else {
                    unsafe {
                        std::slice::from_raw_parts::<[u16; 2]>(data[0x18..].as_ptr() as *const _, 1)
                    }
                };
                let (v19, v17) = if v20 < v15 {
                    (slice[0][0], slice[0][0] + 1)
                } else {
                    (0, slice[0][0])
                };

                let v24 = v14 + v18 + v19;

                let block_size = if v15 >= (header.seek_table_size as u32 - 1) {
                    header.unk10 - v24 as u32 - min_size as u32
                } else {
                    v17 as u32
                };

                BinkA2Seek {
                    pos: v24 as u32 + min_size as u32,
                    unk,
                    block_size,
                }
            }
        }
    }

    pub fn get_sample_byte_pos(&self, allocd: &[u8], sample_num: u32) -> BinkA2Seek {
        let header = unsafe { &*(allocd.as_ptr() as *const BinkA2HeaderClass) };
        let samples_in_block = Self::get_samples_count_in_block(header.sample_rate);

        let sample_num = if sample_num > header.samples_count {
            header.samples_count - 1
        } else {
            sample_num
        };

        let some_sample_count = samples_in_block * header.unk16;
        let v11 = if sample_num >= some_sample_count {
            let ret = ((sample_num - some_sample_count) / some_sample_count) + 1;
            if ret >= header.seek_table_size {
                header.seek_table_size - 1
            } else {
                ret
            }
        } else {
            0
        };

        let unk = some_sample_count * v11;
        let ptr = unsafe { *allocd.as_ptr().add(72).cast::<*const u32>() };
        let b = unsafe { *ptr.add(v11 as usize) };
        let block_size = if v11 < (header.seek_table_size - 1) {
            unsafe {
                let a = *ptr.add(v11 as usize + 1);
                a - b
            }
        } else {
            header.unk10 - b
        };

        BinkA2Seek {
            pos: u32::from_le_bytes(allocd[28..32].try_into().unwrap()) + b,
            unk,
            block_size,
        }
    }

    fn get_block_size_detail(&self, allocd: &mut [u8], streaming_data: &[u8]) -> BinkA2Block {
        // TODO: swap for rust impl
        self.reset_byte_pos(allocd);

        let mut pos = 0;
        let consumed = if streaming_data.len() < 4 {
            0xFFFF
        } else {
            loop {
                if &streaming_data[pos..pos + 4] == b"BCF1" {
                    break u32::from_le_bytes([allocd[28], allocd[29], allocd[30], allocd[31]]);
                }

                if streaming_data[pos..pos + 2] == [0x99, 0x99] {
                    let (block_size, header_size) =
                        if streaming_data[pos + 2..pos + 4] == [0xFF, 0xFF] {
                            if (streaming_data.len() - pos) < 8 {
                                break 0xFFFF;
                            }

                            (
                                u16::from_le_bytes(
                                    streaming_data[pos + 4..pos + 6].try_into().unwrap(),
                                ),
                                8,
                            )
                        } else {
                            (
                                u16::from_le_bytes(
                                    streaming_data[pos + 2..pos + 4].try_into().unwrap(),
                                ),
                                4,
                            )
                        };

                    if block_size <= u16::from_le_bytes([allocd[12], allocd[13]]) {
                        return BinkA2Block {
                            consumed: pos as u32,
                            reported_block_size: header_size + block_size as u32,
                            required_for_next_call: 8,
                        };
                    }
                }

                pos += 1;
                if (streaming_data.len() - pos) < 4 {
                    break 0xFFFF;
                }
            }
        };

        BinkA2Block {
            consumed,
            reported_block_size: 0xFFFF,
            required_for_next_call: 8,
        }
    }

    pub fn get_block_size(&self, allocd: &mut [u8], streaming_data: &[u8]) -> BinkA2Block {
        if allocd.len() < 4 {
            return BinkA2Block {
                consumed: 0,
                reported_block_size: 0xFFFF,
                required_for_next_call: 0,
            };
        }

        if [streaming_data[0], streaming_data[1]] != [0x99, 0x99] {
            self.get_block_size_detail(allocd, streaming_data)
        } else {
            let size = u16::from_le_bytes([streaming_data[2], streaming_data[3]]);
            let (size, v8) = if size == 0xFFFF {
                if streaming_data.len() < 8 {
                    return BinkA2Block {
                        consumed: 0,
                        reported_block_size: 0xFFFF,
                        required_for_next_call: 0,
                    };
                }

                (size, 8)
            } else {
                (size, 4)
            };

            if size > u16::from_le_bytes([allocd[12], allocd[13]]) {
                self.get_block_size_detail(allocd, streaming_data)
            } else {
                BinkA2Block {
                    consumed: 0,
                    reported_block_size: v8 + size as u32,
                    required_for_next_call: 8,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
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
