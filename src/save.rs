//! Save files: versioned JSON with migrations, written atomically and flushed to disk, with
//! the previous save kept as `save.json.bak` in case the newest one is ever lost.
//!
//! Schema history:
//!   2 - `{"schema": 2, "saved_at": ..., "game": {...}}`, the first released format.
//!   3 - `settings.look` holds every part of the device's look; `settings.shell` is now just
//!       the preset last picked. Older saves get the look of the shell they had.
//!
//! When the format changes, bump `SCHEMA` and add a `migrate_vN` that `from_json` runs on
//! older files, so existing saves keep working.

use std::ffi::OsString;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::sim::Game;

pub const SCHEMA: u64 = 3;

#[derive(Serialize, Deserialize)]
pub struct SaveFile {
    pub schema: u64,
    /// Unix time of the save, used to catch up on time spent closed.
    pub saved_at: f64,
    pub game: Game,
}

#[derive(Debug)]
pub enum LoadError {
    Missing,
    Corrupt(String),
    TooNew(u64),
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadError::Missing => write!(f, "no save file"),
            LoadError::Corrupt(e) => write!(f, "save file is damaged: {e}"),
            LoadError::TooNew(v) => write!(f, "save file is from a newer Bytepet (schema {v})"),
        }
    }
}

/// Per-user data folder.
pub fn data_dir() -> PathBuf {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    if cfg!(target_os = "windows") {
        std::env::var_os("APPDATA").map(PathBuf::from).unwrap_or(home).join("Bytepet")
    } else if cfg!(target_os = "macos") {
        home.join("Library").join("Application Support").join("Bytepet")
    } else {
        std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".local").join("share"))
            .join("bytepet")
    }
}

pub fn default_save_path() -> PathBuf {
    data_dir().join("save.json")
}

/// Parse save JSON of any known schema into the current format.
pub fn from_json(text: &str) -> Result<SaveFile, LoadError> {
    let mut v: Value = serde_json::from_str(text).map_err(|e| LoadError::Corrupt(e.to_string()))?;
    let schema = v.get("schema").and_then(|s| s.as_u64()).unwrap_or(0);
    if schema > SCHEMA {
        return Err(LoadError::TooNew(schema));
    }
    if schema < 3 {
        migrate_v2(&mut v);
    }
    serde_json::from_value(v).map_err(|e| LoadError::Corrupt(e.to_string()))
}

/// Schema 2 kept only a shell number; give the save that shell's look.
fn migrate_v2(v: &mut Value) {
    let Some(settings) = v.pointer_mut("/game/settings").and_then(Value::as_object_mut) else { return };
    if settings.contains_key("look") {
        return;
    }
    let shell = settings.get("shell").and_then(Value::as_u64).unwrap_or(0);
    let look = crate::look::PRESETS[shell as usize % crate::look::PRESETS.len()].look;
    if let Ok(look) = serde_json::to_value(look) {
        settings.insert("look".into(), look);
    }
}

/// A file next to the save, named after it: `save.json` + `.bak` is `save.json.bak`.
fn sibling(path: &Path, suffix: &str) -> PathBuf {
    let mut name: OsString = path.file_name().map(OsString::from).unwrap_or_else(|| "save.json".into());
    name.push(suffix);
    path.with_file_name(name)
}

/// The previous save, kept by [`save`].
pub fn backup_path(path: &Path) -> PathBuf {
    sibling(path, ".bak")
}

/// A save as loaded, and why the backup had to be used instead, if it was.
pub struct Loaded {
    pub file: SaveFile,
    pub restored: Option<LoadError>,
}

fn load_file(path: &Path) -> Result<SaveFile, LoadError> {
    match std::fs::read_to_string(path) {
        Ok(t) => from_json(&t),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(LoadError::Missing),
        Err(e) => Err(LoadError::Corrupt(e.to_string())),
    }
}

/// Load the save, falling back to the backup when it is damaged or missing (a power cut can
/// leave it empty).
pub fn load(path: &Path) -> Result<SaveFile, LoadError> {
    load_any(path).map(|l| l.file)
}

/// [`load`], also telling whether the backup was used.
pub fn load_any(path: &Path) -> Result<Loaded, LoadError> {
    let e = match load_file(path) {
        Ok(file) => return Ok(Loaded { file, restored: None }),
        // never fall back past a save made by a newer version
        Err(e @ LoadError::TooNew(_)) => return Err(e),
        Err(e) => e,
    };
    if let LoadError::Corrupt(_) = &e {
        // keep the damaged file for inspection instead of silently overwriting it
        let _ = std::fs::rename(path, sibling(path, ".corrupt"));
    }
    match load_file(&backup_path(path)) {
        Ok(file) => Ok(Loaded { file, restored: Some(e) }),
        Err(b @ LoadError::TooNew(_)) => Err(b),
        Err(_) => Err(e),
    }
}

