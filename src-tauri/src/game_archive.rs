//! Bounded, read-only HashFS v2 access for installed game localization files.
//!
//! Format references: SCS Game Archive Packer documentation and
//! https://github.com/sk-zk/TruckLib.HashFs/blob/master/TruckLib.HashFs/HashFsV2/MainMetadata.cs
//! (format inspection only, no TruckLib code copied). The CityHash implementation
//! below is MIT-licensed scs_tools src/cityhash.rs, pinned to commit
//! e147e755778579e78f80a406a53c64515113d7a1, using Google's 2011 CityHash variant.
//! Its original notices are retained in licenses/locale-cityhash-MIT.txt.
//! No directory walk, extraction to disk, game writes, or external tools.

use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use std::sync::Mutex;

use flate2::read::ZlibDecoder;

type Result<T> = std::result::Result<T, String>;
const MAX_ARCHIVE_BYTES: u64 = 128 * 1024 * 1024 * 1024;
const MAX_INDEX_BYTES: usize = 128 * 1024 * 1024;
const MAX_ENTRIES: usize = 2_000_000;
const MAX_PAYLOAD_BYTES: usize = 32 * 1024 * 1024;

#[derive(Clone, Copy)]
struct Entry {
    offset: u64,
    compressed: usize,
    expanded: usize,
    compression: u8,
}

pub struct Archive {
    file: Mutex<File>,
    length: u64,
    entries: HashMap<u64, Entry>,
}

fn u32_at(bytes: &[u8], offset: usize) -> Result<u32> {
    let end = offset.checked_add(4).ok_or("HashFS offset overflow")?;
    let data = bytes.get(offset..end).ok_or("Truncated HashFS metadata")?;
    Ok(u32::from_le_bytes(
        data.try_into().map_err(|_| "Invalid u32")?,
    ))
}

fn u64_at(bytes: &[u8], offset: usize) -> Result<u64> {
    let end = offset.checked_add(8).ok_or("HashFS offset overflow")?;
    let data = bytes.get(offset..end).ok_or("Truncated HashFS metadata")?;
    Ok(u64::from_le_bytes(
        data.try_into().map_err(|_| "Invalid u64")?,
    ))
}

fn read_range(
    file: &mut File,
    length: u64,
    offset: u64,
    size: usize,
    limit: usize,
) -> Result<Vec<u8>> {
    if size > limit {
        return Err("HashFS read exceeds size limit".into());
    }
    let end = offset
        .checked_add(size as u64)
        .ok_or("HashFS range overflow")?;
    if end > length {
        return Err("HashFS range exceeds archive length".into());
    }
    let mut data = vec![0; size];
    file.seek(SeekFrom::Start(offset))
        .map_err(|e| format!("Seek HashFS: {e}"))?;
    file.read_exact(&mut data)
        .map_err(|e| format!("Read HashFS: {e}"))?;
    Ok(data)
}

fn inflate(data: &[u8], expected: usize, limit: usize) -> Result<Vec<u8>> {
    if expected > limit {
        return Err("HashFS expanded data exceeds size limit".into());
    }
    // Reading one byte past the advertised length detects a forged length while
    // bounding a decompression bomb. Reading to EOF also validates the zlib CRC.
    let mut decoder = ZlibDecoder::new(data).take(expected as u64 + 1);
    let mut out = Vec::with_capacity(expected.min(1024 * 1024));
    decoder
        .read_to_end(&mut out)
        .map_err(|e| format!("Invalid HashFS zlib data: {e}"))?;
    if out.len() != expected {
        return Err("HashFS expanded length does not match metadata".into());
    }
    Ok(out)
}

