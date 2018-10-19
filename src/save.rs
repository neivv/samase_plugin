use std;
use std::cell::{RefCell};
use std::io::{self, BufRead, Read, Write, Seek, SeekFrom};

use bincode;
use byteorder::{ReadBytesExt, WriteBytesExt, LE};
use flate2;
use parking_lot::{Mutex, MutexGuard};
use thread_local::CachedThreadLocal;

pub type SaveHook = Option<unsafe extern fn(unsafe extern fn(*const u8, usize))>;
pub type LoadHook = Option<unsafe extern fn(*const u8, usize) -> u32>;

const SAVE_MAGIC: u32 = 0x53736d53;
const SAVE_VERSION: u32 = 0;

lazy_static! {
    static ref SAVE_HOOKS: Mutex<Vec<Hook>> = Mutex::new(Vec::new());
    static ref CURRENT_HOOK: CachedThreadLocal<RefCell<Vec<u8>>> = CachedThreadLocal::new();
}

quick_error! {
    #[derive(Debug)]
    pub enum Error {
        Io(e: io::Error) {
            display("I/O error {}", e)
            from()
        }
        Bincode(e: bincode::Error) {
            display("Bincode: {}", e)
            from()
        }
        HookFail(t: String) {
            display("Extension failure {}", t)
        }
        BadSave {
            display("Invalid save")
        }
    }
}


struct Hook {
    tag: String,
    save: SaveHook,
    load: LoadHook,
    init: unsafe extern fn(),
}

fn save_hooks() -> MutexGuard<'static, Vec<Hook>> {
    SAVE_HOOKS.lock()
}

pub fn add_hook(tag: String, save: SaveHook, load: LoadHook, init: unsafe extern fn()) {
    save_hooks().push(Hook {
        tag,
        save,
        load,
        init,
    });
}

pub trait File: Read + Seek + Write {
    fn warn(&mut self, msg: &str);
}

pub fn call_init_hooks() {
    let hooks = save_hooks();
    for hook in hooks.iter() {
        unsafe {
            (hook.init)();
        }
    }
}

struct IterExtensions<'a, T: File + 'a> {
    file: &'a mut T,
    chunks: Vec<SerializedChunk>,
    pos: usize,
}

#[derive(Debug)]
struct Chunk {
    tag: String,
    data: Vec<u8>,
}

impl<'a, T: File + 'a> Iterator for IterExtensions<'a, T> {
    type Item = Result<Chunk, Error>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.pos >= self.chunks.len() {
            return None;
        }
        let pos = self.file.seek(SeekFrom::Current(0)).unwrap();
        let mut x = Vec::new();
        self.file.read_to_end(&mut x).unwrap();
        self.file.seek(SeekFrom::Start(pos)).unwrap();
        let mut next = || {
            let pos = self.pos;
            self.pos += 1;
            let mut buf = vec![0; self.chunks[pos].length];
            let current_pos = self.file.seek(SeekFrom::Current(0))?;
            let end_pos = current_pos + self.chunks[pos].compressed as u64;
            {
                let mut reader = flate2::read::DeflateDecoder::new(&mut *self.file);
                reader.read_exact(&mut buf)?;
            }
            self.file.seek(SeekFrom::Start(end_pos))?;
            Ok(Chunk {
                tag: self.chunks[pos].tag.clone(),
                data: buf,
            })
        };
        Some(next())
    }
}

fn find_extended_data_offset<T: File>(file: &mut T) -> Option<u64> {
    let mut read = io::BufReader::new(file);
    loop {
        let (skip_amt, end) = {
            let buf = read.fill_buf().ok()?;
            if let Some(pos) = buf.iter().position(|&x| x == 0x1a) {
                (pos + 1, true)
            } else {
                (buf.len(), false)
            }
        };
        read.consume(skip_amt);
        if end {
            break;
        }
    }
    let version = read.read_u32::<LE>().ok()?;
    if version & 0xffff < 4 {
        read.seek(SeekFrom::End(4)).ok()?;
        read.read_u32::<LE>().ok().map(u64::from)
    } else {
        let _ = read.read_u32::<LE>().ok()?;
        let chunk_count = read.read_u32::<LE>().ok()?;
        for _ in 0..chunk_count {
            let chunk_size = read.read_u32::<LE>().ok()?;
            read.seek(SeekFrom::Current(i64::from(chunk_size))).ok()?;
        }
        Some(u64::from(read.read_u32::<LE>().ok()?))
    }
}

