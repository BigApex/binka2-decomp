use std::ffi::c_void;

pub type FnCbRead = extern "fastcall" fn(
    out: *mut c_void,
    size: usize,
    class: *mut c_void, // sizeof = 0x120 = 288
) -> usize;

pub type FnCbReadOld =
    extern "fastcall" fn(class: *mut c_void, out: *mut c_void, size: usize) -> usize;

pub type FnParseMetadata = extern "fastcall" fn(
    data: *const c_void,
    size: usize,
    out_channels: *mut u16,
    out_samplerate: *mut u32,
    out_samples_count: *mut u32,
    adw4: *mut u32,
) -> u64;
pub type FnOpenStream = extern "fastcall" fn(
    data: *mut c_void,
    _aqw4: *mut u64, // unused in BinkA2?
    callback: FnCbRead,
    class: *mut u64, // sizeof = 0x120 = 288
) -> i64;
pub type FnResetBytePos = extern "fastcall" fn(data: *mut c_void) -> u8;
pub type FnGetSampleBytePos = extern "fastcall" fn(
    data: *mut c_void, // might be const
    a2: u32,
    a3: *mut u32,
    a4: *mut u32,
) -> u32;
pub type FnGetSeekPosData = extern "fastcall" fn(
    header: *const c_void,
    header_size: usize,
    a3: u32,
    out1: *mut u32,
    out2: *mut u32,
) -> u64;
pub type FnDecode = extern "fastcall" fn(
    allocd: *mut c_void,
    stream_data: *const c_void, // yeah, in theory shouldn't get mutated
    stream_data_size: usize,    // u32 in reality, idk
    out_data: *mut c_void,      // idk how size is calculated, amma be real
    a5: u32,                    // or i32, idk, might be the size of the out_data
    consumed: *mut u32,
    out2: *mut u32, // if(out2) -> assert(consumed <= reported (aka prev_out2))
) -> usize; // might be void
pub type FnGetBlockSize = extern "fastcall" fn(
    allocd: *mut c_void,
    stream_data: *const c_void,
    stream_data_size: usize, // u32 in reality, idk
    out1: *mut u32,
    reported_block_size: *mut u32,
    out3: *mut u32,
);

pub type FnOpenStreamOld = extern "fastcall" fn(
    class: *mut u64,
    data: *mut c_void,
    _aqw4: *mut u64, // unused in BinkA2?
    callback: FnCbReadOld,
) -> i64;
pub type FnDecoderOld = extern "fastcall" fn(
    class: *mut c_void,
    data: *mut c_void,
    decoded: *mut f32,
    size: usize,
    cb: FnCbReadOld,
) -> usize;

pub type FnDecoderApex2019 = extern "fastcall" fn(
    data: *mut c_void,
    decoded: *mut f32,
    size: usize,
    size2: usize,
    cb: FnCbRead,
    class: *mut c_void,
) -> usize;
