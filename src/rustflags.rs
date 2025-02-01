use std::env;
use std::ffi::OsString;

const CARGO_ENCODED_RUSTFLAGS: &str = "CARGO_ENCODED_RUSTFLAGS";
const RUSTFLAGS: &str = "RUSTFLAGS";
const IGNORED_LINTS: &[&str] = &["dead_code"];

pub(crate) fn make_vec() -> Vec<&'static str> {
    let mut rustflags = vec!["--cfg", "trybuild"];

    for &lint in IGNORED_LINTS {
        rustflags.push("-A");
        rustflags.push(lint);
    }

    rustflags
}

pub(crate) fn envs() -> impl IntoIterator<Item = (&'static str, OsString)> {
    // The precedence of rustflags is:
    // 1. CARGO_ENCODED_RUSTFLAGS
    // 2. RUSTFLAGS
    // 3. target.<triple>.rustflags (CARGO_TARGET_<triple>_RUSTFLAGS) and target.<cfg>.rustflags
    // 4. build.rustflags (CARGO_BUILD_RUSTFLAGS)
    // Refs: https://doc.rust-lang.org/nightly/cargo/reference/config.html#buildrustflags
    // For now, skip 3 and 4 because 3 is complex and handling 4 without 3 incorrectly overwrite rustflags.
    // TODO: Consider using cargo-config2 crate that implements it.
    let (key, mut val, separator) = match env::var_os(CARGO_ENCODED_RUSTFLAGS) {
        Some(val) => (CARGO_ENCODED_RUSTFLAGS, val, "\x1f"),
        None => match env::var_os(RUSTFLAGS) {
            Some(val) => (RUSTFLAGS, val, " "),
            None => return None,
        },
    };

    for flag in make_vec() {
        if !val.is_empty() {
            val.push(separator);
        }
        val.push(flag);
    }

    Some((key, val))
}