pub fn to_json(game: &Game, saved_at: f64) -> String {
    let file = json!({ "schema": SCHEMA, "saved_at": saved_at, "game": game });
    serde_json::to_string_pretty(&file).unwrap_or_default()
}

/// Write via a temporary file that is flushed to disk before it replaces the save, so neither
/// a crash nor a power cut mid-save can destroy it. The save it replaces becomes the backup.
pub fn save(path: &Path, game: &Game, saved_at: f64) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = sibling(path, ".tmp");
    {
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(to_json(game, saved_at).as_bytes())?;
        f.sync_all()?;
    }
    // a damaged save is no backup: keep the one already there instead
    if load_file(path).is_ok() {
        std::fs::rename(path, backup_path(path))?;
    }
    std::fs::rename(&tmp, path)?;
    sync_dir(path);
    Ok(())
}

/// Make the renames themselves durable. Windows has no directory handle to flush; NTFS
/// journals renames.
fn sync_dir(path: &Path) {
    #[cfg(unix)]
    if let Some(dir) = path.parent() {
        let dir = if dir.as_os_str().is_empty() { Path::new(".") } else { dir };
        if let Ok(d) = std::fs::File::open(dir) {
            let _ = d.sync_all();
        }
    }
    #[cfg(not(unix))]
    let _ = path;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::Content;
    use crate::rng::Pcg32;

    #[test]
    fn roundtrip_current_schema() {
        let c = Content::builtin();
        let mut g = Game::new(&c, Pcg32::new(3));
        g.pet.name = "BEAN".into();
        let text = to_json(&g, 42.0);
        let s = from_json(&text).unwrap();
        assert_eq!(s.schema, SCHEMA);
        assert_eq!(s.game.pet.name, "BEAN");
        assert_eq!(s.game.rng, g.rng);
    }

    #[test]
    fn newer_schema_is_refused_not_clobbered() {
        assert!(matches!(from_json(r#"{"schema": 99, "game": {}}"#), Err(LoadError::TooNew(99))));
        assert!(matches!(from_json("not json"), Err(LoadError::Corrupt(_))));
    }

    #[test]
    fn missing_fields_get_defaults() {
        let s = from_json(r#"{"schema": 2, "saved_at": 0, "game": {"pet": {"name": "X"}}}"#).unwrap();
        assert_eq!(s.game.pet.name, "X");
        assert_eq!(s.game.settings.speed, 1);
    }

    #[test]
    fn a_schema_2_save_keeps_its_shell() {
        use crate::look::{Pattern, PRESETS};
        let s = from_json(r#"{"schema": 2, "saved_at": 0, "game": {"settings": {"shell": 5}}}"#).unwrap();
        assert_eq!(s.game.settings.shell, 5);
        assert_eq!(s.game.settings.look, PRESETS[5].look);
        assert_eq!(s.game.settings.look.pattern, Pattern::Flame);
        assert_eq!(s.game.settings.shell_name(), "FLAME");
        // and a customised look survives a round trip
        let c = Content::builtin();
        let mut g = Game::new(&c, Pcg32::new(3));
        g.settings.look.step(crate::look::Field::Antenna, 2);
        let back = from_json(&to_json(&g, 1.0)).unwrap();
        assert_eq!(back.game.settings.look, g.settings.look);
        assert_eq!(back.game.settings.shell_name(), "CUSTOM");
    }

    fn temp_dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("bytepet-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn a_lost_save_falls_back_to_the_backup() {
        let dir = temp_dir("backup");
        let path = dir.join("save.json");
        let c = Content::builtin();
        let mut g = Game::new(&c, Pcg32::new(3));
        g.pet.name = "FIRST".into();
        save(&path, &g, 1.0).unwrap();
        assert!(!backup_path(&path).exists(), "nothing to back up yet");
        g.pet.name = "SECOND".into();
        save(&path, &g, 2.0).unwrap();
        assert!(!sibling(&path, ".tmp").exists());
        assert_eq!(load(&path).unwrap().game.pet.name, "SECOND");
        // a power cut left the newest save empty: the family survives in the backup
        std::fs::write(&path, "").unwrap();
        let l = load_any(&path).unwrap();
        assert_eq!(l.file.game.pet.name, "FIRST");
        assert!(matches!(l.restored, Some(LoadError::Corrupt(_))));
        assert!(sibling(&path, ".corrupt").exists());
        // saving over a damaged file doesn't make it the backup
        std::fs::write(&path, "").unwrap();
        save(&path, &l.file.game, 3.0).unwrap();
        assert_eq!(load_file(&backup_path(&path)).unwrap().game.pet.name, "FIRST");
        // a save gone missing between the two renames comes back from the backup too
        std::fs::remove_file(&path).unwrap();
        assert!(matches!(load_any(&path).unwrap().restored, Some(LoadError::Missing)));
        // with neither file, it's a new game
        std::fs::remove_file(backup_path(&path)).unwrap();
        assert!(matches!(load(&path), Err(LoadError::Missing)));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
