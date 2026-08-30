//! `java.io.DataInputStream` over jar-resource bytes — the JVM-primitive I/O
//! model the transliterated loaders read through.
//!
//! Implementation #1 support (like `jrandom`): this is `java.io` runtime
//! behavior, not game code. The baseline always builds
//! `new DataInputStream(cls.getResourceAsStream(name))`; the stream the game
//! then reads is fully buffered here (jar entries are small) with the exact
//! Java error surface the game's `try/catch` blocks discriminate on:
//!
//! - `getResourceAsStream` miss → the Java object wraps a **null** stream
//!   (`DataInputStream(null)` constructs fine); every subsequent read raises
//!   `NullPointerException` ([`JavaError::NullPointer`]). Class `o`'s ctor and
//!   class `j`'s `open_pak` rely on exactly this to fall into their catch arms.
//! - reading past the end → `EOFException` ([`JavaError::Io`]).
//! - `readUTF` decodes **modified UTF-8** (a `u16` byte length, then 1/2/3-byte
//!   sequences producing UTF-16 code units); malformed input →
//!   `UTFDataFormatException` ([`JavaError::Io`]).
//! - `skipBytes` never raises EOF: it skips at most what remains and returns
//!   the count actually skipped.

use j2me_jvm::JavaError;

/// A `DataInputStream` over an optional byte payload. `data == None` models
/// `new DataInputStream(null)` — construction succeeded, every read is an NPE.
#[derive(Debug, Clone)]
pub struct DataInput {
    data: Option<Vec<u8>>,
    pos: usize,
}

impl DataInput {
    /// `new DataInputStream(getResourceAsStream(...))` — never throws, even on
    /// a `null` underlying stream.
    pub fn new(data: Option<Vec<u8>>) -> DataInput {
        DataInput { data, pos: 0 }
    }

    fn payload(&self) -> Result<&Vec<u8>, JavaError> {
        // Reading through a DataInputStream wrapping null → NullPointerException.
        self.data.as_ref().ok_or(JavaError::NullPointer)
    }

    fn take(&mut self, n: usize) -> Result<&[u8], JavaError> {
        let pos = self.pos;
        let data = self.data.as_ref().ok_or(JavaError::NullPointer)?;
        if pos + n > data.len() {
            return Err(JavaError::Io("EOFException".to_string()));
        }
        self.pos += n;
        Ok(&data[pos..pos + n])
    }

    /// `readByte()`.
    pub fn read_byte(&mut self) -> Result<i8, JavaError> {
        Ok(self.take(1)?[0] as i8)
    }

    /// `readShort()` — big-endian `i16`.
    pub fn read_short(&mut self) -> Result<i16, JavaError> {
        let b = self.take(2)?;
        Ok(i16::from_be_bytes([b[0], b[1]]))
    }

    /// `readUnsignedShort()` — big-endian `u16` widened to `i32` (the `readUTF`
    /// length prefix).
    pub fn read_unsigned_short(&mut self) -> Result<i32, JavaError> {
        let b = self.take(2)?;
        Ok(u16::from_be_bytes([b[0], b[1]]) as i32)
    }

