use std::{
    ffi::c_void,
    fs::File,
    io::{BufReader, Read, Seek, SeekFrom},
};

mod binka;
mod util;

#[allow(dead_code)]
extern "fastcall" fn read_callback(out: *mut c_void, size: usize, class: *mut c_void) -> usize {
    unsafe {
        let buf = class.cast::<BufReader<File>>().as_mut().unwrap();
        let mut tmp = vec![0u8; size];
        let pos = buf.stream_position().unwrap();
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
    if std::env::args().len() > 1 {
        if std::env::args().nth(1).unwrap() == "2" {
            main_old()
        } else {
            main_2019()
        }
    } else {
        main_new()
    }
}

fn main_2019() {
    if let Some(binka) = util::lla("binkawin64_2019.dll") {
        println!("DLL: {:?}", binka);
        let binka = util::get_decoder(binka)
            .unwrap()
            .cast::<binka::CBinkA2_2019>();
        let decoder_rust = binka::BinkA2::new(std::ptr::null());
        println!("{:#?}", binka);
        let file = File::open(
            "D:\\SteamLibrary\\steamapps\\common\\Apex Legends\\audio\\ship\\general_stream.mstr",
        )
        .unwrap();
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
            eprintln!("[OLD] {} - {}", adw4[0], metadata.alloc_size);

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
                        "[2019] unk20 - {} - {} {}",
                        ((*binka).unk20)(
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
                    "[2019] unk18 - {}",
                    ((*binka).unk18)(allocd.as_mut_ptr() as *mut _),
                );

                let alloc_2 = (metadata.channels as usize) << 8;
                println!("[2019] {} - {}", alloc_2, alloc_2 / 4);
                let mut alloc_2 = vec![0u32; alloc_2 / 4];
                loop {
                    let ret = ((*binka).decode)(
                        allocd.as_mut_ptr() as *mut _,
                        alloc_2.as_mut_ptr(),
                        64,
                        64,
                        read_callback,
                        (&mut cursor) as *mut _ as *mut _,
                    );
                    println!("[2019] {} {:X?}", ret, alloc_2);
                    break;
                }
            }
        } else {
            unreachable!("Failed to parse metadata using Rust's impl!")
        }
    } else {
        unreachable!("Failed to load 2019 Apex's binkawin64(_2019).dll!")
    }
}

fn main_old() {
    if let Some(binka) = util::lla("binkawin64_old.dll") {
        println!("DLL: {:?}", binka);
        let binka = util::get_decoder(binka)
            .unwrap()
            .cast::<binka::CBinkA2_old>();
        let decoder_rust = binka::BinkA2::new(std::ptr::null());
        println!("{:#?}", binka);
        let file = File::open(
            "D:\\SteamLibrary\\steamapps\\common\\Apex Legends\\audio\\ship\\general_stream.mstr",
        )
        .unwrap();
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
                        "[OLD] unk20 - {} - {} {}",
                        ((*binka).unk20)(
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
                    "[OLD] unk18 - {}",
                    ((*binka).unk18)(allocd.as_mut_ptr() as *mut _),
                );

                let alloc_2 = (metadata.channels as usize) << 8;
                println!("[OLD] {} - {}", alloc_2, alloc_2 / 4);
                let mut alloc_2 = vec![0u32; alloc_2 / 4];
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
        let file = File::open(
            "D:\\SteamLibrary\\steamapps\\common\\Apex Legends\\audio\\ship\\general_stream.mstr",
        )
        .unwrap();
        let mut cursor = BufReader::new(file);
        cursor.seek(SeekFrom::Start(0x20)).unwrap();
        let mut data = [0u8; 24];
        cursor.read_exact(&mut data).unwrap();
        if let Some(metadata) = decoder.parse_metadata_c(&data) {
            println!(
                "C: {:#?}\nR: {:#?}\n",
                metadata,
                decoder.parse_metadata(&data).unwrap()
            );

            let mut allocd = vec![0u8; metadata.alloc_size as usize];
            cursor.seek(SeekFrom::Start(0x20)).unwrap();
            println!(
                "open_stream - {}",
                decoder.open_stream_c(
                    &mut allocd,
                    read_callback,
                    (&mut cursor) as *mut _ as *mut c_void
                )
            );
            // println!("0x{:X}", u32::from_le_bytes((&allocd[16..20]).try_into().unwrap()));
            println!("unk20 - {:?}", decoder.unk20_c(&mut allocd, 0));
            println!("unk18 - {:?}", decoder.unk18_c(&mut allocd));

            let alloc_2 = (metadata.channels as usize) << 8;
            println!("{} - {}", alloc_2, alloc_2 / 4);
            let alloc_2 = vec![0u32; alloc_2 / 4];
            loop {
                // wtf, the function which did streaming by itself died _(
                println!("{:X?}", alloc_2);
                break;
            }
        } else {
            debug_assert!(decoder.parse_metadata(&data).is_none());
            eprintln!("Failed to load parse metadata!")
        }
    } else {
        eprintln!("failed to load binkawin64.dll!");
    }
}
