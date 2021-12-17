mod binka;
mod util;

fn main() {
    if let Some(binka) = util::lla("binkawin64.dll") {
        println!("DLL: {:?}", binka);
        let decoder =
            binka::BinkA2::new(unsafe { binka.cast::<u8>().add(0x19000).cast::<binka::CBinkA2>() });
        println!("{:#?}", decoder);

        const DATA: [u8; 24] = [
            0x31, 0x46, 0x43, 0x42, 0x02, 0x01, 0x80, 0xBB, 0x30, 0xDE, 0x03, 0x00, 0xF0, 0x02,
            0x00, 0x00, 0x7E, 0xF1, 0x00, 0x00, 0x85, 0x00, 0x01, 0x00,
        ];
        if let Some(metadata) = decoder.parse_metadata_c(&DATA) {
            println!(
                "C: {:#?}\nR: {:#?}",
                metadata,
                decoder.parse_metadata(&DATA).unwrap()
            )
        } else {
            debug_assert!(decoder.parse_metadata(&DATA).is_none());
            eprintln!("Failed to load parse metadata!")
        }
    } else {
        eprintln!("failed to load binkawin64.dll!");
    }
}
