//! `.pak` — XOR-packed resource packs (`res/res0.pak` … `res/res4.pak`).
//!
//! Reversed from the consumer (`docs/FORMATS.md`, rulebook R10): class `j` —
//! `j.a(String, byte key, Class)` opens & reads the directory, `j.a()` returns
//! the current entry decrypting each byte with the key, `j.a(int n)` skips `n`
//! entries by walking the directory cumulatively. Every caller passes key
//! `(byte)83` = `0x53` = ASCII `'S'`. All integers are Java `readInt`
//! (**big-endian**):
//!
//! ```text
//! i32 BE          count
//! count × i32 BE  length[i]     entry byte-lengths (sequential index; no names)
//! then, back to back:
//!   entry i : length[i] bytes, each byte XOR 0x53
//! ```
//!
//! File length is exactly `4 + 4*count + Σ length[i]`; anything else is
//! rejected. Entries are *usually* complete PNGs (entry order = sprite-slot
//! order), but not always — `res0.pak` entry 2 is the bitmap-font descriptor
//! blob — so this parser does **not** assert PNG-ness; the corpus gate proves
//! it on the entries that are PNGs, with an independent PNG decoder.
//!
//! `res/scenes.pak` is a **different format** despite the extension — see
//! [`crate::scenes`].

use crate::{FormatError, Reader};

/// The entry cipher key every caller passes to class `j`: `0x53` = ASCII `'S'`.
pub const XOR_KEY: u8 = 0x53;

/// A parsed pack: the decrypted entries, in directory order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pak {
    entries: Vec<Vec<u8>>,
}

impl Pak {
    /// Parse a `.pak` blob and decrypt every entry (XOR `0x53`). Rejects a
    /// negative count or entry length, a directory/body that over-runs EOF, or
    /// trailing bytes after the last entry, with a typed error; never panics.
    pub fn parse(data: &[u8]) -> Result<Pak, FormatError> {
        let mut r = Reader::new(data);
        let at = r.position();
        let count = r.i32_be()?;
        if count < 0 {
            return Err(FormatError::Invalid {
                at,
                reason: "negative pak entry count",
            });
        }
        let count = count as usize;

        let mut lengths = Vec::with_capacity(count.min(1 << 16));
        for _ in 0..count {
            let at = r.position();
            let len = r.i32_be()?;
            if len < 0 {
                return Err(FormatError::Invalid {
                    at,
                    reason: "negative pak entry length",
                });
            }
            lengths.push(len as usize);
        }

        let mut entries = Vec::with_capacity(count.min(1 << 16));
        for &len in &lengths {
            let raw = r.bytes(len)?;
            entries.push(raw.iter().map(|&b| b ^ XOR_KEY).collect());
        }

        if r.remaining() != 0 {
            return Err(FormatError::Invalid {
                at: r.position(),
                reason: "trailing bytes after the last pak entry",
            });
        }
        Ok(Pak { entries })
    }

    /// All decrypted entries, in directory order.
    pub fn entries(&self) -> &[Vec<u8>] {
        &self.entries
    }

    /// Number of entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the pack holds no entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// One decrypted entry by index, or `None` when out of range.
    pub fn entry(&self, index: usize) -> Option<&[u8]> {
        self.entries.get(index).map(Vec::as_slice)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a synthetic, hand-authored pack from plaintext entries (rulebook
    /// R1: no game bytes) — the builder XOR-encrypts, the parser decrypts.
    fn build(entries: &[&[u8]]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&(entries.len() as i32).to_be_bytes());
        for e in entries {
            out.extend_from_slice(&(e.len() as i32).to_be_bytes());
        }
        for e in entries {
            out.extend(e.iter().map(|&b| b ^ XOR_KEY));
        }
        out
    }

    #[test]
    fn parses_and_decrypts_entries_in_directory_order() {
        let blob = build(&[b"first entry", b"", b"third"]);
        let pak = Pak::parse(&blob).unwrap();
        assert_eq!(pak.len(), 3);
        assert_eq!(pak.entry(0), Some(&b"first entry"[..]));
        assert_eq!(pak.entry(1), Some(&b""[..]));
        assert_eq!(pak.entry(2), Some(&b"third"[..]));
        assert_eq!(pak.entry(3), None);
    }

    #[test]
    fn empty_pack_is_valid() {
        let pak = Pak::parse(&build(&[])).unwrap();
        assert!(pak.is_empty());
    }

    // ----- malformed / truncated rejection (rulebook R3) -----

    #[test]
    fn rejects_a_corrupted_length_that_breaks_the_total_invariant() {
        let mut blob = build(&[b"abc", b"defg"]);
        // Bump entry 0's length from 3 to 4: the body now over-runs EOF.
        blob[7] = 4;
        assert!(matches!(
            Pak::parse(&blob),
            Err(FormatError::Truncated { .. })
        ));
    }

    #[test]
    fn rejects_trailing_bytes_after_the_last_entry() {
        let mut blob = build(&[b"abc"]);
        blob.push(0x99);
        assert!(matches!(
            Pak::parse(&blob),
            Err(FormatError::Invalid {
                reason: "trailing bytes after the last pak entry",
                ..
            })
        ));
    }

    #[test]
    fn rejects_a_negative_count_and_a_negative_length() {
        let mut blob = build(&[b"abc"]);
        blob[0] = 0xFF; // count sign bit
        assert_eq!(
            Pak::parse(&blob),
            Err(FormatError::Invalid {
                at: 0,
                reason: "negative pak entry count",
            })
        );

        let mut blob = build(&[b"abc"]);
        blob[4] = 0xFF; // length[0] sign bit
        assert_eq!(
            Pak::parse(&blob),
            Err(FormatError::Invalid {
                at: 4,
                reason: "negative pak entry length",
            })
        );
    }

    #[test]
    fn rejects_a_truncated_directory() {
        let blob = build(&[b"abc", b"def"]);
        // Cut inside the second directory word.
        assert!(matches!(
            Pak::parse(&blob[..10]),
            Err(FormatError::Truncated { .. })
        ));
    }
}
