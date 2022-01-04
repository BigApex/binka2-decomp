use super::*;

#[allow(dead_code)]
impl BinkA2 {
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
                    max_stream_size: adw4[1],
                    frame_len: adw4[2],
                })
            }
        }
    }

    pub fn open_stream_c<T>(&self, data: &mut [u8], cb: FnCbRead, class: *mut T) -> i64 {
        unsafe {
            ((*self.binka).open_stream)(
                data.as_mut_ptr() as *mut _,
                std::ptr::null_mut(), // unused
                cb,
                class as *mut u64,
            )
        }
    }

    pub fn get_sample_byte_pos_c(&self, data: &mut [u8], a2: u32) -> BinkA2Seek {
        unsafe {
            let mut a3 = 0u32;
            let mut a4 = 0u32;
            let ret = ((*self.binka).get_sample_byte_pos)(
                data.as_mut_ptr() as *mut _,
                a2,
                (&mut a3) as *mut _,
                (&mut a4) as *mut _,
            );
            BinkA2Seek {
                pos: ret,
                unk: a3,
                block_size: a4,
            }
        }
    }

    pub fn reset_byte_pos_c(&self, data: &mut [u8]) -> u8 {
        unsafe { ((*self.binka).reset_byte_pos)(data.as_mut_ptr() as *mut _) }
    }

    pub fn get_seek_pos_data_c(&self, header: &[u8], a3: u32) -> BinkA2Seek {
        unsafe {
            let mut out = [0u32; 2];
            let ret = ((*self.binka).get_seek_pos_data)(
                header.as_ptr() as *const _,
                header.len(),
                a3,
                &mut out[0] as *mut _,
                &mut out[1] as *mut _,
            );
            BinkA2Seek {
                pos: ret as u32,
                unk: out[0],
                block_size: out[1],
            }
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
}
