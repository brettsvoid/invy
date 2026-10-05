//! User configuration, read from `~/.config/invy/config.toml`.
//!
//! `$XDG_CONFIG_HOME` stands in for `~/.config` when it is set. A missing file
//! is not an error: every setting falls back to its default.

use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Database file. `--db` beats it. A leading `~` is the home directory,
    /// and a relative path is taken from the directory the config file is in.
    pub db: Option<PathBuf>,
}

/// Where the config file is looked for: `~/.config/invy/config.toml`.
pub fn config_path() -> Result<PathBuf> {
    Ok(xdg_dir("XDG_CONFIG_HOME", ".config")?
        .join("invy")
        .join("config.toml"))
}

/// An XDG base directory: `$var` if it is an absolute path, else `~/fallback`.
///
/// Used on every platform, rather than the platform directories, so macOS
/// gets the same path as Linux.
fn xdg_dir(var: &str, fallback: &str) -> Result<PathBuf> {
    match std::env::var_os(var).map(PathBuf::from) {
        Some(dir) if dir.is_absolute() => Ok(dir),
        _ => Ok(home_dir()?.join(fallback)),
    }
}

/// Read the config file, or the defaults if there is none.
pub fn load() -> Result<Config> {
    let path = config_path()?;
    match std::fs::read_to_string(&path) {
        Ok(text) => parse(&text, &path),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(Config::default()),
        Err(e) => Err(e).with_context(|| format!("Failed to read config at {:?}", path)),
    }
}

fn parse(text: &str, path: &Path) -> Result<Config> {
    let mut config: Config =
        toml::from_str(text).with_context(|| format!("Invalid config at {:?}", path))?;

    let dir = path.parent().unwrap_or(Path::new(""));
    config.db = config.db.map(|db| resolve(&db, dir)).transpose()?;

    Ok(config)
}

/// Expand a leading `~` and anchor a relative path to `dir`.
fn resolve(path: &Path, dir: &Path) -> Result<PathBuf> {
    match path.strip_prefix("~") {
        Ok(rest) => Ok(home_dir()?.join(rest)),
        // Joining an absolute path replaces `dir` entirely.
        Err(_) => Ok(dir.join(path)),
    }
}

fn home_dir() -> Result<PathBuf> {
    let base = directories::BaseDirs::new()
        .ok_or_else(|| anyhow!("Could not determine home directory"))?;
    Ok(base.home_dir().to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    const PATH: &str = "/etc/invy/config.toml";

    fn parse_ok(text: &str) -> Config {
        parse(text, Path::new(PATH)).unwrap()
    }

    #[test]
    fn empty_file_sets_nothing() {
        assert_eq!(parse_ok("").db, None);
    }

    #[test]
    fn absolute_db_is_kept() {
        let config = parse_ok(r#"db = "/srv/inventory.db""#);
        assert_eq!(config.db, Some(PathBuf::from("/srv/inventory.db")));
    }

    #[test]
    fn relative_db_is_taken_from_the_config_directory() {
        let config = parse_ok(r#"db = "inventory.db""#);
        assert_eq!(config.db, Some(PathBuf::from("/etc/invy/inventory.db")));
    }

    #[test]
    fn tilde_is_the_home_directory() {
        let config = parse_ok(r#"db = "~/inventory.db""#);
        assert_eq!(config.db, Some(home_dir().unwrap().join("inventory.db")));
    }

    #[test]
    fn tilde_only_expands_as_a_whole_component() {
        let config = parse_ok(r#"db = "~bob/inventory.db""#);
        assert_eq!(
            config.db,
            Some(PathBuf::from("/etc/invy/~bob/inventory.db"))
        );
    }

    #[test]
    fn unknown_keys_are_rejected() {
        let err = parse(r#"database = "x.db""#, Path::new(PATH)).unwrap_err();
        let message = format!("{:#}", err);
        assert!(message.contains(PATH), "{message}");
        assert!(message.contains("unknown field"), "{message}");
    }

    #[test]
    fn malformed_toml_names_the_file() {
        let err = parse("db = ", Path::new(PATH)).unwrap_err();
        assert!(format!("{:#}", err).contains(PATH));
    }
}
