//! Save files: versioned JSON with migrations, written atomically.
//!
//! Schema history:
//!   1 - the Python version of Bytepet (`{"version": 1, ...}`, pet field `char`).
//!   2 - this version: `{"schema": 2, "saved_at": ..., "game": {...}}`.
//!
//! Loading always runs every migration from the file's schema up to `SCHEMA`, so old saves
//! keep working when the format changes. Add a new `migrate_vN` for every format change.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

use crate::sim::Game;

pub const SCHEMA: u64 = 2;

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

/// Per-user data folder (the same one the Python version used).
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
    let schema = schema_of(&v);
    if schema > SCHEMA {
        return Err(LoadError::TooNew(schema));
    }
    if schema < 2 {
        v = migrate_v1(v);
    }
    // future: if schema < 3 { v = migrate_v2(v); } ...
    serde_json::from_value(v).map_err(|e| LoadError::Corrupt(e.to_string()))
}

fn schema_of(v: &Value) -> u64 {
    if let Some(s) = v.get("schema").and_then(|s| s.as_u64()) {
        return s;
    }
    if v.get("version").is_some() || v.get("pet").is_some() {
        return 1;
    }
    0
}

/// Python Bytepet (schema 1) -> schema 2.
fn migrate_v1(v: Value) -> Value {
    let mut old = match v {
        Value::Object(m) => m,
        _ => Map::new(),
    };
    let saved_at = old.remove("saved_at").and_then(|s| s.as_f64()).unwrap_or(0.0);
    let mut game = Map::new();
    for key in ["device_id", "gp", "items", "friends"] {
        if let Some(x) = old.remove(key) {
            game.insert(key.into(), x);
        }
    }
    // pet: `char` became `character`; calls lost their `missed` flag
    if let Some(Value::Object(mut pet)) = old.remove("pet") {
        if let Some(ch) = pet.remove("char") {
            pet.insert("character".into(), ch);
        }
        if let Some(Value::Object(call)) = pet.get_mut("call") {
            call.remove("missed");
        }
        for (k, val) in pet.iter_mut() {
            // Python wrote floats for some integer fields; make them whole numbers
            if [
                "hunger",
                "happy",
                "discipline",
                "poop",
                "doses",
                "weight",
                "gen",
                "mistakes",
                "baby_mistakes",
                "last_school_day",
            ]
            .contains(&k.as_str())
            {
                if let Some(f) = val.as_f64() {
                    *val = json!(f.round() as i64);
                }
            }
        }
        if let Some(Value::Object(skills)) = pet.get_mut("skills") {
            for (_, s) in skills.iter_mut() {
                if let Some(f) = s.as_f64() {
                    *s = json!(f.max(0.0).round() as u64);
                }
            }
        }
        pet.insert("id".into(), json!(1));
        game.insert("pet".into(), Value::Object(pet));
    }
    if let Some(Value::Array(album)) = old.remove("album") {
        let fixed: Vec<Value> = album
            .into_iter()
            .map(|e| match e {
                Value::Object(mut m) => {
                    if let Some(ch) = m.remove("char") {
                        m.insert("character".into(), ch);
                    }
                    Value::Object(m)
                }
                other => other,
            })
            .collect();
        game.insert("album".into(), Value::Array(fixed));
    }
    if let Some(Value::Object(mut settings)) = old.remove("settings") {
        settings.remove("clock_offset"); // the clock no longer drifts, so there is nothing to keep
        game.insert("settings".into(), Value::Object(settings));
    }
    game.insert("next_pet_id".into(), json!(2));
    json!({ "schema": 2, "saved_at": saved_at, "game": Value::Object(game) })
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
    use crate::content::{Content, Stage};
    use crate::rng::Pcg32;

    const PYTHON_SAVE: &str = r#"{
 "version": 1, "device_id": "a1b2c3d4e5f6",
 "pet": {"name": "KIWI", "gen": 2, "stage": "teen", "char": "teen_g", "t": 200000.5, "age_s": 199000.0,
         "stage_s": 50000.0, "hunger": 3, "happy": 2, "weight": 17, "discipline": 50, "poop": 1,
         "sick": false, "doses": 0, "asleep": false, "lights": true, "mistakes": 3, "baby_mistakes": 1,
         "skills": {"humor": 12, "gorgeous": 88, "passion": 40}, "job": null, "last_work": -1e12,
         "married": false, "partner": null, "mystic": false,
         "call": {"reason": "need", "t": 199990.0, "missed": false}, "need_called": true, "recall_t": 0.0,
         "scold_t": 0.0, "praise_t": 0.0, "t_hunger": 100.0, "t_happy": 200.0, "t_poop": 300.0,
         "t_mail": 400.0, "t_fake": 500.0, "t_show": 600.0, "zero_s": 0.0, "poop_s": 10.0,
         "sick_s": 0.0, "fat_s": 0.0, "mail": [{"kind": "gp", "t": 1.0, "gp": 150}],
         "last_school_day": 1, "gone_reason": null},
 "gp": 345, "items": {"CAKE": 2}, "album": [{"name": "MOCHI", "char": "h_a", "gen": 1, "age": 6,
 "job": "OFFICE", "partner": "PRIMPY", "ending": "MARRIED"}],
 "friends": {"ffee": {"name": "BOB", "count": 4}},
 "settings": {"speed": 60, "muted": true, "shell": 3, "run_away": true, "clock_offset": 1234.0,
              "ui": {"scale": 0.75, "x": 100, "y": 200, "on_top": false, "notify": true}},
 "saved_at": 1790000000.0
}"#;

    #[test]
    fn python_save_migrates() {
        let s = from_json(PYTHON_SAVE).unwrap();
        let g = &s.game;
        assert_eq!(s.saved_at, 1790000000.0);
        assert_eq!(g.pet.name, "KIWI");
        assert_eq!(g.pet.stage, Stage::Teen);
        assert_eq!(g.pet.character, "teen_g");
        assert_eq!(g.pet.skills.gorgeous, 88);
        assert_eq!(g.pet.mail[0].gp, Some(150));
        assert_eq!(g.gp, 345);
        assert_eq!(g.items["CAKE"], 2);
        assert_eq!(g.album[0].character, "h_a");
        assert_eq!(g.friends["ffee"].count, 4);
        assert_eq!(g.settings.ui.scale, 0.75);
        assert!(g.settings.muted);
        assert_eq!(g.pet.id, 1);
    }

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
}
