use std::{
    ffi::c_void,
    fs::File,
    io::{BufReader, Read, Seek, SeekFrom},
};

use crate::binka::{BinkA2ClassHeader, BinkA2DecoderInternal};

mod binka;
mod util;

#[allow(dead_code)]
extern "fastcall" fn read_callback(out: *mut c_void, size: usize, class: *mut c_void) -> usize {
    unsafe {
        let buf = class.cast::<BufReader<File>>().as_mut().unwrap();
        let pos = buf.stream_position().unwrap();
        let mut tmp = vec![0u8; size];
        if let Ok(read) = buf.read(&mut tmp) {
            eprintln!(
                "[read_callback] Reading from 0x{:X} - {} - {}",
                pos, size, read
            );
            out.cast::<u8>().copy_from(tmp.as_ptr(), read);
            read
        } else {
            0
        }
    }
}

#[allow(dead_code)]
extern "fastcall" fn read_callback_old(class: *mut c_void, out: *mut c_void, size: usize) -> usize {
    unsafe {
        let buf = class.cast::<BufReader<File>>().as_mut().unwrap();
        let mut tmp = vec![0u8; size];
        let pos = buf.stream_position().unwrap();
        if let Ok(read) = buf.read(&mut tmp) {
            eprintln!(
                "[read_callback_old] Reading from 0x{:X} - {} - {}",
                pos, size, read
            );
            out.cast::<u8>().copy_from(tmp.as_ptr(), read);
            read
        } else {
            0
        }
    }
}

fn main() {
    if std::env::args().nth(1).is_none() {
        println!("Invalid usage! mstr [a/2/o]");
    }
    match std::env::args()
        .nth(2)
        .unwrap_or_else(|| "a".to_owned())
        .as_str()
    {
        "o" => main_old(),
        "2" => main_2019(),
        _ => main_new(), // Ironically - only this really works
    }
}

fn main_2019() {
    eprintln!("BROKEN TF|2 IMPL! NEED TO REFACTOR READER TO TAKE INTO A COUNT HEADER DATA!");
    if let Some(binka) = util::lla("binkawin64_2019.dll") {
        println!("DLL: {:?}", binka);
        let binka = util::get_decoder(binka)
            .unwrap()
            .cast::<binka::CBinkA2_2019>();
        let decoder_rust = binka::BinkA2::new(std::ptr::null());
        println!("{:#?}", binka);
        let file = File::open(std::env::args().nth(1).unwrap()).unwrap();
        let mut cursor = BufReader::new(file);
        cursor.seek(SeekFrom::Start(0x20)).unwrap();
        let mut data = [0u8; 24];
        cursor.read_exact(&mut data).unwrap();

        if let Some(metadata) = decoder_rust.parse_metadata(&data) {
            println!("R: {:#?}\n", metadata);

            let mut channels = 0u16;
            let mut samplerate = 0u32;
            let mut samples_count = 0u32;
            let mut adw4 = [0u32; 4];
            unsafe {
                ((*binka).parse_metadata)(
                    data.as_ptr() as *const _,
                    data.len(),
                    (&mut channels) as *mut _,
                    (&mut samplerate) as *mut _,
                    (&mut samples_count) as *mut _,
                    adw4.as_mut_ptr(),
                );
            }
            // 7296 != 704, bruh
            // assert_eq!(adw4[0], metadata.alloc_size);
            eprintln!("[2019] {} - {}", adw4[0], metadata.alloc_size);

            let mut allocd = vec![0u8; adw4[0] as usize];
            cursor.seek(SeekFrom::Start(0x20)).unwrap();
            unsafe {
                println!(
                    "[2019] open_stream - {}",
                    ((*binka).open_stream)(
                        allocd.as_mut_ptr() as *mut _,
                        std::ptr::null_mut(),
                        read_callback,
                        (&mut cursor) as *mut _ as *mut _,
                    )
                );
                {
                    let mut a3 = 0u32;
                    let mut a4 = 0u32;
                    println!(
                        "[2019] get_sample_byte_pos - {} - {} {}",
                        ((*binka).get_sample_byte_pos)(
                            allocd.as_mut_ptr() as *mut _,
                            0,
                            (&mut a3) as *mut _,
                            (&mut a4) as *mut _
                        ),
                        a3,
                        a4
                    );
                };
                println!(
                    "[2019] reset_byte_pos - {}",
                    ((*binka).reset_byte_pos)(allocd.as_mut_ptr() as *mut _),
                );
                eprintln!(
                    "[2019] dd[4] = {} | {}",
                    u32::from_le_bytes((&allocd[16..20]).try_into().unwrap()),
                    channels
                );

                let mut alloc_2 = vec![0f32; (channels as usize) * 64];
                let mut bruh = Vec::<u8>::with_capacity(4 * samples_count as usize);

                for _ in (0..samples_count).step_by(64) {
                    let ret = ((*binka).decode)(
                        allocd.as_mut_ptr() as *mut _,
                        alloc_2.as_mut_ptr(),
                        64,
                        64,
                        read_callback,
                        (&mut cursor) as *mut _ as *mut _,
                    );
                    // let buf = &alloc_2[0..ret * channels as usize];
                    let buf = &alloc_2[..];
                    println!("[2019] {} {:X?}", ret, buf);

                    // for i in buf {
                    //     // let val = (*i) * 32768f32;
                    //     // let val = (val as i16).min(32767);
                    //     // let val = val.max(-32767);
                    //     let val = *i;
                    //     bruh.extend_from_slice(&val.to_le_bytes());
                    // }
                    if !buf.is_empty() {
                        debug_assert_eq!(buf.len(), alloc_2.len());
                        let mut out = Vec::<f32>::with_capacity(buf.len());
                        for j in 0..4 {
                            for i in 0..16 {
                                for chan in 0..channels as usize {
                                    out.push(buf[(j * 16) + i + (chan * 64)]);
                                }
                            }
                        }
                        debug_assert_eq!(out.len(), alloc_2.len());
                        for i in out {
                            bruh.extend_from_slice(&i.to_le_bytes());
                        }
                    }

                    if ret != 64 {
                        break;
                    }
                }

                std::fs::write("bruh.raw", bruh).unwrap();
            }
        } else {
            unreachable!("Failed to parse metadata using Rust's impl!")
        }
    } else {
        unreachable!("Failed to load 2019 Apex's binkawin64(_2019).dll!")
    }
}

