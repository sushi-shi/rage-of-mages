//! `.utf` — localization string tables (`res/1.utf` … `res/skills.utf`).
//!
//! Reversed from the consumer (`docs/FORMATS.md`, rulebook R10/R11): every file
//! opens via `ae.a(InputStream)` → a plain `DataInputStream`, and every consumer
//! reads it with `readUTF()` (`ae.m3a(DataInputStream)` = one record). The
//! container is a **flat concatenation of Java `readUTF()` records** tiling the
//! whole file to exact EOF — no count header, no offset table, no encoding byte:
//!
//! ```text
//! per record:
//!   u16 BE       byte_length   (0 = an empty record, used as a delimiter)
//!   byte_length  Java modified-UTF-8 payload
//! ```
//!
//! Empty records are *valid records* (the per-file grammars use them as section
//! delimiters) and are kept as empty strings. The per-file record grammars
//! (count-prefixed groups in `common.utf`, double-empty dialog blocks, …) are
//! consumer-side; this parser stops at the flat record stream.
//!
//! The payload decoder mirrors `DataInputStream.readUTF` exactly: 1/2/3-byte
//! forms, including Java's quirks — a raw `0x00` byte decodes to U+0000, the
//! two-byte `C0 80` form of U+0000 is accepted (Java tolerates overlong two-byte
//! encodings), and surrogates arrive as 3-byte `0xED …` code units. Because Rust
//! strings are UTF-8, surrogate *pairs* are combined into the supplementary
//! character and **unpaired** surrogates are rejected (the corpus is BMP-only,
//! so this never fires on real data). Bad lead/continuation bytes and a
//! partial character at the end of a record are rejected, as Java does.

use crate::{FormatError, Reader};

/// A parsed `.utf` table: the flat record stream, in file order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Utf {
    records: Vec<String>,
}

impl Utf {
    /// Parse a `.utf` blob: `u16 BE length` + modified-UTF-8 records until the
    /// stream tiles to exact EOF. A stream that does not tile (a stray trailing
    /// byte, or a final record over-running EOF) is rejected with a typed
    /// error; never panics.
    pub fn parse(data: &[u8]) -> Result<Utf, FormatError> {
        let mut r = Reader::new(data);
        let mut records = Vec::new();
        while r.remaining() > 0 {
            let len = r.u16_be()? as usize;
            let at = r.position();
            let raw = r.bytes(len)?;
            records.push(decode_modified_utf8(at, raw)?);
        }
        Ok(Utf { records })
    }

    /// All records, in file order (empty delimiters included).
    pub fn records(&self) -> &[String] {
        &self.records
    }

    /// Number of records.
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Whether the table holds no records.
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// One record by index, or `None` when out of range.
    pub fn get(&self, index: usize) -> Option<&str> {
        self.records.get(index).map(String::as_str)
    }
}

