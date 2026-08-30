//! Bounded parsers for Rage of Mages (J2ME) original binary resources.
//!
//! Formats present in the corpus (see `docs/FORMATS.md`):
//! - [`map`] — `.map` level terrain grids
//! - [`utf`] — `.utf` localization string tables (model from evidence, not an
//!   assumed character encoding; rulebook R11)
//! - [`pak`] — `res0..4.pak` XOR-packed sprite/data packs
//! - [`scenes`] — `scenes.pak` per-level scripting data (a different,
//!   nested-int format despite the extension)
//! - [`int_table`] — `sincos/*.int` sin/cos lookup tables
//! - `.png` sprites, `.mid` music (standard formats; no parser here)
//!
//! Contract (project rule): every parser rejects malformed or truncated input
//! with a typed error and never panics, and is exercised against every unique
//! blob in the corpus (`tests/corpus_oracle.rs`).
//!
//! Each format's layout (endianness included) is reversed from the game's own
//! bytecode, never guessed; the modules build on the bounded [`Reader`]
//! foundation below — a thin adapter over the shared, `no_std`
//! [`j2me_codec::Reader`] that keeps this crate's game-specific [`FormatError`]
//! surface (the domain `Invalid` variant the parsers raise on bad values).

use j2me_codec::DecodeError;
use thiserror::Error;

pub mod int_table;
pub mod map;
pub mod pak;
pub mod scenes;
pub mod utf;

/// A bounded reader over an in-memory resource blob.
///
/// All reads are checked; running off the end returns [`FormatError::Truncated`]
/// instead of panicking, so a malformed archive can never crash a parser. The
/// byte-level bounds checking is the shared [`j2me_codec::Reader`]; this type
/// adapts its [`DecodeError`] into the crate's [`FormatError`] and preserves the
/// original method surface the parsers were written against.
#[derive(Debug, Clone)]
pub struct Reader<'a> {
    inner: j2me_codec::Reader<'a>,
    len: usize,
}

/// Errors a bounded parser may return. Never panic on bad input.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum FormatError {
    #[error("unexpected end of input: needed {needed} byte(s) at offset {at}, {have} available")]
    Truncated {
        at: usize,
        needed: usize,
        have: usize,
    },
    #[error("invalid value at offset {at}: {reason}")]
    Invalid { at: usize, reason: &'static str },
}

/// Map the shared reader's [`DecodeError`] onto the crate's [`FormatError`],
/// reproducing the original `Truncated { at, needed, have }` shape a bounds
/// failure yielded (`at` = the read offset, `have` = the bytes actually left).
/// `len` is the blob's total length (captured once at construction).
fn map_decode(error: DecodeError, len: usize) -> FormatError {
    match error {
        DecodeError::UnexpectedEof { offset, needed } => FormatError::Truncated {
            at: offset,
            needed,
            have: len.saturating_sub(offset),
        },
        // The shared reader guards `offset + length` against `usize` overflow;
        // corpus blobs never reach it, but surface it as a bounds failure.
        DecodeError::LengthOverflow => FormatError::Truncated {
            at: len,
            needed: usize::MAX,
            have: 0,
        },
        // Only raised by `finish()`, which this adapter never calls.
        DecodeError::TrailingData { offset, .. } => FormatError::Invalid {
            at: offset,
            reason: "unexpected trailing data",
        },
    }
}

impl<'a> Reader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            inner: j2me_codec::Reader::new(data),
            len: data.len(),
        }
    }

    pub fn position(&self) -> usize {
        self.inner.position()
    }

    pub fn remaining(&self) -> usize {
        self.inner.remaining()
    }

    /// One unsigned byte.
    pub fn u8(&mut self) -> Result<u8, FormatError> {
        let len = self.len;
        self.inner.read_u8().map_err(|e| map_decode(e, len))
    }

    /// One signed byte (Java `DataInput.readByte` order/sign).
    pub fn i8(&mut self) -> Result<i8, FormatError> {
        let len = self.len;
        self.inner.read_i8().map_err(|e| map_decode(e, len))
    }

    /// Big-endian u16 (Java `DataInput` order).
    pub fn u16_be(&mut self) -> Result<u16, FormatError> {
        let len = self.len;
        self.inner.read_u16_be().map_err(|e| map_decode(e, len))
    }

    /// Big-endian i16 (Java `DataInput.readShort` order/sign).
    pub fn i16_be(&mut self) -> Result<i16, FormatError> {
        let len = self.len;
        self.inner.read_i16_be().map_err(|e| map_decode(e, len))
    }

    /// Little-endian u16.
    pub fn u16_le(&mut self) -> Result<u16, FormatError> {
        let len = self.len;
        self.inner.read_u16_le().map_err(|e| map_decode(e, len))
    }

    /// Big-endian u32 (Java `DataInput` order).
    pub fn u32_be(&mut self) -> Result<u32, FormatError> {
        let len = self.len;
        self.inner.read_u32_be().map_err(|e| map_decode(e, len))
    }

    /// Advance the cursor by `n` bytes, or fail if fewer remain.
    pub fn skip(&mut self, n: usize) -> Result<(), FormatError> {
        let len = self.len;
        self.inner.skip(n).map_err(|e| map_decode(e, len))
    }

    /// Seek to an absolute offset, failing if it is past the end of input.
    pub fn seek(&mut self, at: usize) -> Result<(), FormatError> {
        let len = self.len;
        self.inner.seek(at).map_err(|e| map_decode(e, len))
    }

    /// Total length of the underlying blob.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Whether the underlying blob is empty.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Big-endian i32 (Java `DataInput` order).
    pub fn i32_be(&mut self) -> Result<i32, FormatError> {
        let len = self.len;
        self.inner.read_i32_be().map_err(|e| map_decode(e, len))
    }

    /// Borrow `n` raw bytes.
    pub fn bytes(&mut self, n: usize) -> Result<&'a [u8], FormatError> {
        let len = self.len;
        self.inner.read_exact(n).map_err(|e| map_decode(e, len))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_reads_never_panic_on_truncation() {
        let mut r = Reader::new(&[0x01]);
        assert_eq!(r.u8(), Ok(1));
        // Reading past the end returns an error, never panics.
        assert_eq!(
            r.u16_be(),
            Err(FormatError::Truncated {
                at: 1,
                needed: 2,
                have: 0
            })
        );
    }

    #[test]
    fn big_endian_matches_java_datainput() {
        let mut r = Reader::new(&[0x12, 0x34, 0x00, 0x00, 0x00, 0x2a]);
        assert_eq!(r.u16_be(), Ok(0x1234));
        assert_eq!(r.i32_be(), Ok(42));
    }
}
