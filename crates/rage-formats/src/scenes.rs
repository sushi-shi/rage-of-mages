//! `res/scenes.pak` — per-level scripting data. **Not** an XOR pak: despite the
//! extension this is a big-endian nested-`i32` structure, distinct from
//! [`crate::pak`].
//!
//! Reversed from the consumer (`docs/FORMATS.md`, rulebook R10): class `m`,
//! method `g(int i)` (`i` = level index) reads with a raw `DataInputStream`,
//! `readInt()` throughout (**big-endian**). The file is a flat sequence of
//! per-level scenes with no top-level count (scene index = level index; the
//! loader reaches scene `i` by skipping `i` scenes). Each scene is exactly
//! **three groups**, each group a ragged int-array table:
//!
//! ```text
//! per scene, 3 groups; each group:
//!   i32 BE  rows
//!   rows × ( i32 BE len ; len × i32 BE )
//! ```
//!
//! This module is **framing-only**. The three groups populate `m.f197e`,
//! `m.f196d`, `m.f195c`, and group 3's rows are spawn records (`row[0]` = type,
//! `row[1..]` fed to `new n(...)` — x, y, w, h, flags, …), but the per-int
//! semantics are **uncertain** and deliberately not interpreted here: the
//! parser exposes the raw ragged tables and nothing more. Any framing mismatch
//! (a scene cut mid-group, a negative count, bytes that do not tile to exact
//! EOF) is rejected.

use crate::{FormatError, Reader};

/// One ragged int-array table: `rows × (len; len ints)`.
pub type Group = Vec<Vec<i32>>;

/// One per-level scene: exactly three groups (→ `m.f197e`, `m.f196d`,
/// `m.f195c`, in file order).
pub type Scene = [Group; 3];

/// The parsed `scenes.pak`: every scene, in file order (index = level index).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scenes {
    scenes: Vec<Scene>,
}

impl Scenes {
    /// Parse a `scenes.pak` blob: scenes of three groups each, until the stream
    /// tiles to exact EOF. Rejects a negative row/length count, truncation
    /// mid-scene, or any non-exact tiling with a typed error; never panics.
    pub fn parse(data: &[u8]) -> Result<Scenes, FormatError> {
        let mut r = Reader::new(data);
        let mut scenes = Vec::new();
        while r.remaining() > 0 {
            let mut scene: Scene = [Vec::new(), Vec::new(), Vec::new()];
            for group in &mut scene {
                *group = parse_group(&mut r)?;
            }
            scenes.push(scene);
        }
        Ok(Scenes { scenes })
    }

    /// All scenes, in file order.
    pub fn scenes(&self) -> &[Scene] {
        &self.scenes
    }

    /// Number of scenes.
    pub fn len(&self) -> usize {
        self.scenes.len()
    }

    /// Whether the file held no scenes.
    pub fn is_empty(&self) -> bool {
        self.scenes.is_empty()
    }

    /// One scene by level index, or `None` when out of range.
    pub fn get(&self, index: usize) -> Option<&Scene> {
        self.scenes.get(index)
    }
}

/// Read one group: `i32 BE rows`, then `rows × (i32 BE len; len × i32 BE)`.
fn parse_group(r: &mut Reader<'_>) -> Result<Group, FormatError> {
    let at = r.position();
    let rows = r.i32_be()?;
    if rows < 0 {
        return Err(FormatError::Invalid {
            at,
            reason: "negative row count in scenes group",
        });
    }
    let rows = rows as usize;
    let mut group = Vec::with_capacity(rows.min(1 << 16));
    for _ in 0..rows {
        let at = r.position();
        let len = r.i32_be()?;
        if len < 0 {
            return Err(FormatError::Invalid {
                at,
                reason: "negative row length in scenes group",
            });
        }
        let len = len as usize;
        let mut row = Vec::with_capacity(len.min(1 << 16));
        for _ in 0..len {
            row.push(r.i32_be()?);
        }
        group.push(row);
    }
    Ok(group)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn push_i32(out: &mut Vec<u8>, v: i32) {
        out.extend_from_slice(&v.to_be_bytes());
    }

    /// Append one group (`rows`, then each row as `len; ints`).
    fn push_group(out: &mut Vec<u8>, rows: &[&[i32]]) {
        push_i32(out, rows.len() as i32);
        for row in rows {
            push_i32(out, row.len() as i32);
            for &v in *row {
                push_i32(out, v);
            }
        }
    }

    /// Build a synthetic, hand-authored blob of scenes (rulebook R1: no game
    /// bytes); each scene is three groups.
    fn build(scenes: &[[&[&[i32]]; 3]]) -> Vec<u8> {
        let mut out = Vec::new();
        for scene in scenes {
            for group in scene {
                push_group(&mut out, group);
            }
        }
        out
    }

    #[test]
    fn parses_scenes_of_three_ragged_groups_to_exact_eof() {
        let blob = build(&[
            [&[&[0, 3, 0, 8], &[7]], &[], &[&[1, -2, 3]]],
            [&[], &[&[42]], &[]],
        ]);
        let s = Scenes::parse(&blob).unwrap();
        assert_eq!(s.len(), 2);
        let scene0 = s.get(0).unwrap();
        assert_eq!(scene0[0], vec![vec![0, 3, 0, 8], vec![7]]);
        assert!(scene0[1].is_empty());
        assert_eq!(scene0[2], vec![vec![1, -2, 3]]);
        let scene1 = s.get(1).unwrap();
        assert_eq!(scene1[1], vec![vec![42]]);
        assert_eq!(s.get(2), None);
    }

    #[test]
    fn empty_blob_is_zero_scenes() {
        assert!(Scenes::parse(&[]).unwrap().is_empty());
    }

    // ----- malformed / truncated rejection (rulebook R3) -----

    #[test]
    fn rejects_truncation_mid_scene() {
        let blob = build(&[[&[&[1, 2]], &[&[3]], &[]]]);
        // Cut one byte off the final group's rows word.
        assert!(matches!(
            Scenes::parse(&blob[..blob.len() - 1]),
            Err(FormatError::Truncated { .. })
        ));
        // Cut a whole trailing group: the scene is no longer three groups.
        assert!(matches!(
            Scenes::parse(&blob[..blob.len() - 4]),
            Err(FormatError::Truncated { .. })
        ));
    }

    #[test]
    fn rejects_trailing_bytes_that_do_not_tile_into_a_scene() {
        let mut blob = build(&[[&[], &[], &[]]]);
        blob.extend_from_slice(&[0, 0]); // 2 bytes cannot start a new scene
        assert!(matches!(
            Scenes::parse(&blob),
            Err(FormatError::Truncated { .. })
        ));
    }

    #[test]
    fn rejects_negative_row_count_and_row_length() {
        let mut blob = build(&[[&[&[5]], &[], &[]]]);
        blob[0] = 0xFF; // group 0's rows word sign bit
        assert_eq!(
            Scenes::parse(&blob),
            Err(FormatError::Invalid {
                at: 0,
                reason: "negative row count in scenes group",
            })
        );

        let mut blob = build(&[[&[&[5]], &[], &[]]]);
        blob[4] = 0xFF; // row 0's len word sign bit
        assert_eq!(
            Scenes::parse(&blob),
            Err(FormatError::Invalid {
                at: 4,
                reason: "negative row length in scenes group",
            })
        );
    }
}
