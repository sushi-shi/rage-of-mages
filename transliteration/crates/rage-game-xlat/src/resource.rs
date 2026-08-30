//! `Class.getResourceAsStream(String)` — the MIDlet's only byte source.
//!
//! This is the GAME's resource seam (the RoM-specific `getResourceAsStream`
//! byte contract), kept in the transliteration crate rather than the shared
//! device runtime: the neutral `j2me-me` provides Graphics/Canvas/Display and
//! the `Image.createImage` decoders, but the *name→bytes* provider is a
//! game/host concern (docs/DEVICE_RUNTIME.md §7).
//!
//! Every resource the baseline reads comes through `getResourceAsStream` on a
//! game class: the `res*.pak` sprite containers, `sincos/*.int`, the `.utf`
//! text files, `.map`/`scenes.pak`, `.txt` stat tables, and the two `.mid`
//! tracks. The transliteration calls this trait where the Java calls
//! `getResourceAsStream`; the *host* decides where the bytes come from (the
//! test backs it with the baseline jar, a production host with its content
//! store). The transliteration's own reversed loaders decode the bytes — this
//! trait hands over raw octets only.
//!
//! Java contract modeled: a missing entry returns `None` (Java returns `null`;
//! the callers' `try/catch` NPE paths do the rest) and a present entry returns
//! the complete byte payload (jar entries are small; the stream position state
//! lives in the transliteration's `DataInputStream` model, not here).

/// A provider of jar-resource bytes, standing in for
/// `Class.getResourceAsStream`.
pub trait Resources {
    /// The complete bytes of the named entry, or `None` when the entry does
    /// not exist (Java: a `null` `InputStream`). `name` is passed exactly as
    /// the game passes it — with or without a leading `/` (the game uses both
    /// `"/res/res0.pak"` and `"res/bgsound.mid"`); implementations should
    /// resolve both against the jar root via [`normalize_resource_name`]
    /// (the game's classes live in the default package, so a relative name
    /// resolves from the root too).
    fn resource_as_stream(&self, name: &str) -> Option<Vec<u8>>;
}

/// Resolve a `getResourceAsStream` name to a jar-root-relative path: strip the
/// optional leading `/`. With the game's classes in the default package both
/// forms address the same entry.
pub fn normalize_resource_name(name: &str) -> &str {
    name.strip_prefix('/').unwrap_or(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_normalize_to_jar_root() {
        assert_eq!(normalize_resource_name("/res/res0.pak"), "res/res0.pak");
        assert_eq!(
            normalize_resource_name("res/bgsound.mid"),
            "res/bgsound.mid"
        );
        assert_eq!(normalize_resource_name("/sincos/sin.int"), "sincos/sin.int");
    }
}
