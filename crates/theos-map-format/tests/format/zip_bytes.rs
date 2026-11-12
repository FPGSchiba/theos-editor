//! A small ZIP parser for tests: local headers, the central directory and the end-of-central-
//! directory record are read straight from the bytes, so the writer's output is checked without
//! the `zip` crate that wrote it. Only deflate and stored entries without ZIP64 are understood.

use std::io::Read;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// One central-directory entry together with the fields of its local header.
#[derive(Debug)]
pub struct Entry {
    pub name: String,
    pub version_made_by: u16,
    pub version_needed: u16,
    pub flags: u16,
    pub crc32: u32,
    pub compressed_size: u32,
    pub uncompressed_size: u32,
    pub disk_start: u16,
    pub internal_attrs: u16,
    pub local: Local,
    pub method: u16,
    pub mod_time: u16,
    pub mod_date: u16,
    pub external_attrs: u32,
    pub local_extra_len: u16,
    pub central_extra_len: u16,
    pub comment_len: u16,
    data: Vec<u8>,
}

/// The fields of a local file header.
#[derive(Debug)]
pub struct Local {
    pub version_needed: u16,
    pub flags: u16,
    pub method: u16,
    pub mod_time: u16,
    pub mod_date: u16,
    pub crc32: u32,
    pub compressed_size: u32,
    pub uncompressed_size: u32,
    pub name: String,
}

/// The end-of-central-directory record.
#[derive(Debug)]
pub struct Eocd {
    pub disk: u16,
    pub central_dir_disk: u16,
    pub entries_on_disk: u16,
    pub entries_total: u16,
    pub comment_len: u16,
}

/// A parsed archive.
#[derive(Debug)]
pub struct Zip {
    entries: Vec<Entry>,
    pub eocd: Eocd,
}

fn u16_at(bytes: &[u8], at: usize) -> Result<u16> {
    let raw = bytes.get(at..at + 2).ok_or("truncated u16")?;
    Ok(u16::from_le_bytes([raw[0], raw[1]]))
}

fn u32_at(bytes: &[u8], at: usize) -> Result<u32> {
    let raw = bytes.get(at..at + 4).ok_or("truncated u32")?;
    Ok(u32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]))
}

fn offset(value: u32) -> Result<usize> {
    Ok(usize::try_from(value)?)
}

/// Parses `bytes` as a ZIP archive.
pub fn parse(bytes: &[u8]) -> Result<Zip> {
    let eocd = (0..bytes.len().saturating_sub(21))
        .rev()
        .find(|&at| bytes[at..].starts_with(&[0x50, 0x4b, 0x05, 0x06]))
        .ok_or("no end of central directory")?;
    let count = usize::from(u16_at(bytes, eocd + 10)?);
    let mut at = offset(u32_at(bytes, eocd + 16)?)?;
    let mut entries = Vec::new();
    for _ in 0..count {
        if u32_at(bytes, at)? != 0x0201_4b50 {
            return Err("bad central directory signature".into());
        }
        let name_len = usize::from(u16_at(bytes, at + 28)?);
        let central_extra_len = u16_at(bytes, at + 30)?;
        let comment_len = u16_at(bytes, at + 32)?;
        let name_bytes = bytes.get(at + 46..at + 46 + name_len).ok_or("name")?;
        let name = String::from_utf8(name_bytes.to_vec())?;
        let compressed = offset(u32_at(bytes, at + 20)?)?;
        let local = offset(u32_at(bytes, at + 42)?)?;
        if u32_at(bytes, local)? != 0x0403_4b50 {
            return Err("bad local header signature".into());
        }
        let local_name_len = usize::from(u16_at(bytes, local + 26)?);
        let local_name_bytes = bytes
            .get(local + 30..local + 30 + local_name_len)
            .ok_or("name")?;
        let local_fields = Local {
            version_needed: u16_at(bytes, local + 4)?,
            flags: u16_at(bytes, local + 6)?,
            method: u16_at(bytes, local + 8)?,
            mod_time: u16_at(bytes, local + 10)?,
            mod_date: u16_at(bytes, local + 12)?,
            crc32: u32_at(bytes, local + 14)?,
            compressed_size: u32_at(bytes, local + 18)?,
            uncompressed_size: u32_at(bytes, local + 22)?,
            name: String::from_utf8(local_name_bytes.to_vec())?,
        };
        let local_extra_len = u16_at(bytes, local + 28)?;
        let start = local + 30 + local_name_len + usize::from(local_extra_len);
        let raw = bytes.get(start..start + compressed).ok_or("data")?;
        let method = u16_at(bytes, at + 10)?;
        let data = match method {
            0 => raw.to_vec(),
            8 => {
                let mut out = Vec::new();
                flate2::read::DeflateDecoder::new(raw).read_to_end(&mut out)?;
                out
            }
            other => return Err(format!("method {other}").into()),
        };
        entries.push(Entry {
            name,
            version_made_by: u16_at(bytes, at + 4)?,
            version_needed: u16_at(bytes, at + 6)?,
            flags: u16_at(bytes, at + 8)?,
            crc32: u32_at(bytes, at + 16)?,
            compressed_size: u32_at(bytes, at + 20)?,
            uncompressed_size: u32_at(bytes, at + 24)?,
            disk_start: u16_at(bytes, at + 34)?,
            internal_attrs: u16_at(bytes, at + 36)?,
            local: local_fields,
            method,
            mod_time: u16_at(bytes, at + 12)?,
            mod_date: u16_at(bytes, at + 14)?,
            external_attrs: u32_at(bytes, at + 38)?,
            local_extra_len,
            central_extra_len,
            comment_len,
            data,
        });
        at += 46 + name_len + usize::from(central_extra_len) + usize::from(comment_len);
    }
    Ok(Zip {
        entries,
        eocd: Eocd {
            disk: u16_at(bytes, eocd + 4)?,
            central_dir_disk: u16_at(bytes, eocd + 6)?,
            entries_on_disk: u16_at(bytes, eocd + 8)?,
            entries_total: u16_at(bytes, eocd + 10)?,
            comment_len: u16_at(bytes, eocd + 20)?,
        },
    })
}

impl Zip {
    /// The entry names in archive order.
    pub fn names(&self) -> Vec<&str> {
        self.entries.iter().map(|e| e.name.as_str()).collect()
    }

    /// The entries in archive order.
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    /// The decompressed bytes of the entry called `name`.
    pub fn entry(&self, name: &str) -> Result<Vec<u8>> {
        let found = self.entries.iter().find(|e| e.name == name);
        Ok(found.ok_or("no such entry")?.data.clone())
    }
}
