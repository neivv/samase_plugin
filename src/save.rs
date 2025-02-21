use std::cell::{RefCell};
use std::io::{self, BufRead, Read, Write, Seek, SeekFrom};

use byteorder::{ByteOrder, WriteBytesExt, LE, LittleEndian};
use once_cell::sync::Lazy;
use parking_lot::{Mutex, MutexGuard, const_mutex};
use quick_error::quick_error;
use thread_local::ThreadLocal;

pub type SaveHook = Option<unsafe extern "C" fn(unsafe extern "C" fn(*const u8, usize))>;
pub type LoadHook = Option<unsafe extern "C" fn(*const u8, usize) -> u32>;

const SAVE_MAGIC: u32 = 0x53736d53;
const SAVE_VERSION: u32 = 0;

static SAVE_HOOKS: Mutex<Vec<Hook>> = const_mutex(Vec::new());
static CURRENT_HOOK: Lazy<ThreadLocal<RefCell<Vec<u8>>>> = Lazy::new(|| ThreadLocal::new());

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
    init: unsafe extern "C" fn(),
}

fn save_hooks() -> MutexGuard<'static, Vec<Hook>> {
    SAVE_HOOKS.lock()
}

pub fn add_hook(tag: String, save: SaveHook, load: LoadHook, init: unsafe extern "C" fn()) {
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

fn read_scr_extension_offset<T: File>(file: &mut T) -> Option<Option<u32>> {
    let mut read = io::BufReader::with_capacity(0x400, file);
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
    let mut header = [0u8; 0x10];
    read.read_exact(&mut header).ok()?;
    let version = LittleEndian::read_u32(&header);
    if version & 0xffff < 4 {
        return Some(None);
    }
    // SC:R extension offset is past compressed header struct (0xb5 bytes)
    // Chunk_count should always be 1 here since it fits in a single 0x1000 byte chunk
    let chunk_count = LittleEndian::read_u32(&header[8..]);
    if chunk_count != 1 {
        return None;
    }
    let chunk_size = LittleEndian::read_u32(&header[0xc..]);
    read.seek_relative(chunk_size as i64).ok()?;
    let mut offset = [0u8; 4];
    read.read_exact(&mut offset).ok()?;
    Some(Some(LittleEndian::read_u32(&offset)))
}

// Finds extended data with SAVE_MAGIC and reads it
// If version < 4 (1.16.1), tries to find multiple of them and joins them together.
fn read_extended_data<T: File>(file: &mut T) -> Option<Vec<u8>> {
    let scr_ext_offset = read_scr_extension_offset(file)?;
    if let Some(ext_offset) = scr_ext_offset {
        file.seek(SeekFrom::Start(ext_offset.into())).ok()?;
        loop {
            let mut ext_size = [0u8; 8];
            file.read_exact(&mut ext_size).ok()?;
            let extension = LittleEndian::read_u32(&ext_size);
            let size = LittleEndian::read_u32(&ext_size[4..]);
            if extension == SAVE_MAGIC {
                if size > 0x1000000 {
                    return None;
                }
                let mut buffer = Vec::with_capacity(size as usize);
                file.take(size as u64).read_to_end(&mut buffer).ok()?;
                return Some(buffer);
            } else {
                file.seek(SeekFrom::Current(size.into())).ok()?;
            }
        }
    } else {
        // Join multiple save blocks together
        // Pretty hacky way to do it, parses single blocks to get point
        // where header `chunks` and `data` get split and then
        // joins { VERSION, chunk_count, chunks_0, chunks_1, ..., data_0, data_1, ... }
        // but the format makes it work since there are no offsets in header chunks.
        let mut header_buffer: Vec<u8> = Vec::new();
        header_buffer.resize(0xcusize, 0u8);
        let mut chunk_count = 0;
        let mut data_buffer = Vec::new();
        let mut current_offset = file.seek(SeekFrom::End(-4)).ok()?;
        loop {
            let mut buf = [0u8; 4];
            file.read_exact(&mut buf).ok()?;
            let offset = LittleEndian::read_u32(&buf);
            if offset >= current_offset as u32 || offset < 0x100 {
                break;
            }
            file.seek(SeekFrom::Start(offset as u64)).ok()?;
            let mut buf = [0u8; 8];
            file.read_exact(&mut buf).ok()?;
            let magic = LittleEndian::read_u32(&buf);
            let size = LittleEndian::read_u32(&buf[4..]);
            let expected_size = (current_offset as u32).checked_sub(offset)?.checked_sub(4)?;
            if magic != SAVE_MAGIC || size != expected_size {
                break;
            }
            let mut buf = Vec::with_capacity(size as usize);
            buf.resize(size as usize, 0u8);
            file.read_exact(&mut buf).ok()?;
            let ext = iter_extensions_from_data(buf).ok()?;
            let data_start = ext.buffer_pos;
            let data_end = ext.buffer.len().checked_sub(4)?;
            header_buffer.extend_from_slice(&ext.buffer[0xc..data_start]);
            data_buffer.extend_from_slice(&ext.buffer[data_start..data_end]);
            chunk_count += ext.chunks.len();
            current_offset = file.seek(SeekFrom::Start(u64::from(offset - 4))).ok()?;
        }
        header_buffer.extend_from_slice(&data_buffer);
        LittleEndian::write_u32(&mut header_buffer[4..], chunk_count as u32);
        Some(header_buffer)
    }
}

struct ReadBytes<'a>(&'a [u8]);

impl<'a> ReadBytes<'a> {
    #[inline]
    fn read_u32(&mut self) -> Result<u32, Error> {
        if self.0.len() < 4 {
            Err(Error::BadSave)
        } else {
            let result = LittleEndian::read_u32(self.0);
            self.0 = &self.0[4..];
            Ok(result)
        }
    }

    #[inline]
    fn read_u64(&mut self) -> Result<u64, Error> {
        if self.0.len() < 8 {
            Err(Error::BadSave)
        } else {
            let result = LittleEndian::read_u64(self.0);
            self.0 = &self.0[8..];
            Ok(result)
        }
    }
}

fn iter_extensions<T: File>(file: &mut T) -> Result<IterExtensions, Error> {
    file.seek(SeekFrom::Start(0))?;

    let buffer = read_extended_data(file).ok_or(Error::BadSave)?;
    iter_extensions_from_data(buffer)
}

fn iter_extensions_from_data(buffer: Vec<u8>) -> Result<IterExtensions, Error> {
    let mut read = ReadBytes(&buffer[..]);
    let version = read.read_u32()?;
    if version != SAVE_VERSION {
        return Err(Error::BadSave);
    }
    let chunk_count = read.read_u64()? as usize;
    let mut chunks = Vec::with_capacity(chunk_count);
    let mut compressed_sum = 0usize;
    for _ in 0..chunk_count {
        let name_len = read.read_u64()? as usize;
        let name = match read.0.get(..name_len).and_then(|x| std::str::from_utf8(x).ok()) {
            Some(o) => o,
            None => return Err(Error::BadSave),
        };
        read.0 = &read.0[name_len..];
        let length = read.read_u64()? as usize;
        let compressed = read.read_u64()? as usize;
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
    if read.0.len() < compressed_sum {
        return Err(Error::BadSave);
    }

    return Ok(IterExtensions {
        chunks,
        pos: 0,
        buffer_pos: buffer.len() - read.0.len(),
        buffer,
    });
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
    unsafe extern "C" fn add_save_data(data: *const u8, len: usize) {
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
