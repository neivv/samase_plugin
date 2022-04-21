extern crate byteorder;
extern crate samase_plugin;

use std::fs;
use std::io::{self, Cursor, Seek};
use std::slice;

use byteorder::{ReadBytesExt, LE};

use samase_plugin::save;

static mut STATE: u32 = 0;

#[test]
fn repeat_tag() {
    save::add_hook("tag".into(), Some(save_hook), Some(load_hook), init_hook);
    save::add_hook("tag".into(), Some(save_hook), Some(load_hook), init_hook);
    save::add_hook("unrelated".into(), Some(nop_save), Some(nop_load), nop_init);
    let data = fs::read("tests/save.snx").unwrap();
    let orig_data = data.clone();
    let mut save_file = TestFile(Cursor::new(data));
    save::call_save_hooks(&mut save_file).unwrap();
    save::call_init_hooks();
    save_file.seek(io::SeekFrom::Start(1235)).unwrap();
    save::call_load_hooks(&mut save_file).unwrap();
    unsafe {
        // Should call save hook twice, init hook twice, load hook 4 times
        assert_eq!(STATE, 8);
    }
    let data = save_file.0.into_inner();
    assert!(data.len() != orig_data.len());
    assert!(orig_data == &data[..orig_data.len()]);
    assert_eq!((&data[orig_data.len()..]).read_u32::<LE>().unwrap(), 0x53736d53);
    assert_eq!(
        (&data[orig_data.len() + 4..]).read_u32::<LE>().unwrap(),
        (data.len() - orig_data.len() - 8) as u32
    );
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
}

unsafe extern fn nop_save(_add_data: unsafe extern fn(*const u8, usize)) {
}

unsafe extern fn nop_load(_data: *const u8, _length: usize) -> u32 {
    1
}

unsafe extern fn nop_init() {
}

unsafe extern fn init_hook() {
    assert!(STATE >= 2 && STATE < 4);
    STATE += 1;
}

unsafe extern fn save_hook(add_data: unsafe extern fn(*const u8, usize)) {
    assert!(STATE < 2);
    STATE += 1;
    let slice = [1, 2, 3, 4, 5, 7];
    add_data(slice.as_ptr(), slice.len());
}

unsafe extern fn load_hook(data: *const u8, length: usize) -> u32 {
    assert!(STATE >= 4 && STATE < 8);
    STATE += 1;
    let slice = slice::from_raw_parts(data, length);
    assert_eq!(slice, &[1, 2, 3, 4, 5, 7]);
    1
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

unsafe extern fn verify_mtl(data: *const u8, length: usize) -> u32 {
    let slice = slice::from_raw_parts(data, length);
    let compare = include_bytes!("mtl.bin");
    assert_eq!(slice, compare);
    1
}

unsafe extern fn verify_aise(data: *const u8, length: usize) -> u32 {
    let slice = slice::from_raw_parts(data, length);
    let compare = include_bytes!("aise.bin");
    assert_eq!(slice, compare);
    1
}

unsafe extern fn verify_aice(data: *const u8, length: usize) -> u32 {
    let slice = slice::from_raw_parts(data, length);
    let compare = &[
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x16, 0x16, 0x16, 0x16, 0x00, 0x16, 0x16, 0x16,
    ];
    assert_eq!(slice, compare);
    1
}