fn main_old() {
    eprintln!("BROKEN S3 IMPL! NEED TO REFACTOR READER TO TAKE INTO A COUNT HEADER DATA!");
    if let Some(binka) = util::lla("binkawin64_old.dll") {
        println!("DLL: {:?}", binka);
        let binka = util::get_decoder(binka)
            .unwrap()
            .cast::<binka::CBinkA2_old>();
        let decoder_rust = binka::BinkA2::new(std::ptr::null());
        println!("{:#?}", binka);
        let file = File::open(std::env::args().nth(1).unwrap()).unwrap();
        let mut cursor = BufReader::new(file);
        cursor.seek(SeekFrom::Start(0x20)).unwrap();
        let mut data = [0u8; 24];
        cursor.read_exact(&mut data).unwrap();

        if let Some(metadata) = decoder_rust.parse_metadata(&data) {
            println!("R: {:#?}\n", metadata);

            let mut channels = 0u16;
            let mut samplerate = 0u32;
            let mut samples_count = 0u32;
            let mut adw4 = [0u32; 4];
            unsafe {
                ((*binka).parse_metadata)(
                    data.as_ptr() as *const _,
                    data.len(),
                    (&mut channels) as *mut _,
                    (&mut samplerate) as *mut _,
                    (&mut samples_count) as *mut _,
                    adw4.as_mut_ptr(),
                );
            }
            eprintln!("[OLD] {} - {}", adw4[0], metadata.alloc_size);

            let mut allocd = vec![0u8; adw4[0] as usize];
            cursor.seek(SeekFrom::Start(0x20)).unwrap();
            unsafe {
                println!(
                    "[OLD] open_stream - {}",
                    ((*binka).open_stream)(
                        (&mut cursor) as *mut _ as *mut _,
                        allocd.as_mut_ptr() as *mut _,
                        std::ptr::null_mut(),
                        read_callback_old,
                    )
                );
                {
                    let mut a3 = 0u32;
                    let mut a4 = 0u32;
                    println!(
                        "[OLD] get_sample_byte_pos - {} - {} {}",
                        ((*binka).get_sample_byte_pos)(
                            allocd.as_mut_ptr() as *mut _,
                            0,
                            (&mut a3) as *mut _,
                            (&mut a4) as *mut _
                        ),
                        a3,
                        a4
                    );
                };
                println!(
                    "[OLD] reset_byte_pos - {}",
                    ((*binka).reset_byte_pos)(allocd.as_mut_ptr() as *mut _),
                );

                let alloc_2 = (metadata.channels as usize) << 8;
                println!("[OLD] {} - {}", alloc_2, alloc_2 / 4);
                let mut alloc_2 = vec![0f32; alloc_2 / 4];

                #[allow(clippy::never_loop)]
                loop {
                    let ret = ((*binka).decode)(
                        (&mut cursor) as *mut _ as *mut _,
                        allocd.as_mut_ptr() as *mut _,
                        alloc_2.as_mut_ptr(),
                        64,
                        read_callback_old,
                    );
                    println!("[OLD] {} {:X?}", ret, alloc_2);
                    break;
                }
            }
        } else {
            unreachable!("Failed to parse metadata using Rust's impl!")
        }
    } else {
        unreachable!("Failed to load TF|2's binkawin64(_old).dll!")
    }
}

