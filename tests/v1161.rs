extern crate samase_plugin;

use std::fs;
use std::io::{self, Cursor};
use std::slice;
use std::sync::atomic::{AtomicUsize, Ordering};

use samase_plugin::save;

static V1161_STATE: AtomicUsize = AtomicUsize::new(0);

trait ExtAtomic {
    fn get(&self) -> usize;
    fn set(&self, val: usize);
}

impl ExtAtomic for AtomicUsize {
    fn get(&self) -> usize {
        self.load(Ordering::Relaxed)
    }

    fn set(&self, val: usize) {
        self.store(val, Ordering::Relaxed)
    }
}

#[test]
fn v1161_compat() {
    // Tests that reading multiple save sections by different plugins works correctly
    // If format is changed one day then this test breaks and either should be updated
    // or removed.
    save::add_hook("mtl".into(), Some(nop_save), Some(verify_mtl), nop_init);
    save::add_hook("aise".into(), Some(nop_save), Some(verify_aise), nop_init);
    save::add_hook("aice".into(), Some(nop_save), Some(verify_aice), nop_init);
    let data = fs::read("tests/idk.snx").unwrap();
    let mut save_file = TestFile(Cursor::new(data));
    save::call_load_hooks(&mut save_file).unwrap();
    assert_eq!(V1161_STATE.get(), 3);
}

unsafe extern "C" fn nop_save(_add_data: unsafe extern "C" fn(*const u8, usize)) {
}

unsafe extern "C" fn nop_init() {
}

pub struct TestFile(Cursor<Vec<u8>>);

impl io::Read for TestFile {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        self.0.read(out)
    }
}

impl io::Write for TestFile {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        self.0.write(data)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}

impl io::Seek for TestFile {
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        self.0.seek(pos)
    }
}

impl<'a> save::File for &'a mut TestFile {
    fn warn(&mut self, msg: &str) {
        panic!("Warnings not expected: {}", msg);
    }
}

unsafe extern "C" fn verify_mtl(data: *const u8, length: usize) -> u32 {
    V1161_STATE.set(V1161_STATE.get() + 1);
    let slice = slice::from_raw_parts(data, length);
    let compare = include_bytes!("mtl.bin");
    assert_eq!(slice, compare);
    1
}

unsafe extern "C" fn verify_aise(data: *const u8, length: usize) -> u32 {
    V1161_STATE.set(V1161_STATE.get() + 1);
    let slice = slice::from_raw_parts(data, length);
    let compare = include_bytes!("aise.bin");
    assert_eq!(slice, compare);
    1
}

unsafe extern "C" fn verify_aice(data: *const u8, length: usize) -> u32 {
    V1161_STATE.set(V1161_STATE.get() + 1);
    let slice = slice::from_raw_parts(data, length);
    let compare = &[
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x16, 0x16, 0x16, 0x16, 0x00, 0x16, 0x16, 0x16,
    ];
    assert_eq!(slice, compare);
    1
}
