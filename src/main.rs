use std::{
    ffi::c_void,
    fs::File,
    io::{BufReader, Read, Seek, SeekFrom},
};

mod binka;
mod util;

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

fn main() {
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
        let mut DATA = [0u8; 24];
        cursor.read_exact(&mut DATA).unwrap();
        if let Some(metadata) = decoder.parse_metadata_c(&DATA) {
            println!(
                "C: {:#?}\nR: {:#?}\n",
                metadata,
                decoder.parse_metadata(&DATA).unwrap()
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
            println!("{} - {}", alloc_2, alloc_2/4);
            let alloc_2 = vec![0u32; alloc_2/4];
            loop {
                // wtf, the function which did streaming by itself died _(
                println!("{:X?}", alloc_2);
                break
            }
        } else {
            debug_assert!(decoder.parse_metadata(&DATA).is_none());
            eprintln!("Failed to load parse metadata!")
        }
    } else {
        eprintln!("failed to load binkawin64.dll!");
    }
}
