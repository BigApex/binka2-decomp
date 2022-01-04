mod c_impl;
mod consts;
pub mod structs;
mod types;

use std::usize;

pub use c_impl::*;
pub use consts::*;
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
                max_stream_size: max_stream_size,
                frame_len,
            })
        }
    }

    pub fn open_stream<T>(&self, data: &mut [u8], cb: FnCbRead, class: *mut T) -> u64 {
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
            data_header.unk_e = header.unk_e;
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

            let (decoder_alloc_sizes, decoder_alloc_size_total) = get_decoder_alloc_size_channel_num(
                header.channels as u16,
                header.sample_rate as u32,
            );
            debug_assert_eq!(decoder_alloc_size_total, get_total_decoders_alloc_size(header.channels as u16, header.sample_rate as u32));
            let decoder_alloc_size_samples_round = (decoder_alloc_size_total + 128 + 63) & 0xFFFFFFC0;

            let seek_array = unsafe {
                let seek_array = data.as_mut_ptr().add(decoder_alloc_size_samples_round as usize);
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
                        for i in 0..v23 as usize {
                            unsafe {
                                *seek_array.add(v43 + i) = seek_pos;
                                seek_pos += buf[i] as u32;
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
                    if header.unk_e != 0 {
                        BINKA2_FLAG_V2 | BINKA2_FLAG_NOT_ONE_CHAN | BINKA2_FLAG_IDK
                    } else {
                        BINKA2_FLAG_NOT_ONE_CHAN | BINKA2_FLAG_IDK
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
        let size = data.decoders_num;
        if size != 0 {
            unsafe {
                // let oof = std::slice::from_raw_parts::<*mut u8>(
                //     u64::from_le_bytes(allocd[40..48].try_into().unwrap()) as *mut _,
                //     size as usize,
                // );
                for i in &data.decoders {
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
        // TODO: swap for rust impl
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

fn get_channel_num_decoders(channels: u16) -> Vec<u8> {
    let half_channel = (channels + 1) / 2;
    if half_channel > 0 {
        (0..half_channel)
            .map(|i| {
                2 - if channels.wrapping_sub(i * 2) != 0 {
                    1
                } else {
                    0
                }
            })
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    }
}

fn get_total_decoders_alloc_size(channels: u16, samplerate: u32) -> u32 {
    let half_channel = (channels + 1) / 2;
    let alloc_size_samples = if half_channel != 0 {
        (0..half_channel)
            .map(|i| {
                // TODO: are names even correct?
                let chan_id = i * 2;
                let a2 = 2 - if channels.wrapping_sub(chan_id) != 0 {
                    1
                } else {
                    0
                };
                int_calc(samplerate, a2)
            })
            .reduce(|accum: u32, i| accum.wrapping_add(i))
            .unwrap() // Should never panic?
    } else {
        0
    };
    alloc_size_samples
}

fn get_decoder_alloc_size_channel_num(channels: u16, samplerate: u32) -> (Vec<u32>, u32) {
    let half_channel = (channels + 1) / 2;
    let buf = (0..half_channel)
        .map(|i| {
            // TODO: are names even correct?
            let chan_id = i * 2;
            let a2 = 2 - if channels.wrapping_sub(chan_id) != 0 {
                1
            } else {
                0
            };
            int_calc(samplerate, a2)
        })
        .collect::<Vec<_>>();

    if buf.is_empty() {
        (vec![0], 0)
    } else {
        let mut accum = 0;
        let mut ret = Vec::with_capacity(half_channel as usize);
        for i in buf {
            ret.push(i);
            accum += i;
        }
        (ret, accum)
    }
}

fn int_calc(samplerate: u32, channels: u32) -> u32 {
    if samplerate >= 44100 {
        (channels.wrapping_mul(1 << 8) & 0xFFFFFFF).wrapping_add(160 + 15) & 0xFFFFFFF0
    } else {
        let mul = if samplerate >= 22050 { 1024 } else { 512 };
        (channels.wrapping_mul(2).wrapping_mul(mul) >> 4).wrapping_add(160 + 15) & 0xFFFFFFF0
    }
}

fn init_decoder(
    decoder: &mut BinkA2DecoderInternal,
    sample_rate: u32,
    channels: u16,
    flags: u32,
) -> *mut BinkA2DecoderInternal {
    debug_assert!(channels <= 2, "More than 2 channels for internal decoder!");

    let ptr = decoder as *mut BinkA2DecoderInternal;
    unsafe {
        std::slice::from_raw_parts_mut(
            ptr.cast::<u8>(),
            std::mem::size_of::<BinkA2DecoderInternal>(),
        )
        .fill(0)
    };

    let (transform_size, transform_big, transform_small) = if sample_rate < 44100 {
        if sample_rate < 22050 {
            BINKA2_TRANSFORMS[2]
        } else {
            BINKA2_TRANSFORMS[1]
        }
    } else {
        BINKA2_TRANSFORMS[0]
    };
    debug_assert!(
        transform_big > transform_small,
        "{} <= {}",
        transform_big,
        transform_small
    );

    let unk10 = 2 * channels as u32 * transform_size;

    let flags = if (flags & BINKA2_FLAG_V2) != 0 {
        flags | BINKA2_FLAG_IDK
    } else {
        flags
    };

    let (channels, transform_ratio, transform_size, sample_rate) = if (flags & BINKA2_FLAG_IDK) == 0
    {
        let transform_ratio = if channels == 2 {
            transform_big
        } else {
            transform_small
        };
        (
            1,
            transform_ratio,
            transform_size * channels as u32,
            sample_rate * channels as u32,
        )
    } else {
        (channels, transform_small, transform_size, sample_rate)
    };

    if transform_size > 2048 {
        return std::ptr::null_mut();
    }

    let half_rate = (sample_rate + 1) / 2;
    let bands_num = BINKA2_CRIT_FREQS
        .iter()
        .position(|freq| *freq >= half_rate)
        .unwrap_or(BINKA2_CRIT_FREQS.len());
    // debug_assert_ne!(bands_num, BINKA2_CRIT_FREQS.len(), "Ideally this should never happen");

    decoder.ptr = unsafe { ptr.add(1).cast() };
    decoder.unk10 = unk10;
    decoder.unk14 = unk10 >> 4;
    decoder.size = (decoder.unk14 + 160 + 15) & 0xFFFFFFF0;
    decoder.channels = channels as u32;
    decoder.bands_num = bands_num as u32;
    decoder.transform_size = transform_size;
    decoder.transform_ratio = transform_ratio;
    decoder.bits_shift = match decoder.unk14 {
        512 => 8,
        256 => 7,
        128 => 6,
        64 => 5,
        _ => unreachable!("Invalid unk14 {}", decoder.unk14),
    };

    let transform_size_half = transform_size / 2;
    for i in 0..bands_num {
        let band = (BINKA2_CRIT_FREQS[i] * transform_size_half) / half_rate;
        decoder.bands[i] = if band != 0 { band } else { 1 }
    }
    decoder.bands[bands_num] = transform_size_half;

    decoder.start_frame = 1;
    decoder.flags = if channels == 1 {
        flags & (!BINKA2_FLAG_NOT_ONE_CHAN)
    } else {
        flags
    };

    ptr
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
