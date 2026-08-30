//! Class `j` — `ResourceLoader` (symbols.toml): the `.pak` XOR-container walk
//! (+ the raw `.mid` slurp, next slice).
//!
//! Implementation #1: strict transliteration. Provably the same program as the
//! recovered Java, NOT idiomatic Rust — do not refactor (docs/TRANSLITERATION.md).
//! Source: `_reference/decompile/176x220/{jadx,cfr}/j.java`; FORMATS.md res*.pak.
//! Numeric shapes verified against `_reference/numeric-shapes.json` (R8):
//! `j.a(String,B,Class)V` = [iinc]; `j.a()[B` = [ixor,i2b,iinc,iadd]
//! (the XOR-narrow inside the loop, then the `f121a++` field iadd);
//! `j.a(I)V` = [iadd,iinc]; `j.a()V` = [].
//!
//! The pak layout the bytecode defines: BE `i32` entry count, that many BE
//! `i32` entry lengths, then the entry payloads back to back, every payload
//! byte XOR `0x53`. This loader is the game's own decoder; the independent
//! `rage_formats::pak` parser is a TEST-ONLY oracle (Cargo dev-dependency).

use crate::jio::DataInput;
use crate::resource::Resources;
use j2me_jvm::JavaError;

/// The `j` singleton's fields (owned by `m.f140a` on the [`crate::Game`]).
#[derive(Debug, Clone, Default)]
pub struct ResourceLoader {
    /// `j.a: Ljava/io/DataInputStream;` — the open pak stream (`None` = the
    /// Java field is null: before the first open / after `close_pak`).
    pub a: Option<DataInput>,
    /// `j.f119a (jadx): [I` — the entry-length directory (`None` = null:
    /// before a successful open / after close).
    pub f119a: Option<Vec<i32>>,
    /// `j.f120a (jadx): B` — the XOR key (callers pass `(byte) 0x53`).
    pub f120a: i8,
    /// `j.f121a (jadx): I` — the current entry index into the directory.
    pub f121a: i32,
}

/// `j.a (Ljava/lang/String;BLjava/lang/Class;)V` — open_pak: store the key,
/// reset the index, open the stream and read the `i32`-count + `i32` length
/// directory. The whole read is inside `try/catch(Exception){}`: a missing
/// resource NPEs on the first `readInt` and leaves `f119a` null.
#[allow(clippy::needless_range_loop)] // faithful to the Java directory loop
pub fn open_pak(l: &mut ResourceLoader, resources: &dyn Resources, name: &str, key: i8) {
    l.f120a = key;
    l.f121a = 0;
    let mut body = || -> Result<(), JavaError> {
        l.a = Some(DataInput::new(resources.resource_as_stream(name)));
        let stream = l.a.as_mut().expect("just stored");
        let count = stream.read_int()?;
        // `new int[count]` — a negative count is a NegativeArraySizeException,
        // also swallowed by the catch.
        if count < 0 {
            return Err(JavaError::IllegalArgument("NegativeArraySizeException"));
        }
        let mut dir = vec![0i32; count as usize];
        for i in 0..dir.len() {
            dir[i] = stream.read_int()?;
        }
        l.f119a = Some(dir);
        Ok(())
    };
    let _ = body(); // j.a catch arm: swallow
}

/// `j.a ()[B` — read_entry: the current entry's bytes, each XOR the key. The
/// array allocation `new byte[f119a[f121a]]` is UNGUARDED — a null directory
/// (failed open) or an index past the end is the fatal NPE/AIOOBE the MIDlet
/// would have died of, so it panics here. Each `readByte` inside the loop has
/// its own catch that only prints (the byte then stays 0).
#[allow(clippy::needless_range_loop)] // faithful to the Java index loop
pub fn read_entry(l: &mut ResourceLoader) -> Vec<i8> {
    let dir = l
        .f119a
        .as_ref()
        .expect("NullPointerException: j.a()[B with no open pak directory");
    let len = dir[l.f121a as usize]; // AIOOBE panic faithful when past the end
    let mut arr = vec![0i8; len as usize];
    let stream =
        l.a.as_mut()
            .expect("NullPointerException: j.a()[B with a null pak stream");
    for i in 0..arr.len() {
        match stream.read_byte() {
            // (readByte ^ key) is int arithmetic, narrowed back: ixor, i2b.
            Ok(b) => arr[i] = ((b as i32) ^ (l.f120a as i32)) as i8,
            // The per-byte catch arm: print and continue (the slot stays 0).
            Err(e) => println!("{e}"),
        }
    }
    l.f121a = l.f121a.wrapping_add(1); // iadd (field increment)
    arr
}

