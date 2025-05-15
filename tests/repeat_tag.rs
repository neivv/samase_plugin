extern crate byteorder;
extern crate samase_plugin;

use std::fs;
use std::io::{self, Cursor, Seek};
use std::slice;
use std::sync::atomic::{AtomicUsize, Ordering};

use byteorder::{ReadBytesExt, LE};

use samase_plugin::save;

static STATE: AtomicUsize = AtomicUsize::new(0);

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
    // Should call save hook twice, init hook twice, load hook 4 times
    assert_eq!(STATE.get(), 8);
    let data = save_file.0.into_inner();
    assert!(data.len() != orig_data.len());
    assert!(orig_data == &data[..orig_data.len()]);
    assert_eq!((&data[orig_data.len()..]).read_u32::<LE>().unwrap(), 0x53736d53);
    assert_eq!(
        (&data[orig_data.len() + 4..]).read_u32::<LE>().unwrap(),
        (data.len() - orig_data.len() - 8) as u32
    );
}

unsafe extern "C" fn nop_save(_add_data: unsafe extern "C" fn(*const u8, usize)) {
}

unsafe extern "C" fn nop_load(_data: *const u8, _length: usize) -> u32 {
    1
}

unsafe extern "C" fn nop_init() {
}

unsafe extern "C" fn init_hook() {
    assert!(STATE.get() >= 2 && STATE.get() < 4);
    STATE.set(STATE.get() + 1);
}

unsafe extern "C" fn save_hook(add_data: unsafe extern "C" fn(*const u8, usize)) {
    assert!(STATE.get() < 2);
    STATE.set(STATE.get() + 1);
    let slice = [1, 2, 3, 4, 5, 7];
    add_data(slice.as_ptr(), slice.len());
}

unsafe extern "C" fn load_hook(data: *const u8, length: usize) -> u32 {
    assert!(STATE.get() >= 4 && STATE.get() < 8);
    STATE.set(STATE.get() + 1);
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
