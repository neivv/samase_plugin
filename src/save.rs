//! Helper code for implementing save hooks that both 1.16.1 and SC:R patches can use.
//! `mod save_file` has the code for reading save format extensions, while this module has
//! hook code / state.
use std::cell::{RefCell};
use std::io::{self, Write, SeekFrom};

use byteorder::{WriteBytesExt, LE};
use once_cell::sync::Lazy;
use parking_lot::{Mutex, MutexGuard, const_mutex};
use quick_error::quick_error;
use thread_local::ThreadLocal;

pub use super::{SaveHook, LoadHook};
pub use crate::save_file::{File};
use crate::save_file::{self, SAVE_MAGIC, SAVE_VERSION, SerializedChunk};

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
        SaveFile(e: save_file::Error) {
            display("{}", e)
            from()
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

pub fn call_init_hooks() {
    let hooks = save_hooks();
    for hook in hooks.iter() {
        unsafe {
            (hook.init)();
        }
    }
}

pub fn call_load_hooks<T: File>(mut file: T) -> Result<(), Error> {
    let hooks = save_hooks();
    let orig_pos = file.seek(SeekFrom::Current(0))?;
    for chunk in save_file::iter_extensions(&mut file)? {
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
