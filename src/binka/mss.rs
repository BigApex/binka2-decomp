#![allow(clippy::too_many_arguments)]

use std::ffi::c_void;

use bitbuffer::{BitReadBuffer, BitReadStream, LittleEndian};

use super::*;

pub fn get_channel_num_decoders(channels: u16) -> Vec<u8> {
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

pub fn get_total_decoders_alloc_size(channels: u16, samplerate: u32) -> u32 {
    let half_channel = (channels + 1) / 2;
    if half_channel != 0 {
        (0..half_channel)
            .map(|i| {
                // TODO: are names even correct?
                let chan_id = i * 2;
                let a2 = 2 - if channels.wrapping_sub(chan_id) != 0 {
                    1
                } else {
                    0
                };
                get_decoder_size(samplerate, a2)
            })
            .reduce(|accum: u32, i| accum.wrapping_add(i))
            .unwrap() // Should never panic?
    } else {
        0
    }
}

pub fn get_decoder_alloc_size_channel_num(channels: u16, samplerate: u32) -> (Vec<u32>, u32) {
    let half_channel = (channels + 1) / 2;
    let buf = (0..half_channel)
        .map(|i| {
            let chan_id = i * 2;
            let a2 = 2 - if channels.wrapping_sub(chan_id) != 0 {
                1
            } else {
                0
            };
            get_decoder_size(samplerate, a2)
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

pub fn get_decoder_size(samplerate: u32, channels: u32) -> u32 {
    if samplerate >= 44100 {
        (channels.wrapping_mul(1 << 8) & 0xFFFFFFF).wrapping_add(160 + 15) & 0xFFFFFFF0
    } else {
        let mul = if samplerate >= 22050 { 1024 } else { 512 };
        (channels.wrapping_mul(2).wrapping_mul(mul) >> 4).wrapping_add(160 + 15) & 0xFFFFFFF0
    }
}

pub fn init_decoder(
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

    let (transform_size, transform_small, transform_big) = if sample_rate < 44100 {
        if sample_rate < 22050 {
            BINKA2_TRANSFORMS[2]
        } else {
            BINKA2_TRANSFORMS[1]
        }
    } else {
        BINKA2_TRANSFORMS[0]
    };
    debug_assert!(
        transform_small > transform_big,
        "{} <= {}",
        transform_small,
        transform_big
    );

    let unk10 = 2 * channels as u32 * transform_size;

    let flags = if (flags & BINKA2_FLAG_V2) != 0 {
        flags | BINKA2_FLAG_DCT
    } else {
        flags
    };

    let (channels, transform_ratio, transform_size, sample_rate) = if (flags & BINKA2_FLAG_DCT) == 0
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
    for (i, freq) in BINKA2_CRIT_FREQS.iter().enumerate().take(bands_num) {
        let band = (freq * transform_size_half) / half_rate;
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

pub fn mss_decode(
    allocd: &mut [u8],
    streaming_data: &[u8],
    out_data: &mut [u16],
) -> Option<BinkA2Decode> {
    let data = unsafe { &*(allocd.as_mut_ptr() as *mut BinkA2ClassHeader) };

    let end_pos = u16::from_le_bytes([streaming_data[2], streaming_data[3]]);
    let (end_pos, v6, consumed) = if end_pos == 0xFFFF {
        let end_pos = u16::from_le_bytes([streaming_data[4], streaming_data[5]]);
        let v6 = u16::from_le_bytes([streaming_data[6], streaming_data[7]]);
        (end_pos, v6 as usize, 8)
    } else {
        (end_pos, usize::MAX, 4)
    };

    let mut streaming_cursor = &streaming_data[consumed..end_pos as usize + consumed + 72];
    let mut out_cursor = out_data;
    let mut consumed = consumed;
    let mut decoded = 0;
    for decoder in &data.decoders[0..data.decoders_num as usize] {
        let decoder = unsafe { &mut *decoder.cast::<BinkA2DecoderInternal>() };

        let ret = decoder_decompress(decoder, out_cursor, streaming_cursor);
        consumed += ret.consumed;
        decoded += ret.decoded;

        streaming_cursor = &streaming_cursor[ret.consumed..];
        out_cursor = &mut out_cursor[ret.decoded..];
    }

    if v6 != usize::MAX {
        todo!();
    }

    Some(BinkA2Decode {
        consumed: consumed as u32,
        samples: decoded as u32,
    })
}

pub struct BinkA2DecoderDecompress {
    consumed: usize,
    decoded: usize,
}

/// Returns decoded and consumed bytes
fn decoder_decompress(
    decoder: &mut BinkA2DecoderInternal,
    output: &mut [u16],
    input: &[u8],
) -> BinkA2DecoderDecompress {
    let ret = decode_frame(
        decoder.transform_size,
        decoder.transform_ratio,
        decoder.channels,
        decoder.flags,
        output,
        input,
        &decoder.bands[0..decoder.bands_num as usize + 1],
        decoder.unk14,
        if decoder.start_frame != 0 {
            0
        } else {
            decoder.bits_shift
        },
        decoder.ptr,
    );
    decoder.start_frame = 0;
    ret
}

fn decode_frame(
    transform_size: u32,
    transform_ratio: f32,
    channels: u32,
    flags: u32,
    output: &mut [u16],
    input: &[u8],
    bands: &[u32],
    unk14: u32,
    bits_shift: u32,
    ptr: *mut c_void,
) -> BinkA2DecoderDecompress {
    let bit_buffer = BitReadBuffer::new(input, LittleEndian);
    let mut bitreader = BitReadStream::new(bit_buffer);

    if (flags & BINKA2_FLAG_DCT) != 0 {
        bitreader.skip_bits(2).unwrap();
    }

    debug_assert_ne!(channels, 0);
    debug_assert!(channels <= 2);

    let decoded = if channels == 1 {
        let coeffs = if (flags & BINKA2_FLAG_V2) != 0 {
            todo!("V2 read channel data")
        } else {
            read_channel_1(transform_size, bands, &mut bitreader)
        };

        transform_1(
            flags,
            output,
            &coeffs,
            transform_size,
            transform_ratio,
            ptr,
            unk14,
            bits_shift,
        );

        transform_size - unk14 / 2
    } else {
        todo!("Stereo");
    };

    let consumed_bytes = (bitreader.pos() as usize + 7) / 8;
    BinkA2DecoderDecompress {
        consumed: (consumed_bytes + 3) & 0xFFFFFFFFFFFFFFFC,
        decoded: decoded as usize,
    }
}

fn read_channel_1(
    transform_size: u32,
    bands: &[u32],
    bitreader: &mut BitReadStream<LittleEndian>,
) -> Vec<f32> {
    let mut coeffs = vec![0f32; transform_size as usize];
    coeffs[0] = get_fixed_float(bitreader);
    coeffs[1] = get_fixed_float(bitreader);

    let mut quants = [0f32; BINKA2_BANDS_MAX + 2];
    for i in 0..bands.len() - 1 {
        let idx = bitreader.read_int::<u8>(8).unwrap() as usize;
        let idx = idx.min(BINKA2_QUANTS.len() - 1);
        quants[1 + i] = BINKA2_QUANTS[idx];
    }

    // for i in 2..transform_size {
    // We need to have arbitary control over i
    let mut i = 2;
    while i < transform_size {
        // let peek = bitreader.peek_u32(9).unwrap();
        let (bits_len, end) = if bitreader.read_bool().unwrap() {
            // 1 + 4 + 4 = 9
            // bitreader.skip(9).unwrap();
            let idx = bitreader.read_int::<u32>(4).unwrap();
            (
                bitreader.read_int::<u32>(4).unwrap(),
                i + (BINKA2_RLE[idx as usize] * 8),
            )
        } else {
            // 1 + 4 = 5
            // bitreader.skip(5).unwrap();
            (bitreader.read_int::<u32>(4).unwrap(), i + 8)
        };

        let end = end.min(transform_size);

        let bits_len = bits_len & 0xF;

        // eprintln!("[{}]: {} {} {} | ({}-2) {}", bands.len() - 1, i, bits_len, end, bitreader.pos(), bitreader.bits_left());

        if bits_len == 0 {
            // zero-fill
            coeffs[i as usize..end as usize].fill(0f32);
            i = end;
        } else {
            // let mask = (1 << bits_len) - 1;
            // 1 bit is sign
            let bits_plus_1 = bits_len + 1;

            // if bitreader.remaining() as u32 <= (((end - i) * bits_plus_1) + 1) {
            if bitreader.bits_left() as u32 <= ((end - i) * bits_plus_1) {
                eprintln!("{:#?}", &coeffs[..end as usize]);
                panic!(
                    "what the fuck {} {}",
                    bitreader.bits_left(),
                    (((end - i) * bits_plus_1) + 9)
                );
                // coeffs[i as usize..end as usize].fill(0f32);
                // i = end;
                // break;
            }

            while i < end {
                // should never happen???
                let cur_band = bands.iter().position(|b| i < (2 * (*b))).unwrap();
                let band_end = end.min(bands[cur_band] * 2);

                // let q = [quants[cur_band], -quants[cur_band]];
                // there's also a loop that does a few at a time but who cares

                // for c_pos in i as usize..band_end as usize {
                for ceoff in coeffs.iter_mut().take(band_end as usize).skip(i as usize) {
                    let v = bitreader.read_int::<u32>(bits_len as usize).unwrap();
                    *ceoff = if v == 0 {
                        0f32
                    } else {
                        (v as f32)
                            * if bitreader.read_bool().unwrap() {
                                // 1
                                -quants[cur_band]
                            } else {
                                // 0
                                quants[cur_band]
                            } //q[bitreader.read_u8(1).unwrap() as usize]
                    };
                }

                i = band_end;
            }
        }
    }

    coeffs
}

fn transform_1(
    flags: u32,
    output: &mut [u16],
    coeffs: &[f32],
    transform_size: u32,
    transform_ratio: f32,
    ptr: *mut c_void,
    unk14: u32,
    bits_shift: u32,
) {
    if output.len() < (transform_size - unk14) as usize {
        panic!("Sizes don't match!")
    }

    // eprintln!("{:?}", coeffs);

    if output.len() < transform_size as usize {
        // not enough size for the window
        let mut output_windowed = vec![0u16; transform_size as usize];
        output_windowed[0..output.len()].copy_from_slice(output);
        transform_2(
            flags,
            &mut output_windowed,
            coeffs,
            transform_size,
            transform_ratio,
            ptr,
            unk14,
            bits_shift,
        );
        output.copy_from_slice(&output_windowed[0..output.len()]);
    } else {
        transform_2(
            flags,
            output,
            coeffs,
            transform_size,
            transform_ratio,
            ptr,
            unk14,
            bits_shift,
        );
    }
}

fn transform_2(
    flags: u32,
    output: &mut [u16],
    coeffs: &[f32],
    transform_size: u32,
    transform_ratio: f32,
    ptr: *mut c_void,
    unk14: u32,
    bits_shift: u32,
) {
    // let mut transform = vec![0f32; transform_size as usize];

    if (flags & BINKA2_FLAG_DCT) != 0 {
        // Inverse DCT aka DCT3
        // AND
        // Inverse FFT
        idct(
            output,
            transform_ratio,
            // &mut transform,
            coeffs,
            transform_size,
        );
    } else {
        unreachable!("FFT codec is old AF and should never be used");
    };

    // This is frame blending
    if bits_shift != 0 {
        let ptr_slice =
            unsafe { std::slice::from_raw_parts_mut(ptr.cast::<i16>(), unk14 as usize / 2) };
        for i in 0..unk14 as usize / 2 {
            output[i] = ((((output[i] as i16 as i32)
                .overflowing_sub(ptr_slice[i] as i16 as i32)
                .0
                .overflowing_mul(i as i32)
                .0)
                >> bits_shift).overflowing_add(ptr_slice[i] as i32).0) as i16 as u16;
        }
    }

    if unk14 != 0 {
        unsafe {
            let ptr_from = output
                .as_ptr()
                .add(transform_size as usize)
                .cast::<u8>()
                .sub(unk14 as usize);
            std::slice::from_raw_parts_mut(ptr.cast::<u8>(), unk14 as usize)
                .copy_from_slice(std::slice::from_raw_parts(ptr_from, unk14 as usize));
        }
    }
}

fn get_fixed_float(bitreader: &mut BitReadStream<LittleEndian>) -> f32 {
    // 5 + 23 + 1 = 29
    let power = bitreader.read_int::<u32>(5).unwrap();
    let x = bitreader.read_int::<u32>(23).unwrap();
    let f = (x as f32) * BINKA2_FXP_POW[power as usize]; //(2f32.powi(power as i32 - 23));

    if bitreader.read_bool().unwrap() {
        -f
    } else {
        f
    }
}
