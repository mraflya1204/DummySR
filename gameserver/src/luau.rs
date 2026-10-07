//! Minimal editor for the string table of a precompiled Luau chunk.
//!
//! A Luau bytecode chunk starts with:
//!   u8 version, u8 types_version (only when version >= 4),
//!   varint string_count, then string_count x (varint length, bytes)
//! Everything after the table refers to strings by their 1-based index in it, never by byte
//! offset, so an entry can be replaced with a string of a different length and the rest of the
//! chunk copied over unchanged.

use anyhow::{Result, anyhow, bail};

fn read_varint(buf: &[u8], pos: &mut usize) -> Result<usize> {
    let mut value = 0usize;
    let mut shift = 0;
    loop {
        let byte = *buf
            .get(*pos)
            .ok_or_else(|| anyhow!("bytecode ends inside a varint"))?;
        *pos += 1;
        if shift > 28 {
            bail!("varint too long");
        }
        value |= usize::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return Ok(value);
        }
        shift += 7;
    }
}

fn write_varint(out: &mut Vec<u8>, mut value: usize) {
    loop {
        let byte = (value & 0x7f) as u8;
        value >>= 7;
        if value == 0 {
            out.push(byte);
            return;
        }
        out.push(byte | 0x80);
    }
}

/// Returns a copy of `bytecode` where each string-table entry equal to a `from` is replaced by
/// its `to`. Fails if the chunk can't be parsed or if any `from` is missing from the table, so a
/// swapped-out payload doesn't silently lose the replacement.
pub fn replace_strings(bytecode: &[u8], replacements: &[(&str, &str)]) -> Result<Vec<u8>> {
    let version = *bytecode.first().ok_or_else(|| anyhow!("empty bytecode"))?;
    if version == 0 {
        bail!("chunk is a compile error message, not bytecode");
    }

    let header_len = if version >= 4 { 2 } else { 1 };
    let mut pos = header_len;
    let count = read_varint(bytecode, &mut pos)?;

    let mut strings = Vec::with_capacity(count);
    for _ in 0..count {
        let len = read_varint(bytecode, &mut pos)?;
        let end = pos
            .checked_add(len)
            .filter(|&end| end <= bytecode.len())
            .ok_or_else(|| anyhow!("string runs past the end of the bytecode"))?;
        strings.push(&bytecode[pos..end]);
        pos = end;
    }

    for (from, _) in replacements {
        if !strings.iter().any(|s| *s == from.as_bytes()) {
            bail!("string {from:?} is not in the bytecode's string table");
        }
    }

    let mut out = Vec::with_capacity(bytecode.len() + 256);
    out.extend_from_slice(&bytecode[..header_len]);
    write_varint(&mut out, count);
    for s in strings {
        let s = replacements
            .iter()
            .find(|(from, _)| from.as_bytes() == s)
            .map_or(s, |(_, to)| to.as_bytes());
        write_varint(&mut out, s.len());
        out.extend_from_slice(s);
    }
    out.extend_from_slice(&bytecode[pos..]);

    Ok(out)
}

/// Returns a copy of `bytecode` with the instruction sequence `find` overwritten by `replace`
/// (same number of 32-bit words). The sequence must occur exactly once, so an edit can't land on
/// the wrong spot if the payload changes.
pub fn replace_code(bytecode: &[u8], find: &[u32], replace: &[u32]) -> Result<Vec<u8>> {
    if find.is_empty() || find.len() != replace.len() {
        bail!("find and replace must be non-empty and the same length");
    }

    let needle: Vec<u8> = find.iter().flat_map(|word| word.to_le_bytes()).collect();
    let mut hits = bytecode
        .windows(needle.len())
        .enumerate()
        .filter(|(_, window)| *window == needle.as_slice())
        .map(|(at, _)| at);

    let at = hits
        .next()
        .ok_or_else(|| anyhow!("instruction sequence not found in bytecode"))?;
    if hits.next().is_some() {
        bail!("instruction sequence found more than once in bytecode");
    }

    let mut out = bytecode.to_vec();
    for (k, word) in replace.iter().enumerate() {
        out[at + 4 * k..at + 4 * k + 4].copy_from_slice(&word.to_le_bytes());
    }

    Ok(out)
}
