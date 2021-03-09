use std::cell::{RefCell};
use std::io::{self, BufRead, Read, Write, Seek, SeekFrom};

use byteorder::{ByteOrder, ReadBytesExt, WriteBytesExt, LE, LittleEndian};
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

struct IterExtensions {
    buffer: Vec<u8>,
    chunks: Vec<SerializedChunk>,
    pos: usize,
    buffer_pos: usize,
}

#[derive(Debug)]
struct Chunk {
    tag: String,
    data: Vec<u8>,
}

impl Iterator for IterExtensions {
    type Item = Result<Chunk, Error>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.pos >= self.chunks.len() {
            return None;
        }

        let mut next = || {
            let pos = self.pos;
            self.pos += 1;
            let chunk = &self.chunks[pos];
            trace!(
                "Yield save extension {} {:x}/{:x}",
                chunk.tag, chunk.length, chunk.compressed,
            );
            let mut buf = vec![0; chunk.length];
            {
                let slice = &self.buffer[self.buffer_pos..][..chunk.compressed];
                let mut reader = flate2::read::DeflateDecoder::new(slice);
                reader.read_exact(&mut buf)?;
            }
            self.buffer_pos += chunk.compressed;
            Ok(Chunk {
                tag: chunk.tag.clone(),
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

fn iter_extensions<T: File>(file: &mut T) -> Result<IterExtensions, Error> {
    file.seek(SeekFrom::Start(0))?;
    let ext_offset = find_extended_data_offset(file).ok_or_else(|| Error::BadSave)?;
    trace!("Save extended offset {:x}", ext_offset);
    file.seek(SeekFrom::Start(ext_offset))?;
    loop {
        let mut ext_size = [0u8; 8];
        file.read_exact(&mut ext_size)?;
        let extension = LittleEndian::read_u32(&ext_size);
        let size = LittleEndian::read_u32(&ext_size[4..]);
        if extension == SAVE_MAGIC {
            if size > 0x1000000 {
                return Err(Error::BadSave);
            }
            let mut buffer = Vec::with_capacity(size as usize);
            file.take(size as u64).read_to_end(&mut buffer)?;
            let mut read = &buffer[..];
            let version = read.read_u32::<LE>()?;
            if version != SAVE_VERSION {
                return Err(Error::BadSave);
            }
            let chunk_count = read.read_u64::<LE>()? as usize;
            let mut chunks = Vec::with_capacity(chunk_count);
            let mut compressed_sum = 0usize;
            for _ in 0..chunk_count {
                let name_len = read.read_u64::<LE>()? as usize;
                let name = match std::str::from_utf8(&read[..name_len]) {
                    Ok(o) => o,
                    Err(_) => return Err(Error::BadSave),
                };
                read = &read[name_len..];
                let length = read.read_u64::<LE>()? as usize;
                let compressed = read.read_u64::<LE>()? as usize;
                if length > 0x0400_0000 {
                    return Err(Error::BadSave);
                }
                compressed_sum = compressed_sum.checked_add(compressed)
                    .ok_or_else(|| Error::BadSave)?;
                chunks.push(SerializedChunk {
                    tag: name.into(),
                    length,
                    compressed,
                });
            }
            // Won't be exactly same since there's also 1161-compatibility u32
            if read.len() < compressed_sum {
                return Err(Error::BadSave);
            }
            return Ok(IterExtensions {
                chunks,
                pos: 0,
                buffer_pos: buffer.len() - read.len(),
                buffer,
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

#[derive(Debug)]
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
    trace!("Writing save extension chunk starting from offset {:x}", chunk_start);
    // Format: (First 2 fields are part of SC:R extension header)
    // u32 magic
    // u32 rest_len
    // u32 version (0)
    // u64 extension_count
    // Extension chunks[extension_count] {
    //     u64 name_len
    //     char name[name_len] (Not null-terminated)
    //     u64 length
    //     u64 compressed_length
    // }
    // u8 chunk_data [compressed_length][extension_count] (Deflated)
    let mut buffer = Vec::with_capacity(0x2000);
    buffer.write_u32::<LE>(SAVE_MAGIC)?;
    buffer.write_u32::<LE>(0)?;
    buffer.write_u32::<LE>(SAVE_VERSION)?;
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
    buffer.write_u64::<LE>(chunks.len() as u64)?;
    let chunks_size = chunks.iter()
        .map(|x| (8usize * 3).wrapping_add(x.tag.len()))
        .sum();
    let chunks_start = buffer.len();
    buffer.resize_with(chunks_start + chunks_size, || 0);
    for (block, chunk) in data.iter().zip(chunks.iter_mut()) {
        let compressed_size = {
            let mut writer = flate2::write::DeflateEncoder::new(
                &mut buffer,
                flate2::Compression::default(),
            );
            writer.write_all(&block)?;
            writer.try_finish()?;
            writer.total_out() as usize
        };
        chunk.compressed = compressed_size;
        trace!("Write save extension {} {:x}/{:x}", chunk.tag, chunk.length, chunk.compressed);
    }

    // Quick hack for 1.16.1 saves. Store samase chunk offset
    // as last u32 of the file. (For SC:R it is stored among all other extended chunks)
    buffer.write_u32::<LE>(chunk_start as u32)?;
    // Fix header offsets
    let chunk_size = buffer.len() - 8;
    assert!(chunk_size < 0x1000000);
    (&mut buffer[4..]).write_u32::<LE>(chunk_size as u32)?;
    let mut out = &mut buffer[chunks_start..][..chunks_size];
    for chunk in &chunks {
        let tag = chunk.tag.as_bytes();
        out.write_u64::<LE>(tag.len() as u64)?;
        (&mut out[..tag.len()]).copy_from_slice(tag);
        out = &mut out[tag.len()..];
        out.write_u64::<LE>(chunk.length as u64)?;
        out.write_u64::<LE>(chunk.compressed as u64)?;
    }
    file.write_all(&buffer).map_err(|x| x.into())
}
