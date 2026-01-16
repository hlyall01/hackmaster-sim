#[cfg(target_os = "windows")]
mod windows {
    use std::fs::OpenOptions;
    use std::mem;
    use std::os::windows::io::AsRawHandle;

    type Handle = *mut core::ffi::c_void;

    const ATTACH_PARENT_PROCESS: u32 = 0xFFFF_FFFF;
    const STD_INPUT_HANDLE: u32 = (-10i32) as u32;
    const STD_OUTPUT_HANDLE: u32 = (-11i32) as u32;
    const STD_ERROR_HANDLE: u32 = (-12i32) as u32;

    unsafe extern "system" {
        fn AllocConsole() -> i32;
        fn AttachConsole(dwProcessId: u32) -> i32;
        fn SetStdHandle(nStdHandle: u32, hHandle: Handle) -> i32;
    }

    pub fn enable_console() {
        unsafe {
            if AttachConsole(ATTACH_PARENT_PROCESS) == 0 {
                AllocConsole();
            }
        }

        let conout = OpenOptions::new().read(true).write(true).open("CONOUT$");
        let conin = OpenOptions::new().read(true).write(true).open("CONIN$");

        if let Ok(conout) = conout {
            unsafe {
                SetStdHandle(STD_OUTPUT_HANDLE, conout.as_raw_handle());
                SetStdHandle(STD_ERROR_HANDLE, conout.as_raw_handle());
            }
            mem::forget(conout);
        }
        if let Ok(conin) = conin {
            unsafe {
                SetStdHandle(STD_INPUT_HANDLE, conin.as_raw_handle());
            }
            mem::forget(conin);
        }
    }
}

pub fn maybe_enable_console() {
    #[cfg(target_os = "windows")]
    {
        if std::env::args().any(|arg| arg == "--console") {
            windows::enable_console();
        }
    }
}
