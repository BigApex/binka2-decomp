use std::ffi::c_void;

#[allow(dead_code)]
#[link(name = "kernel32")]
extern "stdcall" {
    fn LoadLibraryA(lpLibFileName: *const u8) -> *const c_void;
    fn FreeLibrary(hLibModule: *const c_void) -> i32;
    fn GetProcAddress(hModule: *const c_void, lpProcName: *const u8) -> *const c_void;
}

pub fn lla(dll: &str) -> Option<*const c_void> {
    let name = [dll.as_bytes(), &[0u8]].concat();
    unsafe {
        let h = LoadLibraryA(name.as_ptr());

        if h.is_null() {
            None
        } else {
            Some(h)
        }
    }
}

pub fn gpa(h: *const c_void, proc: &str) -> Option<*const c_void> {
    let name = [proc.as_bytes(), &[0u8]].concat();
    unsafe {
        let proc = GetProcAddress(h, name.as_ptr());
        if proc.is_null() {
            None
        } else {
            Some(proc)
        }
    }
}

pub fn get_decoder(h: *const c_void) -> Option<*const c_void> {
    if let Some(register) = gpa(h, "MilesDriverRegisterBinkAudio") {
        unsafe {
            let off_ptr = register.cast::<u8>().add(3);
            let off = *off_ptr.cast::<u32>();
            Some(off_ptr.add(4).add(off as usize).cast())
        }
    } else {
        None
    }
}
