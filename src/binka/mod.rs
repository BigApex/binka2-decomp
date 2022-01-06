#![allow(clippy::too_many_arguments)]

mod c_impl;
mod consts;
mod math;
mod mss;
pub mod structs;
mod types;

use std::usize;

pub use c_impl::*;
pub use consts::*;
use math::*;
use mss::*;
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
    pub fn parse_metadata(&self, data: &[u8]) -> Option<BinkA2Metadata> {
        if data.len() < 24 || (&data[0..4] != b"1FCB" || data[4] > 2) {
            None
        } else {
            let header = unsafe { &*(data.as_ptr() as *const BinkA2Header) };

            let channels = header.channels as u16;
            let samplerate = header.sample_rate as u32;
            let samples_count = header.samples_count;

            // ??? number of frames in seek table (C) Kostya's Boring Codec World
            let prepend_array_size = if header.version == 2 {
                header.seek_table_size as u32
            } else {
                u32::from_le_bytes(data[20..24].try_into().unwrap())
            };

            /*
                movzx   r13d, word ptr [rsp+0x3C]
                add     r13d, 10h
            */
            // God knows what it means
            let max_block_size = header.max_block_size as u32; //u16::from_le_bytes(data[12..14].try_into().unwrap()) as u32; // idk if it's fast or not :/
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

            let alloc_size_samples = get_total_decoders_alloc_size(channels, samplerate);
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

    pub fn open_stream<T>(&self, data: &mut [u8], cb: FnCbRead, class: *mut T) -> u64 {
        // we read the data right after that...
        #[allow(clippy::uninit_assumed_init)]
        let mut header: BinkA2Header = unsafe { std::mem::MaybeUninit::uninit().assume_init() };
        let read = cb(
            (&mut header) as *mut _ as *mut _,
            std::mem::size_of::<BinkA2Header>(),
            class as *mut _,
        );
        if read != 24 || header.header != 0x42434631 || header.version > 2 {
            0
        } else {
            // Remove mutability from now on
            let header = &header;
            let data_header = unsafe { &mut *(data.as_mut_ptr() as *mut BinkA2ClassHeader) };
            unsafe {
                std::slice::from_raw_parts_mut(
                    data.as_mut_ptr(),
                    std::mem::size_of::<BinkA2ClassHeader>(),
                )
                .fill(0)
            };
            data_header.header = 0x42434631;
            data_header.version = header.version;
            data_header.channels = header.channels;
            data_header.sample_rate = header.sample_rate;
            data_header.samples_count = header.samples_count;
            data_header.max_block_size = header.max_block_size;
            data_header.is_new_codec = header.is_new_codec;
            data_header.total_size = header.total_size;
            let (seek_table_size, unk16) = if header.version != 2 {
                todo!()
            } else {
                (header.seek_table_size, header.unk16)
            };
            data_header.seek_table_size = seek_table_size as u32;
            data_header.unk16 = unk16 as u32;

            let decoders_num = (header.channels + 1) >> 1;
            let decoder_chan_num = get_channel_num_decoders(header.channels as u16);

            let (decoder_alloc_sizes, decoder_alloc_size_total) =
                get_decoder_alloc_size_channel_num(
                    header.channels as u16,
                    header.sample_rate as u32,
                );
            debug_assert_eq!(
                decoder_alloc_size_total,
                get_total_decoders_alloc_size(header.channels as u16, header.sample_rate as u32)
            );
            let decoder_alloc_size_samples_round =
                (decoder_alloc_size_total + 128 + 63) & 0xFFFFFFC0;

            let seek_array = unsafe {
                let seek_array = data
                    .as_mut_ptr()
                    .add(decoder_alloc_size_samples_round as usize);
                // *data.as_mut_ptr().add(72).cast::<*mut u8>() = seek_array;
                // *data.as_mut_ptr().add(28).cast::<u32>() = (2 * data_header.seek_table_size) + 24;
                data_header.seek_table = seek_array.cast();
                data_header.min_stream_size = (2 * data_header.seek_table_size) + 24;

                data_header.seek_table
            };

            // Populate internal seek table
            if seek_table_size != 0 {
                let mut seek_pos = 0;
                let mut v20 = 0;
                let mut v22 = 0;
                let mut v43 = 0;
                loop {
                    let v23 = seek_table_size as u32 - v20;
                    let (v23, v24) = if v23 > 0x80 {
                        (0x80, 0x80)
                    } else {
                        (v23, seek_table_size as u32 - v22)
                    };
                    if v23 != 0 {
                        let mut buf = [0u16; 128];
                        if cb(
                            buf.as_mut_ptr() as *mut _,
                            2 * v23 as usize,
                            class as *mut _,
                        ) != 2 * v23 as usize
                        {
                            // eprintln!("Brih");
                            return 0;
                        }
                        for (i, size) in buf.iter().enumerate().take(v23 as usize) {
                            unsafe {
                                *seek_array.add(v43 + i) = seek_pos;
                                seek_pos += *size as u32;
                            }
                        }
                    }
                    v20 += v23;
                    v22 = v24 + v43 as u32;
                    v43 += v24 as usize;

                    if v20 >= seek_table_size as u32 {
                        break;
                    }
                }
                unsafe { *seek_array.add(seek_table_size as usize) = seek_pos };
            } else {
                unsafe { *seek_array.add(seek_table_size as usize) = 0 };
            }

            // Populate half chan data
            // BinkA MSS decoder uses one REAL decoder per 2 channels
            // Idk if this is the part of Bink video too
            data_header.decoders_num = decoders_num;
            let decoders_start = unsafe {
                data.as_mut_ptr()
                    .add(std::mem::size_of::<BinkA2ClassHeader>())
                    .cast::<u8>()
            };
            let mut decoder_ptr = decoders_start;
            for i in 0..decoders_num as usize {
                let decoder = unsafe { &mut *decoder_ptr.cast::<BinkA2DecoderInternal>() };
                data_header.decoders_byte[i] = decoder_chan_num[i];
                data_header.decoders[i] = init_decoder(
                    decoder,
                    header.sample_rate as u32,
                    decoder_chan_num[i] as u16,
                    if header.is_new_codec != 0 {
                        BINKA2_FLAG_V2 | BINKA2_FLAG_NOT_ONE_CHAN | BINKA2_FLAG_DCT
                    } else {
                        BINKA2_FLAG_NOT_ONE_CHAN | BINKA2_FLAG_DCT
                    },
                )
                .cast();

                decoder_ptr = unsafe { decoder_ptr.add(decoder_alloc_sizes[i] as usize) };
            }

            if seek_table_size != 0 {
                2
            } else {
                1
            }
        }
    }

    /// Resets start frame to 1 for all internal decoders
    pub fn reset_start_frame(&self, allocd: &mut [u8]) {
        let data = unsafe { &*(allocd.as_mut_ptr() as *mut BinkA2ClassHeader) };
        let size = data.decoders_num as usize;
        if size != 0 {
            unsafe {
                // let oof = std::slice::from_raw_parts::<*mut u8>(
                //     u64::from_le_bytes(allocd[40..48].try_into().unwrap()) as *mut _,
                //     size as usize,
                // );
                for i in &data.decoders[0..size] {
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
                    header.total_size - v24 as u32 - min_size as u32
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
        let header = unsafe { &*(allocd.as_ptr() as *const BinkA2ClassHeader) };
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
            header.total_size - b
        };

        BinkA2Seek {
            pos: u32::from_le_bytes(allocd[28..32].try_into().unwrap()) + b,
            unk,
            block_size,
        }
    }

    fn get_block_size_detail(&self, allocd: &mut [u8], streaming_data: &[u8]) -> BinkA2Block {
        self.reset_start_frame(allocd);

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

    // Meat of the show
    pub fn decode(
        &self,
        allocd: &mut [u8],
        streaming_data: &[u8],
        out_data: &mut [u16],
    ) -> BinkA2Decode {
        let data = unsafe { &*(allocd.as_mut_ptr() as *mut BinkA2ClassHeader) };
        const BAD: BinkA2Decode = BinkA2Decode {
            consumed: 4,
            samples: 0,
        };

        if streaming_data.len() < 4
            || u16::from_le_bytes([streaming_data[0], streaming_data[1]]) != 0x9999
        {
            return BAD;
        }

        let v12 = u16::from_le_bytes([streaming_data[2], streaming_data[3]]);
        let (v12, consumed) = if v12 == 0xFFFF {
            if streaming_data.len() < 8 {
                return BAD;
            }
            (
                u16::from_le_bytes([streaming_data[4], streaming_data[5]]),
                8,
            )
        } else {
            (v12, 4)
        };

        if v12 > data.max_block_size {
            return BAD;
        }

        let v12 = v12 as usize;

        if (v12 + consumed) > streaming_data.len() {
            return BAD;
        } else if (v12 + consumed) != streaming_data.len() {
            let pos = v12 + consumed;
            if (pos + 2) <= streaming_data.len()
                && u16::from_le_bytes([streaming_data[pos], streaming_data[pos + 1]]) != 0x9999
            {
                return BAD;
            }
        };

        if let Some(ret) = mss_decode(allocd, streaming_data, out_data) {
            ret
        } else {
            // Bad
            BAD
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

        let header = unsafe { &*(allocd.as_ptr() as *const BinkA2ClassHeader) };

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

            if size > header.max_block_size {
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
