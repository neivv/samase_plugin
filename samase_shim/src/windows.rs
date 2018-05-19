#![allow(non_snake_case)]
#![allow(dead_code)]

//! Provides safe winapi wrappers with nicer string handling

use std::ffi::{CString, OsString, OsStr};
use std::os::windows::ffi::{OsStringExt, OsStrExt};
use std::ptr::null_mut;

use winapi::shared::minwindef::{FARPROC, HMODULE};
use winapi::um::libloaderapi::{self, GetModuleFileNameW, LoadLibraryW};
use winapi::um::winuser::{MessageBoxW};

pub fn GetProcAddress(handle: HMODULE, func: &str) -> FARPROC {
    unsafe {
        let name = CString::new(func.as_bytes()).unwrap();
        libloaderapi::GetProcAddress(handle, name.as_ptr())
    }
}

pub fn LoadLibrary(name: &str) -> HMODULE {
    unsafe { LoadLibraryW(winapi_str(name).as_ptr()) }
}

pub fn winapi_str<T: AsRef<OsStr>>(input: T) -> Vec<u16> {
    input.as_ref().encode_wide().chain(Some(0)).collect::<Vec<u16>>()
}

pub fn os_string_from_winapi(input: &[u16]) -> OsString {
    OsString::from_wide(input)
}

pub fn module_name(handle: HMODULE) -> Option<OsString> {
    unsafe {
        let mut buf_size = 128;
        let mut buf = Vec::with_capacity(buf_size);
        loop {
            let result = GetModuleFileNameW(handle, buf.as_mut_ptr(), buf_size as u32);
            match result {
                n if n == buf_size as u32 => {
                    // reserve does not guarantee to reserve exactly specified size,
                    // unline with_capacity
                    let reserve_amt = buf.capacity();
                    buf.reserve(reserve_amt);
                    buf_size = buf.capacity();
                }
                0 => {
                    // Error
                    return None;
                }
                n => {
                    let winapi_str = ::std::slice::from_raw_parts(buf.as_ptr(), n as usize);
                    return Some(os_string_from_winapi(winapi_str));
                }
            }
        }
    }
}

pub fn message_box(caption: &str, msg: &str) {
    unsafe {
        MessageBoxW(null_mut(), winapi_str(msg).as_ptr(), winapi_str(caption).as_ptr(), 0);
    }
}
