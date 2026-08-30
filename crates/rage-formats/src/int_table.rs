//! `sincos/sin.int`, `sincos/cos.int` — fixed-point trig lookup tables.
//!
//! Reversed from the consumer (`docs/FORMATS.md`, rulebook R10): class `o`'s
//! constructor reads each file into a `short[90]` with `readShort()`
//! (**big-endian i16**) — `90 × i16 BE`, one entry per degree `0..=89` (a
//! quarter turn, 1° step). The stored value is `round(SCALE · sin(deg))` /
//! `round(SCALE · cos(deg))` with [`SCALE`] `= 10000` (`o.a()` returns that
//! constant); the corpus gate proves the curve with **max error 0** against
//! ideal trigonometry. The full circle is reconstructed game-side by quadrant
//! folding (`o.a(short)` / `o.b(short)`), not stored here.
//!
//! The parser is generic over the entry count (it consumes the whole blob);
//! the corpus tables carry exactly 90 entries.

use crate::{FormatError, Reader};

/// The fixed-point scale of the stored values (`o.a()`'s constant).
pub const SCALE: i32 = 10_000;

/// A parsed `.int` table: consecutive big-endian `i16` values to EOF.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntTable {
    values: Vec<i16>,
}

impl IntTable {
    /// Parse an `.int` blob. Rejects an odd byte length (not a whole number of
    /// `i16` entries) with a typed error; never panics.
    pub fn parse(data: &[u8]) -> Result<IntTable, FormatError> {
        if !data.len().is_multiple_of(2) {
            return Err(FormatError::Invalid {
                at: data.len() - 1,
                reason: "int table byte length is odd (not a whole number of i16)",
            });
        }
        let mut r = Reader::new(data);
        let mut values = Vec::with_capacity(data.len() / 2);
        while r.remaining() > 0 {
            values.push(r.i16_be()?);
        }
        Ok(IntTable { values })
    }

    /// All values, in file order (index = degree for the corpus tables).
    pub fn values(&self) -> &[i16] {
        &self.values
    }

    /// Number of entries.
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Whether the table holds no entries.
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// One value by index, or `None` when out of range.
    pub fn get(&self, index: usize) -> Option<i16> {
        self.values.get(index).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a synthetic, hand-authored table (rulebook R1: no game bytes).
    fn build(values: &[i16]) -> Vec<u8> {
        values.iter().flat_map(|v| v.to_be_bytes()).collect()
    }

    #[test]
    fn parses_big_endian_signed_shorts_to_eof() {
        let t = IntTable::parse(&build(&[0, 7071, -175, i16::MAX, i16::MIN])).unwrap();
        assert_eq!(t.len(), 5);
        assert_eq!(t.values(), &[0, 7071, -175, i16::MAX, i16::MIN]);
        assert_eq!(t.get(1), Some(7071));
        assert_eq!(t.get(5), None);
    }

    #[test]
    fn empty_blob_is_an_empty_table() {
        assert!(IntTable::parse(&[]).unwrap().is_empty());
    }

    // ----- malformed / truncated rejection (rulebook R3) -----

    #[test]
    fn rejects_an_odd_byte_length() {
        let mut blob = build(&[1234, 5678]);
        blob.push(0x42); // one stray byte cannot be an i16
        assert_eq!(
            IntTable::parse(&blob),
            Err(FormatError::Invalid {
                at: 4,
                reason: "int table byte length is odd (not a whole number of i16)",
            })
        );
        // A one-byte blob is the minimal odd case.
        assert!(IntTable::parse(&[0x01]).is_err());
    }
}
