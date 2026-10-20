//! Save files: versioned JSON with migrations, written atomically.
//!
//! Schema history:
//!   2 - `{"schema": 2, "saved_at": ..., "game": {...}}`, the first released format.
//!   3 - `settings.look` holds every part of the device's look; `settings.shell` is now just
//!       the preset last picked. Older saves get the look of the shell they had.
//!
//! When the format changes, bump `SCHEMA` and add a `migrate_vN` that `from_json` runs on
//! older files, so existing saves keep working.

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

pub fn load(path: &Path) -> Result<SaveFile, LoadError> {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Err(LoadError::Missing),
        Err(e) => return Err(LoadError::Corrupt(e.to_string())),
    };
    let r = from_json(&text);
    if let Err(LoadError::Corrupt(_)) = &r {
        // keep the damaged file for inspection instead of silently overwriting it
        let _ = std::fs::rename(path, path.with_extension("json.corrupt"));
    }
    r
}

pub fn to_json(game: &Game, saved_at: f64) -> String {
    let file = json!({ "schema": SCHEMA, "saved_at": saved_at, "game": game });
    serde_json::to_string_pretty(&file).unwrap_or_default()
}

/// Write via a temporary file and rename, so a crash mid-save can't destroy the old save.
pub fn save(path: &Path, game: &Game, saved_at: f64) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, to_json(game, saved_at))?;
    std::fs::rename(&tmp, path)
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
}