impl Archive {
    pub fn open(path: &Path) -> Result<Self> {
        let mut file = File::open(path).map_err(|e| format!("Open HashFS: {e}"))?;
        let length = file
            .metadata()
            .map_err(|e| format!("Stat HashFS: {e}"))?
            .len();
        if !(52..=MAX_ARCHIVE_BYTES).contains(&length) {
            return Err("HashFS archive length is outside supported bounds".into());
        }
        let header = read_range(&mut file, length, 0, 52, 52)?;
        if &header[..4] != b"SCS#" || header[4..6] != 2u16.to_le_bytes() {
            return Err("Only HashFS v2 archives are supported".into());
        }
        if header[6..8] != [0, 0] || &header[8..12] != b"CITY" {
            return Err("Unsupported HashFS salt or hash method".into());
        }
        let count = u32_at(&header, 12)? as usize;
        let meta_words = u32_at(&header, 20)? as usize;
        if count > MAX_ENTRIES {
            return Err("HashFS entry count exceeds limit".into());
        }
        let entry_size = count.checked_mul(16).ok_or("HashFS entry size overflow")?;
        let meta_size = meta_words
            .checked_mul(4)
            .ok_or("HashFS metadata size overflow")?;
        if entry_size > MAX_INDEX_BYTES || meta_size > MAX_INDEX_BYTES {
            return Err("HashFS index exceeds limit".into());
        }
        let entry_raw = read_range(
            &mut file,
            length,
            u64_at(&header, 28)?,
            u32_at(&header, 16)? as usize,
            MAX_INDEX_BYTES,
        )?;
        let entries_raw = inflate(&entry_raw, entry_size, MAX_INDEX_BYTES)?;
        let meta_raw = read_range(
            &mut file,
            length,
            u64_at(&header, 36)?,
            u32_at(&header, 24)? as usize,
            MAX_INDEX_BYTES,
        )?;
        let metadata = inflate(&meta_raw, meta_size, MAX_INDEX_BYTES)?;
        let mut entries = HashMap::with_capacity(count);
        for row in entries_raw.chunks_exact(16) {
            let hash = u64_at(row, 0)?;
            let first = u32_at(row, 8)? as usize;
            let parts = u16::from_le_bytes([row[12], row[13]]) as usize;
            let end = first
                .checked_add(parts)
                .ok_or("HashFS part count overflow")?;
            if end > meta_words {
                return Err("HashFS parts exceed metadata table".into());
            }
            let mut payload = None;
            for word in first..end {
                let descriptor = u32_at(&metadata, word * 4)?;
                let kind = descriptor >> 24;
                if kind & 0x80 == 0 {
                    continue;
                }
                if payload.is_some() {
                    return Err("Multiple HashFS data descriptors are unsupported".into());
                }
                let start = (descriptor as usize & 0x00ff_ffff) * 4;
                let compressed_word = u32_at(&metadata, start)?;
                let expanded_word = u32_at(&metadata, start + 4)?;
                let offset = u32_at(&metadata, start + 12)? as u64 * 16;
                let compressed = (compressed_word & 0x0fff_ffff) as usize;
                if offset
                    .checked_add(compressed as u64)
                    .ok_or("HashFS payload range overflow")?
                    > length
                {
                    return Err("HashFS payload exceeds archive length".into());
                }
                payload = Some(Entry {
                    offset,
                    compressed,
                    expanded: (expanded_word & 0x0fff_ffff) as usize,
                    compression: ((compressed_word >> 24) & 0xf0) as u8,
                });
            }
            // Resource-only metadata (such as a texture descriptor) need not
            // represent a plain file. Localization always has one data part.
            if let Some(payload) = payload {
                if entries.insert(hash, payload).is_some() {
                    return Err("Duplicate HashFS path hash".into());
                }
            }
        }
        Ok(Self {
            file: Mutex::new(file),
            length,
            entries,
        })
    }

    pub fn read(&self, path: &str) -> Result<Option<Vec<u8>>> {
        let path = path.strip_prefix('/').unwrap_or(path);
        if path.is_empty()
            || path.contains('\\')
            || path.contains('\0')
            || path
                .split('/')
                .any(|part| part.is_empty() || part == "." || part == "..")
        {
            return Err("Invalid HashFS resource path".into());
        }
        let Some(entry) = self.entries.get(&cityhash::cityhash64(path.as_bytes())) else {
            return Ok(None);
        };
        if entry.expanded > MAX_PAYLOAD_BYTES || entry.compressed > MAX_PAYLOAD_BYTES {
            return Err("HashFS localization payload exceeds limit".into());
        }
        if !matches!(entry.compression, 0 | 0x10) {
            return Err(format!(
                "Unsupported HashFS compression: {:#x}",
                entry.compression
            ));
        }
        let mut file = self
            .file
            .lock()
            .map_err(|_| "HashFS reader lock poisoned")?;
        let raw = read_range(
            &mut file,
            self.length,
            entry.offset,
            entry.compressed,
            MAX_PAYLOAD_BYTES,
        )?;
        drop(file);
        let data = if entry.compression == 0x10 {
            inflate(&raw, entry.expanded, MAX_PAYLOAD_BYTES)?
        } else {
            if raw.len() != entry.expanded {
                return Err("HashFS stored length does not match metadata".into());
            }
            raw
        };
        Ok(Some(data))
    }
}

