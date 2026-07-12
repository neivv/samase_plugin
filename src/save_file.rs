//! Functionality for reading samase (plugin) save extensions from a given file.

use std::io::{self, BufRead, Read, Write, Seek, SeekFrom};

use byteorder::{ByteOrder, LittleEndian};
use quick_error::quick_error;

pub const SAVE_MAGIC: u32 = 0x53736d53;
pub const SAVE_VERSION: u32 = 0;

quick_error! {
    #[derive(Debug)]
    pub enum Error {
        Io(e: io::Error) {
            display("I/O error {}", e)
            from()
        }
        BadSave {
            display("Invalid save")
        }
    }
}

pub trait File: Read + Seek + Write {
    fn warn(&mut self, msg: &str);
}

#[derive(Debug)]
pub(crate) struct SerializedChunk {
    pub tag: String,
    pub length: usize,
    pub compressed: usize,
}

pub struct IterExtensions {
    buffer: Vec<u8>,
    chunks: Vec<SerializedChunk>,
    pos: usize,
    buffer_pos: usize,
}

pub struct Chunk {
    pub tag: String,
    pub data: Vec<u8>,
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

pub fn iter_extensions<T: File>(file: &mut T) -> Result<IterExtensions, Error> {
    file.seek(SeekFrom::Start(0))?;

    let buffer = read_extended_data(file).ok_or(Error::BadSave)?;
    iter_extensions_from_data(buffer)
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
