use std::env;
use std::process::Command;

const CARGO_ENCODED_RUSTFLAGS: &str = "CARGO_ENCODED_RUSTFLAGS";
const CARGO_ENCODED_RUSTFLAGS_SEP: &str = "\x1f";
const RUSTFLAGS: &str = "RUSTFLAGS";
const RUSTFLAGS_SEP: &str = " ";
const CARGO_BUILD_RUSTFLAGS: &str = "CARGO_BUILD_RUSTFLAGS";
const IGNORED_LINTS: &[&str] = &["dead_code"];

fn make_vec(extra_rustflags: &[&'static str]) -> Vec<&'static str> {
    let mut rustflags = vec!["--cfg", "trybuild", "--verbose"];

    for &lint in IGNORED_LINTS {
        rustflags.push("-A");
        rustflags.push(lint);
    }

    rustflags.extend(extra_rustflags);

    rustflags
}

pub(crate) fn set(cmd: &mut Command, target: &str, extra_rustflags: &[&'static str]) {
    let rustflags = make_vec(extra_rustflags);

    // The precedence of rustflags is:
    //
    // 1. `CARGO_ENCODED_RUSTFLAGS` env var
    // 2. `RUSTFLAGS` env var
    // 3. `target.<triple>.rustflags` (`CARGO_TARGET_<triple>_RUSTFLAGS` env var) and `target.<cfg>.rustflags`
    // 4. `build.rustflags` (`CARGO_BUILD_RUSTFLAGS` env var)
    //
    // Refs: https://doc.rust-lang.org/nightly/cargo/reference/config.html#buildrustflags
    //
    // - For 1. and 2., we get the existing one and append our rustflags to it.
    // - For 3., config passed via `--config` will be merged with existing config
    //   and env var, so pass our rustflags using the `--config` flag.
    // - For 4., config passed via `--config` will be merged with existing config,
    //   but not with existing env var (env var takes precedence), so:
    //   - If env var is set: we get the existing one and append our rustflags to it.
    //   - Otherwise: pass our rustflags using the `--config` flag.
    //
    // Note:
    // - 3\. applies only to the target and not to the host, so if 1. and 2. are
    //   not set, apply 3. and 4. (Using something like `--config=target.'cfg(all())'.rustflags`
    //   might make 3 alone sufficient.)
    // - Since 3. takes precedence over 4., in environments where 1. to 3. is
    //   not set but 4. is set, existing settings will be overwritten. To avoid this, we need to load and merge the config files, like cargo-config2 crate does.
    let (key, separator, mut val) = match env::var_os(CARGO_ENCODED_RUSTFLAGS) {
        Some(val) => (CARGO_ENCODED_RUSTFLAGS, CARGO_ENCODED_RUSTFLAGS_SEP, val),
        None => match env::var_os(RUSTFLAGS) {
            Some(val) => (RUSTFLAGS, RUSTFLAGS_SEP, val),
            None => {
                let rustflags = toml::Value::try_from(rustflags.clone()).unwrap();
                cmd.arg(format!("--config=target.{target}.rustflags={rustflags}"));
                match env::var_os(CARGO_BUILD_RUSTFLAGS) {
                    Some(val) => (CARGO_BUILD_RUSTFLAGS, RUSTFLAGS_SEP, val),
                    None => {
                        cmd.arg(format!("--config=build.rustflags={rustflags}"));
                        return;
                    }
                }
            }
        },
    };

    for flag in rustflags {
        if !val.is_empty() {
            val.push(separator);
        }
        val.push(flag);
    }

    cmd.env(key, val);
}

#[test]
fn test_make() {
    use std::ffi::{OsStr, OsString};

    let expected_vec = make_vec(&[]);
    let expected_toml = toml::Value::try_from(expected_vec.clone()).unwrap();
    let expected_config_target_rustflags =
        &*OsString::from(format!("--config=target.target.rustflags={expected_toml}"));
    let expected_config_build_rustflags =
        &*OsString::from(format!("--config=build.rustflags={expected_toml}"));
    let sep = CARGO_ENCODED_RUSTFLAGS_SEP;
    let expected_env_encoded_rustflags =
        &*OsString::from(format!("--cfg{sep}a{sep}{}", expected_vec.join(sep)));
    let sep = RUSTFLAGS_SEP;
    let expected_env_rustflags =
        &*OsString::from(format!("--cfg{sep}b{sep}{}", expected_vec.join(sep)));
    let expected_env_build_rustflags =
        &*OsString::from(format!("--cfg{sep}c{sep}{}", expected_vec.join(sep)));

    // without CARGO_ENCODED_RUSTFLAGS/RUSTFLAGS/CARGO_BUILD_RUSTFLAGS
    env::remove_var(CARGO_ENCODED_RUSTFLAGS);
    env::remove_var(RUSTFLAGS);
    env::remove_var(CARGO_BUILD_RUSTFLAGS);
    let mut cmd = Command::new("cargo");
    set(&mut cmd, "target", &[]);
    assert_eq!(cmd.get_envs().len(), 0);
    assert_eq!(
        cmd.get_args().collect::<Vec<_>>(),
        [
            expected_config_target_rustflags,
            expected_config_build_rustflags
        ]
    );

    // with CARGO_BUILD_RUSTFLAGS, without CARGO_ENCODED_RUSTFLAGS/RUSTFLAGS
    let flag = OsStr::new(CARGO_BUILD_RUSTFLAGS);
    let sep = RUSTFLAGS_SEP;
    env::set_var(flag, format!("--cfg{sep}c"));
    let mut cmd = Command::new("cargo");
    set(&mut cmd, "target", &[]);
    assert_eq!(
        cmd.get_envs().collect::<Vec<_>>(),
        [(flag, Some(expected_env_build_rustflags))]
    );
    assert_eq!(
        cmd.get_args().collect::<Vec<_>>(),
        [&*expected_config_target_rustflags]
    );

    // with RUSTFLAGS, without CARGO_ENCODED_RUSTFLAGS
    let flag = OsStr::new(RUSTFLAGS);
    env::set_var(flag, format!("--cfg{sep}b"));
    let mut cmd = Command::new("cargo");
    set(&mut cmd, "target", &[]);
    assert_eq!(
        cmd.get_envs().collect::<Vec<_>>(),
        [(flag, Some(expected_env_rustflags))]
    );
    assert_eq!(cmd.get_args().len(), 0);

    // with CARGO_ENCODED_RUSTFLAGS
    let flag = OsStr::new(CARGO_ENCODED_RUSTFLAGS);
    let sep = CARGO_ENCODED_RUSTFLAGS_SEP;
    env::set_var(flag, format!("--cfg{sep}a"));
    let mut cmd = Command::new("cargo");
    set(&mut cmd, "target", &[]);
    assert_eq!(
        cmd.get_envs().collect::<Vec<_>>(),
        [(flag, Some(expected_env_encoded_rustflags))]
    );
    assert_eq!(cmd.get_args().len(), 0);
}