/// Decode the plaintext UTF-8/BOM files used by the verified locale archive.
/// Obfuscated, encrypted and binary SII formats are deliberately unsupported.
pub fn decode_text(data: &[u8]) -> Result<String> {
    let data = data.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(data);
    if data.starts_with(b"3nK")
        || data.starts_with(b"ScsC")
        || data.starts_with(b"BSII")
        || data.contains(&0)
    {
        return Err("Unsupported encoded or binary localization file".into());
    }
    std::str::from_utf8(data)
        .map(str::to_owned)
        .map_err(|e| format!("Invalid localization UTF-8: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::{write::ZlibEncoder, Compression};
    use std::io::Write;

    fn zlib(data: &[u8]) -> Vec<u8> {
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(data).unwrap();
        encoder.finish().unwrap()
    }

    // An independently assembled rootless archive, containing synthetic text.
    fn fixture(path: &str, data: &[u8], compression: u8, expected: usize) -> Vec<u8> {
        let payload = if compression == 0x10 {
            zlib(data)
        } else {
            data.to_vec()
        };
        let mut bytes = vec![0u8; 64];
        bytes.extend_from_slice(&payload);
        let mut row = Vec::new();
        row.extend_from_slice(&cityhash::cityhash64(path.as_bytes()).to_le_bytes());
        row.extend_from_slice(&0u32.to_le_bytes());
        row.extend_from_slice(&1u16.to_le_bytes());
        row.extend_from_slice(&4u16.to_le_bytes());
        let entry_data = zlib(&row);
        let entry_offset = bytes.len() as u64;
        bytes.extend_from_slice(&entry_data);
        let mut meta = Vec::new();
        for value in [
            0x80000001u32,
            (payload.len() as u32) | ((compression as u32) << 24),
            expected as u32,
            0,
            4,
        ] {
            meta.extend_from_slice(&value.to_le_bytes());
        }
        let meta_data = zlib(&meta);
        let meta_offset = bytes.len() as u64;
        bytes.extend_from_slice(&meta_data);
        bytes[0..4].copy_from_slice(b"SCS#");
        bytes[4..6].copy_from_slice(&2u16.to_le_bytes());
        bytes[8..12].copy_from_slice(b"CITY");
        for (offset, value) in [
            (12, 1),
            (16, entry_data.len() as u32),
            (20, 5),
            (24, meta_data.len() as u32),
        ] {
            bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        }
        bytes[28..36].copy_from_slice(&entry_offset.to_le_bytes());
        bytes[36..44].copy_from_slice(&meta_offset.to_le_bytes());
        bytes
    }

    fn save_fixture(bytes: &[u8]) -> tempfile::NamedTempFile {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(bytes).unwrap();
        file
    }

    #[test]
    fn reads_rootless_stored_and_zlib_by_known_path() {
        let path = "locale/zh_cn/localization.sui";
        let text = "\u{feff}key[]: \"sample\"\nval[]: \"测试\"\n";
        for compression in [0, 0x10] {
            let file = save_fixture(&fixture(path, text.as_bytes(), compression, text.len()));
            let archive = Archive::open(file.path()).unwrap();
            let raw = archive.read(path).unwrap().unwrap();
            assert_eq!(
                decode_text(&raw).unwrap(),
                text.trim_start_matches('\u{feff}')
            );
            assert_eq!(archive.read(&format!("/{path}")).unwrap(), Some(raw));
            assert!(archive.read("locale/en_gb/missing.sui").unwrap().is_none());
            assert!(archive.read("../locale/test.sui").is_err());
        }
    }

    #[test]
    fn rejects_bad_archive_headers_and_offsets() {
        let valid = fixture("locale/common.sui", b"# test", 0, 6);
        for bytes in [
            {
                let mut b = valid.clone();
                b[4] = 1;
                b
            },
            {
                let mut b = valid.clone();
                b[6] = 1;
                b
            },
            {
                let mut b = valid.clone();
                b[28..36].copy_from_slice(&u64::MAX.to_le_bytes());
                b
            },
            {
                let mut b = valid.clone();
                b[20..24].copy_from_slice(&u32::MAX.to_le_bytes());
                b
            },
            valid[..30].to_vec(),
        ] {
            let file = save_fixture(&bytes);
            assert!(Archive::open(file.path()).is_err());
        }
    }

    #[test]
    fn rejects_unknown_compression_and_unbounded_expansion() {
        let path = "locale/common.sui";
        for (compression, expected) in [(0x30, 6), (0, 5), (0x10, 5), (0x10, MAX_PAYLOAD_BYTES + 1)]
        {
            let file = save_fixture(&fixture(path, b"# test", compression, expected));
            let archive = Archive::open(file.path()).unwrap();
            assert!(archive.read(path).is_err());
        }
        let file = save_fixture(&fixture(path, &vec![b'a'; 1024 * 1024], 0x10, 8));
        assert!(Archive::open(file.path()).unwrap().read(path).is_err());
    }

    #[test]
    fn decode_text_rejects_unsupported_formats() {
        for data in [b"3nK\x01".as_slice(), b"ScsC", b"BSII", b"a\0", b"\xff"] {
            assert!(decode_text(data).is_err());
        }
    }

    #[test]
    #[ignore = "Requires a locally installed game; set ETS2_LOCALE_ARCHIVE"]
    fn installed_locale_matches_known_paths() {
        let path = std::env::var_os("ETS2_LOCALE_ARCHIVE").expect("set ETS2_LOCALE_ARCHIVE");
        let archive = Archive::open(Path::new(&path)).unwrap();
        for language in ["en_gb", "zh_cn"] {
            for name in [
                "local.sii",
                "localization.sui",
                "accessories.sui",
                "paintjobs.sui",
            ] {
                let path = format!("locale/{language}/{name}");
                let data = archive.read(&path).unwrap().expect("known locale path");
                let text = decode_text(&data).unwrap();
                assert!(text.contains("key[]:"));
                println!(
                    "{path}: {} bytes, {:016x}",
                    data.len(),
                    cityhash::cityhash64(path.as_bytes())
                );
            }
        }
        let common = archive
            .read("locale/common.sui")
            .unwrap()
            .expect("shared locale include");
        assert!(decode_text(&common).unwrap().contains("key[]:"));
    }
}

// Full 2011 CityHash, including long paths used by future includes or DLC.
mod cityhash {
    //! `CityHash64`, exactly the variant SCS Software's `HashFS` archives use.
    //!
    //! This matches the `CityHash64` bundled with `String::CityHash` 0.10 (city.cc,
    //! Copyright 2011 Google). `CityHash` 0.11 and later changed the algorithm and
    //! produce different hashes, so a generic library `CityHash` is **not** a
    //! substitute here.
    //!
    //! Hashes are taken over raw path bytes — no leading slash, forward slashes,
    //! e.g. `b"def/city.sii"`. The archive root directory is the empty string.

    const K0: u64 = 0xc3a5c85c97cb3127;
    const K1: u64 = 0xb492b66fbe98f273;
    const K2: u64 = 0x9ae16a3b2f90404f;
    const K3: u64 = 0xc949d7c7509e6557;
    const K_MUL: u64 = 0x9ddfea08eb382d69;

    fn fetch64(s: &[u8], i: usize) -> u64 {
        u64::from_le_bytes(s[i..i + 8].try_into().expect("8 bytes"))
    }

    fn fetch32(s: &[u8], i: usize) -> u32 {
        u32::from_le_bytes(s[i..i + 4].try_into().expect("4 bytes"))
    }

    /// Rotate right, passing 0 through unchanged the way the reference does.
    const fn rotate(val: u64, shift: u32) -> u64 {
        if shift == 0 {
            val
        } else {
            val.rotate_right(shift)
        }
    }

    const fn shift_mix(val: u64) -> u64 {
        val ^ (val >> 47)
    }

    const fn hash128to64(low: u64, high: u64) -> u64 {
        let a = (low ^ high).wrapping_mul(K_MUL);
        let a = a ^ (a >> 47);
        let b = (high ^ a).wrapping_mul(K_MUL);
        let b = b ^ (b >> 47);
        b.wrapping_mul(K_MUL)
    }

    const fn hash_len16(u: u64, v: u64) -> u64 {
        hash128to64(u, v)
    }

    fn hash_len0to16(s: &[u8]) -> u64 {
        let len = s.len();
        if len > 8 {
            let a = fetch64(s, 0);
            let b = fetch64(s, len - 8);
            return hash_len16(a, (b.wrapping_add(len as u64)).rotate_right(len as u32)) ^ b;
        }
        if len >= 4 {
            let a = u64::from(fetch32(s, 0));
            return hash_len16(len as u64 + (a << 3), u64::from(fetch32(s, len - 4)));
        }
        if len > 0 {
            let a = u32::from(s[0]);
            let b = u32::from(s[len >> 1]);
            let c = u32::from(s[len - 1]);
            let y = u64::from(a.wrapping_add(b << 8));
            let z = u64::from((len as u32).wrapping_add(c << 2));
            return shift_mix(y.wrapping_mul(K2) ^ z.wrapping_mul(K3)).wrapping_mul(K2);
        }
        K2
    }

    fn hash_len17to32(s: &[u8]) -> u64 {
        let len = s.len();
        let a = fetch64(s, 0).wrapping_mul(K1);
        let b = fetch64(s, 8);
        let c = fetch64(s, len - 8).wrapping_mul(K2);
        let d = fetch64(s, len - 16).wrapping_mul(K0);
        hash_len16(
            rotate(a.wrapping_sub(b), 43)
                .wrapping_add(rotate(c, 30))
                .wrapping_add(d),
            a.wrapping_add(rotate(b ^ K3, 20))
                .wrapping_sub(c)
                .wrapping_add(len as u64),
        )
    }

    const fn weak_hash_raw(w: u64, x: u64, y: u64, z: u64, a: u64, b: u64) -> (u64, u64) {
        let mut a = a.wrapping_add(w);
        let mut b = rotate(b.wrapping_add(a).wrapping_add(z), 21);
        let c = a;
        a = a.wrapping_add(x);
        a = a.wrapping_add(y);
        b = b.wrapping_add(rotate(a, 44));
        (a.wrapping_add(z), b.wrapping_add(c))
    }

    fn weak_hash(s: &[u8], off: usize, a: u64, b: u64) -> (u64, u64) {
        weak_hash_raw(
            fetch64(s, off),
            fetch64(s, off + 8),
            fetch64(s, off + 16),
            fetch64(s, off + 24),
            a,
            b,
        )
    }

    fn hash_len33to64(s: &[u8]) -> u64 {
        let len = s.len();
        let mut z = fetch64(s, 24);
        let mut a = fetch64(s, 0).wrapping_add(
            (len as u64)
                .wrapping_add(fetch64(s, len - 16))
                .wrapping_mul(K0),
        );
        let mut b = rotate(a.wrapping_add(z), 52);
        let mut c = rotate(a, 37);
        a = a.wrapping_add(fetch64(s, 8));
        c = c.wrapping_add(rotate(a, 7));
        a = a.wrapping_add(fetch64(s, 16));
        let vf = a.wrapping_add(z);
        let vs = b.wrapping_add(rotate(a, 31)).wrapping_add(c);

        a = fetch64(s, 16).wrapping_add(fetch64(s, len - 32));
        z = fetch64(s, len - 8);
        b = rotate(a.wrapping_add(z), 52);
        c = rotate(a, 37);
        a = a.wrapping_add(fetch64(s, len - 24));
        c = c.wrapping_add(rotate(a, 7));
        a = a.wrapping_add(fetch64(s, len - 16));
        let wf = a.wrapping_add(z);
        let ws = b.wrapping_add(rotate(a, 31)).wrapping_add(c);

        let r = shift_mix(
            vf.wrapping_add(ws)
                .wrapping_mul(K2)
                .wrapping_add(wf.wrapping_add(vs).wrapping_mul(K0)),
        );
        shift_mix(r.wrapping_mul(K0).wrapping_add(vs)).wrapping_mul(K2)
    }

    /// `CityHash64` of `data`, matching the SCS `HashFS` variant.
    pub fn cityhash64(data: &[u8]) -> u64 {
        let len = data.len();
        if len <= 32 {
            return if len <= 16 {
                hash_len0to16(data)
            } else {
                hash_len17to32(data)
            };
        }
        if len <= 64 {
            return hash_len33to64(data);
        }

        let s = data;
        let mut off = 0usize;
        let mut x = fetch64(s, len - 40);
        let mut y = fetch64(s, len - 16).wrapping_add(fetch64(s, len - 56));
        let mut z = hash_len16(
            fetch64(s, len - 48).wrapping_add(len as u64),
            fetch64(s, len - 24),
        );
        let mut v = weak_hash(s, len - 64, len as u64, z);
        let mut w = weak_hash(s, len - 32, y.wrapping_add(K1), x);
        x = x.wrapping_mul(K1).wrapping_add(fetch64(s, 0));

        let mut remaining = (len - 1) & !63;
        loop {
            x = rotate(
                x.wrapping_add(y)
                    .wrapping_add(v.0)
                    .wrapping_add(fetch64(s, off + 8)),
                37,
            )
            .wrapping_mul(K1);
            y = rotate(y.wrapping_add(v.1).wrapping_add(fetch64(s, off + 48)), 42).wrapping_mul(K1);
            x ^= w.1;
            y = y.wrapping_add(v.0).wrapping_add(fetch64(s, off + 40));
            z = rotate(z.wrapping_add(w.0), 33).wrapping_mul(K1);
            v = weak_hash(s, off, v.1.wrapping_mul(K1), x.wrapping_add(w.0));
            w = weak_hash(
                s,
                off + 32,
                z.wrapping_add(w.1),
                y.wrapping_add(fetch64(s, off + 16)),
            );
            std::mem::swap(&mut z, &mut x);
            off += 64;
            remaining -= 64;
            if remaining == 0 {
                break;
            }
        }

        hash_len16(
            hash_len16(v.0, w.0)
                .wrapping_add(shift_mix(y).wrapping_mul(K1))
                .wrapping_add(z),
            hash_len16(v.1, w.1).wrapping_add(x),
        )
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        /// Vectors taken from the Python implementation this was ported from,
        /// which resolves 627,716 of 627,727 entries across a full retail install.
        /// The inputs cover every length bucket: 0-16, 17-32, 33-64 and the
        /// 64-byte main loop.
        #[test]
        fn matches_the_python_implementation() {
            let cases: &[(&[u8], u64)] = &[
                (b"", 0x9ae16a3b2f90404f),
                (b"a", 0x2420662cd003acfa),
                (b"abc", 0x3a912f483a4ece31),
                (b"def/city.sii", 0x9a5cb1ebaac98229),
                (b"version.txt", 0x9df28354d1b80f29),
                (b"xxxxxxxxxxxxxxxxx", 0xccde5b4c17d97655),
                (b"xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx", 0x83d2f4e954e9c837),
                (
                    b"xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
                    0x3613c0a0c0ee3c93,
                ),
                (
                    b"yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy\
                      yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy",
                    0x07a9c72386a250d6,
                ),
                (b"automat/01/0123456789abcdef.dds", 0xf567a715b38ab762),
            ];
            for (input, expected) in cases {
                assert_eq!(
                    cityhash64(input),
                    *expected,
                    "len {} input {:?}",
                    input.len(),
                    String::from_utf8_lossy(&input[..input.len().min(20)])
                );
            }
            assert_eq!(cityhash64(b""), K2, "empty string hashes to k2");
        }
    }
}