fn iter_extensions<T: File>(file: &mut T) -> Result<IterExtensions<T>, Error> {
    file.seek(SeekFrom::Start(0))?;
    let ext_offset = find_extended_data_offset(file).ok_or_else(|| Error::BadSave)?;
    trace!("Save extended offset {:x}", ext_offset);
    file.seek(SeekFrom::Start(ext_offset))?;
    loop {
        let extension = file.read_u32::<LE>()?;
        let size = file.read_u32::<LE>()?;
        if extension == SAVE_MAGIC {
            if size > 0x1000000 {
                return Err(Error::BadSave);
            }
            let version = file.read_u32::<LE>()?;
            if version != SAVE_VERSION {
                return Err(Error::BadSave);
            }
            let mut config = bincode::config();
            config.limit(4096);
            let chunks: Vec<SerializedChunk> = config.deserialize_from(&mut *file)?;
            if chunks.iter().any(|x| x.length > 0x0400_0000) {
                return Err(Error::BadSave);
            }
            if chunks.iter().map(|x| x.compressed).sum::<usize>() > size as usize {
                return Err(Error::BadSave);
            }
            return Ok(IterExtensions {
                file,
                chunks,
                pos: 0,
            });
        } else {
            file.seek(SeekFrom::Current(size as i64))?;
        }
    }
}

pub fn call_load_hooks<T: File>(mut file: T) -> Result<(), Error> {
    let hooks = save_hooks();
    let orig_pos = file.seek(SeekFrom::Current(0))?;
    for chunk in iter_extensions(&mut file)? {
        let chunk = chunk?;
        debug!("Loading {}", chunk.tag);
        for hook in hooks.iter() {
            if hook.tag == chunk.tag {
                trace!("Hook found");
                if let Some(load) = hook.load {
                    trace!("Load hook found");
                    let ok = unsafe {
                        load(chunk.data.as_ptr(), chunk.data.len())
                    };
                    if ok == 0 {
                        return Err(Error::HookFail(chunk.tag.into()));
                    }
                }
            }
        }
    }
    file.seek(SeekFrom::Start(orig_pos))?;
    Ok(())
}

#[derive(Serialize, Deserialize, Debug)]
struct SerializedChunk {
    tag: String,
    length: usize,
    compressed: usize,
}

pub fn call_save_hooks<T: File>(mut file: T) -> Result<(), Error> {
    unsafe extern fn add_save_data(data: *const u8, len: usize) {
        let slice = std::slice::from_raw_parts(data, len);
        let mut current_hook = CURRENT_HOOK.get().unwrap().borrow_mut();
        current_hook.extend_from_slice(slice);
    }

    let mut chunks = Vec::new();
    let mut data = Vec::new();
    let hooks = save_hooks();
    let current_hook_cell = CURRENT_HOOK.get_or(|| RefCell::new(Vec::new()));
    current_hook_cell.replace(Vec::new());
    let chunk_start = file.seek(SeekFrom::End(0))?;
    file.write_u32::<LE>(SAVE_MAGIC)?;
    file.write_u32::<LE>(0)?;
    file.write_u32::<LE>(SAVE_VERSION)?;
    for hook in hooks.iter() {
        if let Some(save) = hook.save {
            unsafe {
                save(add_save_data);
            }
        }
        let previous = current_hook_cell.replace(Vec::new());
        if previous.len() != 0 {
            if previous.len() > 0x0400_0000 {
                file.warn(&format!(
                    "Save failed: extension {} produced too much data ({} bytes)",
                    hook.tag, previous.len(),
                ));
            } else {
                chunks.push(SerializedChunk {
                    tag: hook.tag.clone(),
                    length: previous.len(),
                    compressed: 0,
                });
                data.push(previous);
            }
        }
    }
    bincode::serialize_into(&mut file, &chunks)?;
    for (block, chunk) in data.iter().zip(chunks.iter_mut()) {
        let compressed_size = {
            let mut writer = flate2::write::DeflateEncoder::new(
                &mut file,
                flate2::Compression::default(),
            );
            writer.write_all(&block)?;
            writer.try_finish()?;
            writer.total_out() as usize
        };
        chunk.compressed = compressed_size;
    }
    // Quick hack for 1.16.1 saves
    file.write_u32::<LE>(chunk_start as u32)?;
    let chunk_end = file.seek(SeekFrom::Current(0))?;
    file.seek(SeekFrom::Start(chunk_start + 4))?;
    let chunk_size = chunk_end - (chunk_start + 8);
    assert!(chunk_size < 0x1000000);
    file.write_u32::<LE>(chunk_size as u32)?;
    file.write_u32::<LE>(SAVE_VERSION)?;
    bincode::serialize_into(&mut file, &chunks)?;
    Ok(())
}