/// `j.a (I)V` — skip_entries: `skipBytes` past `i` entries, advancing the
/// directory index; any failure prints and RETURNS EARLY (the shipped shape —
/// remaining entries stay unskipped).
pub fn skip_entries(l: &mut ResourceLoader, i: i32) {
    let mut i2 = 0;
    while i2 < i {
        let mut body = || -> Result<(), JavaError> {
            let dir = l.f119a.as_ref().ok_or(JavaError::NullPointer)?;
            let len = *dir
                .get(l.f121a as usize)
                .ok_or(JavaError::ArrayIndexOutOfBounds {
                    index: l.f121a,
                    length: dir.len() as i32,
                })?;
            let stream = l.a.as_mut().ok_or(JavaError::NullPointer)?;
            stream.skip_bytes(len)?;
            l.f121a = l.f121a.wrapping_add(1); // iadd (field increment)
            Ok(())
        };
        if let Err(e) = body() {
            // j.a(I)V catch arm: print, then `return` — not continue.
            println!("{e}");
            return;
        }
        i2 += 1; // iinc
    }
}

/// `j.a ()V` — close_pak: null the directory, close the stream (a close
/// failure only prints), null the stream field.
pub fn close_pak(l: &mut ResourceLoader) {
    l.f119a = None;
    match l.a.as_mut() {
        Some(stream) => {
            if let Err(e) = stream.close() {
                println!("{e}");
            }
        }
        // `this.a.close()` with the field itself null → NPE, caught + printed.
        None => println!("{}", JavaError::NullPointer),
    }
    l.a = None;
}

// TODO(next-slice): `j.a (Ljava/lang/String;Ljava/lang/Class;)[B` —
// slurp_resource, the raw `.mid` byte-count-then-readFully slurp (no XOR). Its
// only boot caller sits inside `m.<init>`'s stubbed MMAPI try-block, which does
// not affect first-frame pixels.

#[cfg(test)]
mod tests {
    use super::*;

    /// Authored pak bytes (never game data): count, lengths, XOR'd payloads.
    struct FakePak(Vec<u8>);
    impl Resources for FakePak {
        fn resource_as_stream(&self, name: &str) -> Option<Vec<u8>> {
            if name == "/fake.pak" {
                Some(self.0.clone())
            } else {
                None
            }
        }
    }

    fn pak_with(entries: &[&[u8]], key: u8) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&(entries.len() as i32).to_be_bytes());
        for e in entries {
            out.extend_from_slice(&(e.len() as i32).to_be_bytes());
        }
        for e in entries {
            out.extend(e.iter().map(|&b| b ^ key));
        }
        out
    }

    #[test]
    fn open_read_skip_read_walks_the_directory() {
        let res = FakePak(pak_with(&[b"one", b"two2", b"three"], 0x53));
        let mut l = ResourceLoader::default();
        open_pak(&mut l, &res, "/fake.pak", 0x53);
        assert_eq!(l.f119a.as_deref(), Some(&[3i32, 4, 5][..]));
        let e0 = read_entry(&mut l);
        assert_eq!(e0, b"one".iter().map(|&b| b as i8).collect::<Vec<_>>());
        skip_entries(&mut l, 1); // skip "two2"
        let e2 = read_entry(&mut l);
        assert_eq!(e2, b"three".iter().map(|&b| b as i8).collect::<Vec<_>>());
        close_pak(&mut l);
        assert!(l.a.is_none() && l.f119a.is_none());
    }

    #[test]
    fn missing_resource_leaves_a_null_directory() {
        let res = FakePak(Vec::new());
        let mut l = ResourceLoader::default();
        open_pak(&mut l, &res, "/absent.pak", 0x53);
        assert!(l.f119a.is_none(), "the catch arm leaves f119a null");
    }

    #[test]
    #[should_panic(expected = "NullPointerException")]
    fn read_entry_after_failed_open_dies_like_the_midlet() {
        let res = FakePak(Vec::new());
        let mut l = ResourceLoader::default();
        open_pak(&mut l, &res, "/absent.pak", 0x53);
        let _ = read_entry(&mut l); // unguarded NPE in Java → faithful panic
    }
}