    /// `readInt()` — big-endian `i32`.
    pub fn read_int(&mut self) -> Result<i32, JavaError> {
        let b = self.take(4)?;
        Ok(i32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }

    /// `readFully(byte[] b)`.
    pub fn read_fully(&mut self, buf: &mut [i8]) -> Result<(), JavaError> {
        let n = buf.len();
        let src = self.take(n)?;
        for (dst, &s) in buf.iter_mut().zip(src) {
            *dst = s as i8;
        }
        Ok(())
    }

    /// `skipBytes(int n)` — skips at most `n`, never raises EOF, returns the
    /// count actually skipped (the underlying `ByteArrayInputStream.skip`
    /// contract; a null underlying stream still NPEs).
    pub fn skip_bytes(&mut self, n: i32) -> Result<i32, JavaError> {
        let len = self.payload()?.len();
        let want = if n < 0 { 0 } else { n as usize };
        let actual = want.min(len - self.pos);
        self.pos += actual;
        Ok(actual as i32)
    }

    /// `readUTF()` — modified UTF-8: `u16` length prefix, then 1/2/3-byte
    /// sequences yielding UTF-16 code units. Malformed sequences and unpaired
    /// surrogates raise `UTFDataFormatException` — loud, never a silent
    /// substitute character (R10).
    pub fn read_utf(&mut self) -> Result<String, JavaError> {
        let len = self.read_unsigned_short()? as usize;
        let bytes: Vec<u8> = self.take(len)?.to_vec();
        let mut units: Vec<u16> = Vec::with_capacity(len);
        let mut i = 0usize;
        while i < len {
            let b0 = bytes[i] as u32;
            match b0 >> 4 {
                0..=7 => {
                    units.push(b0 as u16);
                    i += 1;
                }
                12 | 13 => {
                    if i + 1 >= len {
                        return Err(JavaError::Io("UTFDataFormatException".to_string()));
                    }
                    let b1 = bytes[i + 1] as u32;
                    if b1 & 0xC0 != 0x80 {
                        return Err(JavaError::Io("UTFDataFormatException".to_string()));
                    }
                    units.push((((b0 & 0x1F) << 6) | (b1 & 0x3F)) as u16);
                    i += 2;
                }
                14 => {
                    if i + 2 >= len {
                        return Err(JavaError::Io("UTFDataFormatException".to_string()));
                    }
                    let b1 = bytes[i + 1] as u32;
                    let b2 = bytes[i + 2] as u32;
                    if b1 & 0xC0 != 0x80 || b2 & 0xC0 != 0x80 {
                        return Err(JavaError::Io("UTFDataFormatException".to_string()));
                    }
                    units.push((((b0 & 0x0F) << 12) | ((b1 & 0x3F) << 6) | (b2 & 0x3F)) as u16);
                    i += 3;
                }
                _ => return Err(JavaError::Io("UTFDataFormatException".to_string())),
            }
        }
        // Java Strings are UTF-16 unit sequences; the game's .utf resources are
        // BMP text, so a well-formed record converts losslessly. An unpaired
        // surrogate would be representable in Java but not in a Rust String —
        // refuse loudly rather than substitute (R10).
        String::from_utf16(&units)
            .map_err(|_| JavaError::Io("readUTF: unpaired surrogate (unrepresentable)".to_string()))
    }

    /// `close()` — a no-op on the buffered payload; still NPEs on a stream
    /// wrapping null (`new DataInputStream(null).close()` throws in Java).
    pub fn close(&mut self) -> Result<(), JavaError> {
        self.payload()?;
        Ok(())
    }
}

/// `Integer.parseInt(String, 10)` — exactly the radix-10 path class `aj` calls
/// on `common.utf` count records. A malformed count is a
/// `NumberFormatException`, which the (unguarded) caller turns into a faithful
/// panic.
pub fn parse_int_radix10(s: &str) -> Result<i32, JavaError> {
    let bytes: Vec<char> = s.chars().collect();
    if bytes.is_empty() {
        return Err(JavaError::IllegalArgument("NumberFormatException: empty"));
    }
    let (neg, start) = match bytes[0] {
        '-' => (true, 1),
        '+' => (false, 1),
        _ => (false, 0),
    };
    if start == bytes.len() {
        return Err(JavaError::IllegalArgument(
            "NumberFormatException: sign only",
        ));
    }
    // Java accumulates negatively to hold Integer.MIN_VALUE; digits outside
    // 0-9 or overflow raise NumberFormatException.
    let mut acc: i32 = 0;
    for &c in &bytes[start..] {
        let d = c.to_digit(10).ok_or(JavaError::IllegalArgument(
            "NumberFormatException: non-digit",
        ))? as i32;
        acc = acc.checked_mul(10).and_then(|v| v.checked_sub(d)).ok_or(
            JavaError::IllegalArgument("NumberFormatException: overflow"),
        )?;
    }
    if neg {
        Ok(acc)
    } else {
        acc.checked_neg().ok_or(JavaError::IllegalArgument(
            "NumberFormatException: overflow",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn null_stream_constructs_then_npes_on_read() {
        // The o/j catch arms depend on exactly this shape.
        let mut s = DataInput::new(None);
        assert_eq!(s.read_byte(), Err(JavaError::NullPointer));
        assert_eq!(s.read_int(), Err(JavaError::NullPointer));
        assert_eq!(s.close(), Err(JavaError::NullPointer));
    }

    #[test]
    fn big_endian_primitives_and_eof() {
        let mut s = DataInput::new(Some(vec![0x01, 0x02, 0x03, 0x04, 0xFF]));
        assert_eq!(s.read_int(), Ok(0x0102_0304));
        assert_eq!(s.read_byte(), Ok(-1));
        assert!(matches!(s.read_byte(), Err(JavaError::Io(_)))); // EOFException
    }

    #[test]
    fn read_utf_decodes_modified_utf8_cyrillic() {
        // "АБ" in modified UTF-8: len=4, then two 2-byte sequences (as found at
        // the head of the res0.pak font descriptor entry).
        let mut s = DataInput::new(Some(vec![0x00, 0x04, 0xD0, 0x90, 0xD0, 0x91]));
        assert_eq!(s.read_utf().unwrap(), "АБ");
    }

    #[test]
    fn read_utf_rejects_malformed_sequences() {
        // A lone continuation byte where a lead byte belongs.
        let mut s = DataInput::new(Some(vec![0x00, 0x01, 0x80]));
        assert!(matches!(s.read_utf(), Err(JavaError::Io(_))));
    }

    #[test]
    fn skip_bytes_clamps_and_reports() {
        let mut s = DataInput::new(Some(vec![1, 2, 3]));
        assert_eq!(s.skip_bytes(2), Ok(2));
        assert_eq!(s.skip_bytes(5), Ok(1)); // only 1 remained — no EOF raise
        assert_eq!(s.skip_bytes(5), Ok(0));
    }

    #[test]
    fn parse_int_matches_java_radix10() {
        assert_eq!(parse_int_radix10("42"), Ok(42));
        assert_eq!(parse_int_radix10("-7"), Ok(-7));
        assert_eq!(parse_int_radix10("2147483647"), Ok(i32::MAX));
        assert_eq!(parse_int_radix10("-2147483648"), Ok(i32::MIN));
        assert!(parse_int_radix10("2147483648").is_err());
        assert!(parse_int_radix10("12a").is_err());
        assert!(parse_int_radix10("").is_err());
    }
}