/// Decode one record body as Java modified-UTF-8 (`DataInputStream.readUTF`
/// semantics). `at` is the record body's absolute offset, used in errors.
fn decode_modified_utf8(at: usize, bytes: &[u8]) -> Result<String, FormatError> {
    // Pass 1 — bytes to UTF-16 code units, exactly as readUTF builds chars.
    let mut units: Vec<(u16, usize)> = Vec::with_capacity(bytes.len());
    let mut i = 0usize;
    while i < bytes.len() {
        let b = bytes[i];
        match b >> 4 {
            // 0xxxxxxx — one byte (Java also takes a raw 0x00 here).
            0x0..=0x7 => {
                units.push((b as u16, at + i));
                i += 1;
            }
            // 110xxxxx 10xxxxxx — two bytes (overlong forms accepted, as Java
            // does; this is how U+0000 is normally carried, as C0 80).
            0xC | 0xD => {
                if i + 2 > bytes.len() {
                    return Err(FormatError::Invalid {
                        at: at + i,
                        reason: "modified-UTF-8: partial character at end of record",
                    });
                }
                let b2 = bytes[i + 1];
                if b2 & 0xC0 != 0x80 {
                    return Err(FormatError::Invalid {
                        at: at + i + 1,
                        reason: "modified-UTF-8: bad continuation byte",
                    });
                }
                units.push(((((b & 0x1F) as u16) << 6) | (b2 & 0x3F) as u16, at + i));
                i += 2;
            }
            // 1110xxxx 10xxxxxx 10xxxxxx — three bytes (surrogates included).
            0xE => {
                if i + 3 > bytes.len() {
                    return Err(FormatError::Invalid {
                        at: at + i,
                        reason: "modified-UTF-8: partial character at end of record",
                    });
                }
                let (b2, b3) = (bytes[i + 1], bytes[i + 2]);
                if b2 & 0xC0 != 0x80 || b3 & 0xC0 != 0x80 {
                    return Err(FormatError::Invalid {
                        at: at + i + if b2 & 0xC0 != 0x80 { 1 } else { 2 },
                        reason: "modified-UTF-8: bad continuation byte",
                    });
                }
                let u =
                    (((b & 0x0F) as u16) << 12) | (((b2 & 0x3F) as u16) << 6) | (b3 & 0x3F) as u16;
                units.push((u, at + i));
                i += 3;
            }
            // 10xxxxxx (stray continuation) or 1111xxxx — rejected, as Java does.
            _ => {
                return Err(FormatError::Invalid {
                    at: at + i,
                    reason: "modified-UTF-8: invalid lead byte",
                });
            }
        }
    }

    // Pass 2 — UTF-16 code units to a Rust string, pairing surrogates.
    let mut out = String::with_capacity(bytes.len());
    let mut j = 0usize;
    while j < units.len() {
        let (u, pos) = units[j];
        match u {
            0xD800..=0xDBFF => {
                let Some(&(lo, _)) = units.get(j + 1) else {
                    return Err(FormatError::Invalid {
                        at: pos,
                        reason: "modified-UTF-8: unpaired high surrogate",
                    });
                };
                if !(0xDC00..=0xDFFF).contains(&lo) {
                    return Err(FormatError::Invalid {
                        at: pos,
                        reason: "modified-UTF-8: unpaired high surrogate",
                    });
                }
                let scalar = 0x10000 + (((u as u32) - 0xD800) << 10) + ((lo as u32) - 0xDC00);
                // 0x10000..=0x10FFFF and never a surrogate: always a valid char,
                // but keep the no-panic contract with a typed error anyway.
                out.push(char::from_u32(scalar).ok_or(FormatError::Invalid {
                    at: pos,
                    reason: "modified-UTF-8: surrogate pair outside Unicode",
                })?);
                j += 2;
            }
            0xDC00..=0xDFFF => {
                return Err(FormatError::Invalid {
                    at: pos,
                    reason: "modified-UTF-8: unpaired low surrogate",
                });
            }
            _ => {
                // A BMP non-surrogate code unit is always a valid char.
                out.push(char::from_u32(u as u32).ok_or(FormatError::Invalid {
                    at: pos,
                    reason: "modified-UTF-8: invalid scalar value",
                })?);
                j += 1;
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Append one record (`u16 BE len` + raw payload bytes) to a stream.
    fn push_record(out: &mut Vec<u8>, payload: &[u8]) {
        out.extend_from_slice(&(payload.len() as u16).to_be_bytes());
        out.extend_from_slice(payload);
    }

    /// Build a synthetic, hand-authored stream (rulebook R1: no game bytes).
    fn build(payloads: &[&[u8]]) -> Vec<u8> {
        let mut out = Vec::new();
        for p in payloads {
            push_record(&mut out, p);
        }
        out
    }

    #[test]
    fn parses_ascii_cyrillic_and_empty_delimiter_records() {
        // BMP text without NUL encodes identically in standard and modified
        // UTF-8, so `str::as_bytes` authors valid payloads.
        let blob = build(&[b"Hello", "Аллоды".as_bytes(), b"", b"end"]);
        let t = Utf::parse(&blob).unwrap();
        assert_eq!(t.len(), 4);
        assert_eq!(t.get(0), Some("Hello"));
        assert_eq!(t.get(1), Some("Аллоды"));
        assert_eq!(t.get(2), Some("")); // empty record kept as a delimiter
        assert_eq!(t.get(3), Some("end"));
        assert_eq!(t.get(4), None);
    }

    #[test]
    fn empty_stream_is_zero_records() {
        assert!(Utf::parse(&[]).unwrap().is_empty());
    }

    #[test]
    fn decodes_the_two_byte_nul_form_and_a_raw_nul() {
        // Java writes U+0000 as C0 80 but its reader also takes a raw 00.
        let blob = build(&[&[0xC0, 0x80], &[0x00]]);
        let t = Utf::parse(&blob).unwrap();
        assert_eq!(t.get(0), Some("\u{0}"));
        assert_eq!(t.get(1), Some("\u{0}"));
    }

    #[test]
    fn combines_a_surrogate_pair_into_the_supplementary_character() {
        // U+10437 (𐐷) = UTF-16 D801 DC37 = modified UTF-8 ED A0 81 ED B0 B7.
        let blob = build(&[&[0xED, 0xA0, 0x81, 0xED, 0xB0, 0xB7]]);
        assert_eq!(Utf::parse(&blob).unwrap().get(0), Some("\u{10437}"));
    }

    // ----- malformed / truncated rejection (rulebook R3) -----

    #[test]
    fn rejects_a_final_record_that_overruns_eof() {
        let mut blob = build(&[b"ok"]);
        push_record(&mut blob, b"tail");
        // Cut one payload byte: the declared length now over-runs the stream.
        assert!(matches!(
            Utf::parse(&blob[..blob.len() - 1]),
            Err(FormatError::Truncated { .. })
        ));
    }

    #[test]
    fn rejects_a_stray_trailing_byte() {
        let mut blob = build(&[b"ok"]);
        blob.push(0x07); // one byte cannot hold the next record's u16 length
        assert!(matches!(
            Utf::parse(&blob),
            Err(FormatError::Truncated { .. })
        ));
    }

    #[test]
    fn rejects_an_invalid_lead_byte() {
        for lead in [0x80u8, 0xBF, 0xF0, 0xFF] {
            let blob = build(&[&[lead]]);
            assert_eq!(
                Utf::parse(&blob),
                Err(FormatError::Invalid {
                    at: 2,
                    reason: "modified-UTF-8: invalid lead byte",
                }),
                "lead byte {lead:#04x}"
            );
        }
    }

    #[test]
    fn rejects_a_bad_continuation_byte() {
        // 0xD0 expects a 10xxxxxx continuation; 0x28 is not one.
        let blob = build(&[&[0xD0, 0x28]]);
        assert_eq!(
            Utf::parse(&blob),
            Err(FormatError::Invalid {
                at: 3,
                reason: "modified-UTF-8: bad continuation byte",
            })
        );
    }

    #[test]
    fn rejects_a_partial_character_at_record_end() {
        for payload in [&[0xD0u8][..], &[0xE2, 0x82][..]] {
            let blob = build(&[payload]);
            assert!(matches!(
                Utf::parse(&blob),
                Err(FormatError::Invalid {
                    reason: "modified-UTF-8: partial character at end of record",
                    ..
                })
            ));
        }
    }

    #[test]
    fn rejects_unpaired_surrogates() {
        // A lone high surrogate, then a lone low surrogate.
        for (payload, reason) in [
            (
                &[0xEDu8, 0xA0, 0x81][..],
                "modified-UTF-8: unpaired high surrogate",
            ),
            (
                &[0xEDu8, 0xB0, 0xB7][..],
                "modified-UTF-8: unpaired low surrogate",
            ),
        ] {
            let blob = build(&[payload]);
            assert_eq!(
                Utf::parse(&blob),
                Err(FormatError::Invalid { at: 2, reason })
            );
        }
    }
}
