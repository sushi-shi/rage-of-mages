//! `.map` — level terrain grids (`res/N.map`, `res/netN.map`, `res/101.map`).
//!
//! Reversed from the consumer (`docs/FORMATS.md`, rulebook R10): class `m`,
//! method `a(int, int)` reads the file with a raw `DataInputStream`, decoding
//! the header through the hand-rolled **little-endian** `u16` helper
//! `m.a(byte[], int)` (`b[i] + (b[i+1] << 8)`).
//!
//! ```text
//! offset  size            field
//! 0       u16 LE          width   (m.f203n)
//! 2       u16 LE          height  (m.f204o)
//! 4       width*height    cells   1 byte/cell, row-major: height rows × width cols
//! ```
//!
//! File length is exactly `4 + width*height` for every corpus map; anything else
//! is rejected. The stored cell bytes are kept **raw**: the `-128` shift, the
//! `m.f251o` 46-entry tile remap and the passability stamping are game-side
//! interpretation, not part of this container (every corpus cell lies in the
//! closed range `128..=173`, the remap's domain — asserted by the corpus gate,
//! not by this parser).

use crate::{FormatError, Reader};

/// A parsed terrain grid: the raw header plus the raw (un-remapped) cell bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Map {
    width: u16,
    height: u16,
    cells: Vec<u8>,
}

impl Map {
    /// Parse a `.map` blob. Rejects a blob whose length is not exactly
    /// `4 + width*height` with a typed error; never panics.
    pub fn parse(data: &[u8]) -> Result<Map, FormatError> {
        let mut r = Reader::new(data);
        let width = r.u16_le()?;
        let height = r.u16_le()?;
        let body = width as usize * height as usize;
        if data.len() != 4 + body {
            return Err(FormatError::Invalid {
                at: 0,
                reason: "map blob length is not 4 + width*height",
            });
        }
        let cells = r.bytes(body)?.to_vec();
        Ok(Map {
            width,
            height,
            cells,
        })
    }

    /// Grid width in cells (`m.f203n`).
    pub fn width(&self) -> u16 {
        self.width
    }

    /// Grid height in cells (`m.f204o`).
    pub fn height(&self) -> u16 {
        self.height
    }

    /// The raw row-major cell bytes (no `-128`/`f251o` remap applied).
    pub fn cells(&self) -> &[u8] {
        &self.cells
    }

    /// The raw cell byte at column `x`, row `y`, or `None` when out of range.
    pub fn cell(&self, x: u16, y: u16) -> Option<u8> {
        if x >= self.width || y >= self.height {
            return None;
        }
        Some(self.cells[y as usize * self.width as usize + x as usize])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a synthetic, hand-authored map blob (rulebook R1: no game bytes).
    fn build(width: u16, height: u16, cells: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&width.to_le_bytes());
        out.extend_from_slice(&height.to_le_bytes());
        out.extend_from_slice(cells);
        out
    }

    #[test]
    fn parses_a_row_major_grid_with_le_header() {
        // 3 wide, 2 tall: rows [128,129,130] then [140,141,142].
        let blob = build(3, 2, &[128, 129, 130, 140, 141, 142]);
        let map = Map::parse(&blob).unwrap();
        assert_eq!((map.width(), map.height()), (3, 2));
        assert_eq!(map.cells(), &[128, 129, 130, 140, 141, 142]);
        assert_eq!(map.cell(0, 0), Some(128));
        assert_eq!(map.cell(2, 0), Some(130));
        assert_eq!(map.cell(0, 1), Some(140));
        assert_eq!(map.cell(2, 1), Some(142));
    }

    #[test]
    fn cell_getter_rejects_out_of_range_coordinates() {
        let map = Map::parse(&build(2, 1, &[150, 151])).unwrap();
        assert_eq!(map.cell(2, 0), None);
        assert_eq!(map.cell(0, 1), None);
    }

    #[test]
    fn header_is_little_endian() {
        // width 0x0102 stored low byte first: 02 01.
        let mut blob = vec![0x02, 0x01, 0x01, 0x00];
        blob.resize(4 + 0x0102, 128u8);
        let map = Map::parse(&blob).unwrap();
        assert_eq!(map.width(), 0x0102);
        assert_eq!(map.height(), 1);
    }

    // ----- malformed / truncated rejection (rulebook R3) -----

    #[test]
    fn rejects_a_blob_one_byte_short() {
        let blob = build(3, 2, &[128, 129, 130, 140, 141, 142]);
        assert_eq!(
            Map::parse(&blob[..blob.len() - 1]),
            Err(FormatError::Invalid {
                at: 0,
                reason: "map blob length is not 4 + width*height",
            })
        );
    }

    #[test]
    fn rejects_a_blob_one_byte_long() {
        let mut blob = build(2, 2, &[128, 129, 130, 131]);
        blob.push(0x99);
        assert_eq!(
            Map::parse(&blob),
            Err(FormatError::Invalid {
                at: 0,
                reason: "map blob length is not 4 + width*height",
            })
        );
    }

    #[test]
    fn rejects_a_truncated_header() {
        assert!(matches!(
            Map::parse(&[0x03]),
            Err(FormatError::Truncated { .. })
        ));
        assert!(matches!(
            Map::parse(&[0x03, 0x00, 0x02]),
            Err(FormatError::Truncated { .. })
        ));
    }
}