fn main_new() {
    if let Some(binka) = util::lla("binkawin64.dll") {
        println!("DLL: {:?}", binka);
        let decoder =
            binka::BinkA2::new(util::get_decoder(binka).unwrap().cast::<binka::CBinkA2>());
        println!("{:#?}", decoder);

        // const DATA: [u8; 24] = [
        //     0x31, 0x46, 0x43, 0x42, 0x02, 0x01, 0x80, 0xBB, 0x30, 0xDE, 0x03, 0x00, 0xF0, 0x02,
        //     0x00, 0x00, 0x7E, 0xF1, 0x00, 0x00, 0x85, 0x00, 0x01, 0x00,
        // ];
        let file = File::open(std::env::args().nth(1).unwrap()).unwrap();
        // TODO: unhardcode...
        const START: u64 = 0x20;
        const HEADER_SIZE: usize = 0x830; // must be gotten from MBNK...
        let mut cursor = BufReader::new(file);
        let magic = {
            let mut ret = [0u8; 4];
            cursor.read_exact(&mut ret).unwrap();
            ret
        };
        if magic != [0x52, 0x54, 0x53, 0x43] {
            unreachable!("Invalid streaming file!");
        }
        cursor.seek(SeekFrom::Start(8)).unwrap();
        let streaming_offset = {
            let mut ret = [0u8; 4];
            cursor.read_exact(&mut ret).unwrap();
            u32::from_le_bytes(ret) as u64
        };
        cursor.seek(SeekFrom::Start(START)).unwrap();
        let mut data = [0u8; HEADER_SIZE];
        cursor.read_exact(&mut data).unwrap();
        if let Some(metadata) = decoder.parse_metadata_c(&data) {
            println!(
                "C: {:#?}\nR: {:#?}\n",
                metadata,
                decoder.parse_metadata(&data).unwrap()
            );

            // order of being called is as follows
            // func before decoder (we actually skipped this one?)
            // get metadata
            // open stream
            // get sample 0th byte position (required seek MSS does to skip true header in header-ish data)
            // reset byte/seek pos (wtf is this order)
            // -- LOOP STARTS --
            // func after the decoder to get required bytes and block sizes?
            // if we have enough bytes - decode until we don't

            // func before decoder...
            // wtf? return is identical to get_sample_byte_pos...
            let seek_data_ret = decoder.get_seek_pos_data_c(&data, 0);
            println!("get_seek_pos_data - {:?}", seek_data_ret,);
            debug_assert_eq!(seek_data_ret, binka::BinkA2::get_seek_pos_data(&data, 0));

            let mut allocd = vec![0u8; metadata.alloc_size as usize];
            cursor.seek(SeekFrom::Start(START)).unwrap();
            let tmp_ret = decoder.open_stream_c(
                &mut allocd,
                read_callback,
                (&mut cursor) as *mut _ as *mut c_void,
            );
            debug_assert_ne!(tmp_ret, 0, "open_stream failed so math table didn't init!");
            cursor.seek(SeekFrom::Start(START)).unwrap();
            println!(
                "open_stream - {}",
                decoder.open_stream(
                    &mut allocd,
                    read_callback,
                    (&mut cursor) as *mut _ as *mut c_void
                )
            );
            unsafe {
                let header = &*allocd.as_ptr().cast::<BinkA2ClassHeader>();
                let decoder = &*header.decoders[0].cast::<BinkA2DecoderInternal>();
                eprintln!("Decoder: {:?}", decoder);
            };

            let seek_shit = decoder.get_sample_byte_pos_c(&mut allocd, 0);
            println!("get_sample_byte_pos - {:?}", seek_shit);
            debug_assert_eq!(seek_shit, decoder.get_sample_byte_pos(&allocd, 0));
            // this is pointer dependant which we obv loose and can't clone at all
            println!(
                "reset_start_frame - {:?} - {}",
                decoder.reset_start_frame(&mut allocd),
                allocd[32],
            );

            // let allocd_header = unsafe { &*(allocd.as_ptr() as *mut BinkA2ClassHeader) };

            cursor.seek(SeekFrom::Start(streaming_offset)).unwrap();
            // let mut streaming_data_real = vec![0u8; allocd_header.total_size as usize - HEADER_SIZE + 8];
            let mut streaming_data_real = vec![0u8; metadata.samples_count as usize];
            cursor.read_exact(&mut streaming_data_real).unwrap();
            let streaming_data_brih =
                [&data[seek_shit.pos as usize..], &streaming_data_real[..]].concat();

            let mut alloc_2 = vec![0u16; 8192 * metadata.channels as usize];
            let mut streaming_data = &streaming_data_brih[..]; //&data[seek_shit.0 as usize..]; // &mut streaming_data_real[..];
            let max_size = metadata.samples_count as usize * 2 * metadata.channels as usize;
            let mut bruh = Vec::<u8>::with_capacity(max_size);
            loop {
                let mut allocd_clone = allocd.clone();
                let unk38 = decoder.get_block_size_c(&mut allocd, streaming_data);
                debug_assert_eq!(
                    unk38,
                    decoder.get_block_size(&mut allocd_clone, streaming_data)
                );
                if unk38.consumed == 65535 {
                    println!(
                        "Reached the end of the line! 0xFFFF {:?} | {}",
                        unk38,
                        streaming_data.len()
                    );
                    break;
                }
                streaming_data = &streaming_data[unk38.consumed as usize..];
                println!("get_block_size - {:?}", unk38);
                if (unk38.consumed + unk38.reported_block_size) as usize > streaming_data.len() {
                    println!(
                        "Reached the end of the line! {:?} | {}",
                        unk38,
                        streaming_data.len()
                    );
                    break;
                }
                alloc_2.fill(0);
                // let alloc_2_clone = alloc_2.clone();
                let decode_ret = decoder.decode(&mut allocd, streaming_data, &mut alloc_2);
                println!("decode - {:?}", decode_ret);
                // alloc_2[..].copy_from_slice(&alloc_2_clone);
                // debug_assert_eq!(
                //     decode_ret,
                //     decoder.decode(&mut allocd, streaming_data, &mut alloc_2)
                // );
                // debug_assert_eq!(
                //     &alloc_2_clone[..decode_ret.samples as usize * metadata.channels as usize],
                //     &alloc_2[..decode_ret.samples as usize * metadata.channels as usize],
                // );
                if decode_ret.samples != 0 {
                    // debug_assert_eq!((decode_ret.1 * metadata.channels as u32) as usize, alloc_2.len());
                    debug_assert_eq!(
                        decode_ret.consumed, unk38.reported_block_size,
                        "consumed bytes != reported block size"
                    );
                }
                let decoded = &alloc_2[..decode_ret.samples as usize * metadata.channels as usize];
                println!("{:X?}", decoded);
                for i in decoded {
                    bruh.extend_from_slice(&i.to_le_bytes());
                    if bruh.len() == bruh.capacity() {
                        break;
                    }
                }
                if bruh.len() >= max_size {
                    println!("Not going overboard! {} >= {}", bruh.len(), max_size);
                    debug_assert_eq!(bruh.len(), max_size);
                    break;
                }
                streaming_data = &streaming_data[decode_ret.consumed as usize..];
            }
            std::fs::write("brih.raw", &bruh).unwrap();
        } else {
            debug_assert!(decoder.parse_metadata(&data).is_none());
            eprintln!("Failed to load parse metadata!")
        }
    } else {
        eprintln!("failed to load binkawin64.dll!");
    }
}
