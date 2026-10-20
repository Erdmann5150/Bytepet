//! Game content loaded from TOML: characters (with sprites), props, items, jobs and rules.
//!
//! The files in `content/` are compiled into the binary. Players can override any of them by
//! putting a file with the same name in `<save folder>/content/`.

use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::sprite::Sprite;

pub const CHARACTERS_FILE: &str = "characters.toml";
pub const PROPS_FILE: &str = "props.toml";
pub const GAME_FILE: &str = "game.toml";

const CHARACTERS_TOML: &str = include_str!("../content/characters.toml");
const PROPS_TOML: &str = include_str!("../content/props.toml");
const GAME_TOML: &str = include_str!("../content/game.toml");

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum Skill {
    Humor,
    Gorgeous,
    Passion,
}

impl Skill {
    pub const ALL: [Skill; 3] = [Skill::Humor, Skill::Gorgeous, Skill::Passion];
    pub fn label(self) -> &'static str {
        match self {
            Skill::Humor => "HUMOR",
            Skill::Gorgeous => "GORGEOUS",
            Skill::Passion => "PASSION",
        }
    }
    pub fn title(self) -> &'static str {
        match self {
            Skill::Humor => "Humor",
            Skill::Gorgeous => "Gorgeous",
            Skill::Passion => "Passion",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Stage {
    #[default]
    Egg,
    Baby,
    Child,
    Teen,
    Adult,
    Senior,
    Gone,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Pose {
    Idle1,
    Idle2,
    Happy,
    Sad,
    Eat,
    Sleep,
}

const POSE_KEYS: [(&str, Pose); 6] = [
    ("idle1", Pose::Idle1),
    ("idle2", Pose::Idle2),
    ("happy", Pose::Happy),
    ("sad", Pose::Sad),
    ("eat", Pose::Eat),
    ("sleep", Pose::Sleep),
];

pub struct Character {
    pub id: String,
    pub name: String,
    pub stage: Stage,
    pub path: Option<Skill>,
    pub grade: Option<String>,
    frames: Vec<Sprite>, // indexed in POSE_KEYS order
}

impl Character {
    pub fn frame(&self, pose: Pose) -> &Sprite {
        let i = POSE_KEYS.iter().position(|(_, p)| *p == pose).unwrap_or(0);
        &self.frames[i]
    }
}

// ------------------------------------------------------------------ raw TOML shapes
#[derive(Deserialize)]
struct CharactersFile {
    character: Vec<RawCharacter>,
}

#[derive(Deserialize)]
struct RawCharacter {
    id: String,
    name: String,
    stage: Stage,
    path: Option<Skill>,
    grade: Option<String>,
    #[serde(flatten)]
    frames: BTreeMap<String, toml::Value>,
}

#[derive(Deserialize)]
struct PropsFile {
    sprite: BTreeMap<String, String>,
    icons: BTreeMap<String, toml::Value>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Life {
    pub egg: f64,
    pub baby: f64,
    pub child: f64,
    pub teen: f64,
    pub senior_age_days: u32,
    pub matchmaker_age_days: u32,
    pub start_gp: i64,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Care {
    pub call_timeout: f64,
    pub recall_baby: f64,
    pub recall: f64,
    pub empty_to_sick: f64,
    pub sick_to_gone_baby: f64,
    pub sick_to_gone: f64,
    pub poop_to_sick_baby: f64,
    pub poop_to_sick: f64,
    pub poop_to_sick_full: f64,
    pub overweight_margin: i32,
    pub overweight_to_sick: f64,
    pub fake_call_min: f64,
    pub fake_call_max: f64,
    pub show_off_min: f64,
    pub show_off_max: f64,
    pub show_off_window: f64,
    pub refuse_window: f64,
    pub refuse_chance: f64,
    pub mail_min: f64,
    pub mail_max: f64,
    pub mail_max_waiting: usize,
    pub school_expires: f64,
    pub work_cooldown: f64,
    pub legacy_skill_share: f64,
}

#[derive(Deserialize, Clone, Debug)]
pub struct StageRule {
    pub hunger: f64,
    pub happy: f64,
    pub poop: [f64; 2],
    pub weight: i32,
    pub sleep: Option<[f64; 2]>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Evolution {
    pub secret_skill: u32,
    pub best_mistakes: u32,
    pub best_discipline: u8,
    pub good_mistakes: u32,
    pub fair_mistakes: u32,
    pub baby_good_mistakes: u32,
}

#[derive(Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct Effects {
    pub happy: u8,
    pub hunger: u8,
    pub weight: i32,
    pub humor: u32,
    pub gorgeous: u32,
    pub passion: u32,
    pub cure: u8,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Item {
    pub id: String,
    pub price: i64,
    #[serde(default)]
    pub food: bool,
    pub effects: Effects,
}

#[derive(Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum JobGame {
    Office,
    Science,
    Chef,
    Fire,
    Rock,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Job {
    pub id: String,
    pub title: String,
    pub skill: Skill,
    pub need: u32,
    pub pay: i64,
    pub game: JobGame,
}

#[derive(Deserialize, Clone, Debug)]
pub struct MailRules {
    pub gp: u32,
    pub heart: u32,
    pub trick: u32,
    pub item: u32,
    pub school: u32,
    pub job: u32,
    pub love: u32,
    pub gp_amounts: Vec<i64>,
    pub items: Vec<String>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct FriendLevel {
    pub links: u32,
    pub name: String,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Rules {
    pub life: Life,
    pub care: Care,
    pub stage: HashMap<String, StageRule>,
    pub evolution: Evolution,
    pub item: Vec<Item>,
    pub job: Vec<Job>,
    pub mail: MailRules,
    pub friend_level: Vec<FriendLevel>,
}

// ------------------------------------------------------------------ loaded content
pub struct Content {
    pub characters: Vec<Character>,
    index: HashMap<String, usize>,
    pub props: HashMap<String, Sprite>,
    pub icons: Vec<Sprite>,
    pub rules: Rules,
    /// Files that were loaded from the mod folder.
    pub overrides: Vec<String>,
}

impl Content {
    /// The built-in content.
    pub fn builtin() -> Content {
        Content::parse(CHARACTERS_TOML, PROPS_TOML, GAME_TOML).expect("built-in content is valid")
    }

    /// Built-in content, with any files from `mod_dir` replacing their built-in versions.
    pub fn load(mod_dir: Option<&Path>) -> Result<Content, String> {
        let mut overrides = Vec::new();
        let mut read = |name: &str, builtin: &'static str| -> Result<String, String> {
            if let Some(dir) = mod_dir {
                let p = dir.join(name);
                if p.is_file() {
                    overrides.push(name.to_string());
                    return std::fs::read_to_string(&p).map_err(|e| format!("{}: {}", p.display(), e));
                }
            }
            Ok(builtin.to_string())
        };
        let c = read(CHARACTERS_FILE, CHARACTERS_TOML)?;
        let p = read(PROPS_FILE, PROPS_TOML)?;
        let g = read(GAME_FILE, GAME_TOML)?;
        let mut content = Content::parse(&c, &p, &g)?;
        content.overrides = overrides;
        Ok(content)
    }

    pub fn parse(characters: &str, props: &str, game: &str) -> Result<Content, String> {
        let cf: CharactersFile = toml::from_str(characters).map_err(|e| format!("{CHARACTERS_FILE}: {e}"))?;
        let pf: PropsFile = toml::from_str(props).map_err(|e| format!("{PROPS_FILE}: {e}"))?;
        let rules: Rules = toml::from_str(game).map_err(|e| format!("{GAME_FILE}: {e}"))?;

        let mut characters = Vec::new();
        let mut index = HashMap::new();
        for rc in cf.character {
            let mut frames = Vec::new();
            for (key, _) in POSE_KEYS {
                let rows = resolve_frame(&rc.frames, key, 0).map_err(|e| format!("character {}: {e}", rc.id))?;
                frames.push(Sprite::from_rows(&rows));
            }
            if index.insert(rc.id.clone(), characters.len()).is_some() {
                return Err(format!("character id {} is used twice", rc.id));
            }
            characters.push(Character {
                id: rc.id,
                name: rc.name.to_uppercase(),
                stage: rc.stage,
                path: rc.path,
                grade: rc.grade,
                frames,
            });
        }

        let mut props = HashMap::new();
        for (k, v) in &pf.sprite {
            props.insert(k.clone(), Sprite::parse(v));
        }
        let order: Vec<String> = pf
            .icons
            .get("order")
            .and_then(|v| v.as_array())
            .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
            .unwrap_or_default();
        if order.len() != 10 {
            return Err(format!("{PROPS_FILE}: [icons] order must list 10 icons"));
        }
        let mut icons = Vec::new();
        for name in &order {
            let art =
                pf.icons.get(name).and_then(|v| v.as_str()).ok_or(format!("{PROPS_FILE}: icon {name} missing"))?;
            icons.push(Sprite::parse(art));
        }

        let c = Content { characters, index, props, icons, rules, overrides: Vec::new() };
        c.validate()?;
        Ok(c)
    }

    fn validate(&self) -> Result<(), String> {
        let need_props = [
            "egg",
            "egg_crack",
            "meal",
            "snack",
            "gift",
            "poop1",
            "poop2",
            "skull",
            "heart",
            "heart_empty",
            "heart_small",
            "apple",
            "bomb",
            "rock",
            "basket",
            "climber1",
            "climber2",
            "walker1",
            "walker2",
            "faller",
            "doc",
            "note",
            "flask",
            "flask_empty",
            "bun",
            "meat",
            "leaf",
            "arrow_l",
            "arrow_r",
            "arrow_u",
            "star",
            "syringe",
            "ufo",
            "mini_pet",
            "mini_rival",
            "window",
            "hose",
            "check",
            "cross",
            "ring",
            "zzz",
        ];
        for p in need_props {
            if !self.props.contains_key(p) {
                return Err(format!("{PROPS_FILE}: sprite '{p}' is missing"));
            }
        }
        let mut need: Vec<(Stage, Option<Skill>, Option<&str>)> = vec![
            (Stage::Baby, None, None),
            (Stage::Child, None, Some("a")),
            (Stage::Child, None, Some("b")),
            (Stage::Senior, None, None),
            (Stage::Adult, None, Some("rare")),
        ];
        for s in Skill::ALL {
            need.push((Stage::Teen, Some(s), None));
            for g in ["a", "b", "c", "d", "secret"] {
                need.push((Stage::Adult, Some(s), Some(g)));
            }
        }
        for (st, path, grade) in need {
            if self.find(st, path, grade).is_none() {
                return Err(format!(
                    "{CHARACTERS_FILE}: no character for stage {:?}, path {:?}, grade {:?}",
                    st, path, grade
                ));
            }
        }
        for st in ["baby", "child", "teen", "adult", "senior"] {
            if !self.rules.stage.contains_key(st) {
                return Err(format!("{GAME_FILE}: [stage.{st}] is missing"));
            }
        }
        for j in &self.rules.job {
            if j.id.chars().count() > 8 {
                return Err(format!("{GAME_FILE}: job id {} is longer than 8 letters", j.id));
            }
        }
        for it in &self.rules.item {
            if it.id.chars().count() > 8 {
                return Err(format!("{GAME_FILE}: item id {} is longer than 8 letters", it.id));
            }
        }
        for m in &self.rules.mail.items {
            if self.item(m).is_none() {
                return Err(format!("{GAME_FILE}: mail item {m} is not an item"));
            }
        }
        Ok(())
    }

    // ------------------------------------------------------------------ lookups
    pub fn character(&self, id: &str) -> Option<&Character> {
        self.index.get(id).map(|&i| &self.characters[i])
    }

    /// The character's sprite for a pose; the egg (or unknown ids) fall back to the egg sprite.
    pub fn sprite(&self, id: &str, pose: Pose) -> &Sprite {
        match self.character(id) {
            Some(c) => c.frame(pose),
            None => &self.props["egg"],
        }
    }

    pub fn name_of(&self, id: &str) -> String {
        self.character(id).map(|c| c.name.clone()).unwrap_or_else(|| "EGG".into())
    }

    pub fn find(&self, stage: Stage, path: Option<Skill>, grade: Option<&str>) -> Option<&str> {
        self.characters
            .iter()
            .find(|c| {
                c.stage == stage
                    && (path.is_none() || c.path == path)
                    && (grade.is_none() || c.grade.as_deref() == grade)
            })
            .map(|c| c.id.as_str())
    }

    pub fn prop(&self, name: &str) -> &Sprite {
        self.props.get(name).unwrap_or(&self.props["egg"])
    }

    pub fn item(&self, id: &str) -> Option<&Item> {
        self.rules.item.iter().find(|i| i.id == id)
    }

    pub fn job(&self, id: &str) -> Option<&Job> {
        self.rules.job.iter().find(|j| j.id == id)
    }

    pub fn stage_rule(&self, stage: Stage) -> &StageRule {
        let key = match stage {
            Stage::Egg | Stage::Baby => "baby",
            Stage::Child => "child",
            Stage::Teen => "teen",
            Stage::Adult | Stage::Gone => "adult",
            Stage::Senior => "senior",
        };
        &self.rules.stage[key]
    }

    pub fn friend_level(&self, links: u32) -> &str {
        self.rules
            .friend_level
            .iter()
            .filter(|l| links >= l.links)
            .max_by_key(|l| l.links)
            .map(|l| l.name.as_str())
            .unwrap_or("STRANGER")
    }

    pub fn soulmate_links(&self) -> u32 {
        self.rules.friend_level.iter().map(|l| l.links).max().unwrap_or(10)
    }

    pub fn adults(&self) -> Vec<&Character> {
        self.characters
            .iter()
            .filter(|c| c.stage == Stage::Adult && matches!(c.grade.as_deref(), Some("a" | "b" | "c" | "d")))
            .collect()
    }
}

/// Resolve a frame (a full drawing or a patch of another frame) into 16 rows of 16 chars.
fn resolve_frame(frames: &BTreeMap<String, toml::Value>, key: &str, depth: u32) -> Result<Vec<String>, String> {
    if depth > 6 {
        return Err(format!("frame {key}: patches refer to each other in a loop"));
    }
    let v = frames.get(key).ok_or(format!("frame '{key}' is missing"))?;
    let rows: Vec<String> = match v {
        toml::Value::String(art) => art.lines().map(|l| l.trim_end().to_string()).filter(|l| !l.is_empty()).collect(),
        toml::Value::Table(t) => {
            let base = t.get("base").and_then(|b| b.as_str()).ok_or(format!("frame {key}: patch needs base"))?;
            let mut rows = resolve_frame(frames, base, depth + 1)?;
            let sx = t.get("shift_x").and_then(|v| v.as_integer()).unwrap_or(0) as i32;
            let sy = t.get("shift_y").and_then(|v| v.as_integer()).unwrap_or(0) as i32;
            if sx != 0 || sy != 0 {
                let mut out = vec![vec!['.'; 16]; 16];
                for (y, r) in rows.iter().enumerate() {
                    for (x, c) in r.chars().enumerate() {
                        let (nx, ny) = (x as i32 + sx, y as i32 + sy);
                        if c == '#' && (0..16).contains(&nx) && (0..16).contains(&ny) {
                            out[ny as usize][nx as usize] = '#';
                        }
                    }
                }
                rows = out.into_iter().map(|r| r.into_iter().collect()).collect();
            }
            if let Some(toml::Value::Table(patch)) = t.get("rows") {
                for (k, v) in patch {
                    let i: usize = k.parse().map_err(|_| format!("frame {key}: row '{k}' is not a number"))?;
                    let s = v.as_str().ok_or(format!("frame {key}: row {k} must be text"))?;
                    if i >= 16 {
                        return Err(format!("frame {key}: row {i} is outside 0-15"));
                    }
                    rows[i] = s.to_string();
                }
            }
            rows
        }
        _ => return Err(format!("frame {key} must be a drawing or a patch")),
    };
    if rows.len() != 16 {
        return Err(format!("frame {key} has {} rows, expected 16", rows.len()));
    }
    for (i, r) in rows.iter().enumerate() {
        if r.chars().count() != 16 {
            return Err(format!("frame {key} row {i} is {} wide, expected 16", r.chars().count()));
        }
    }
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_content_is_valid() {
        let c = Content::builtin();
        assert_eq!(c.characters.len(), 23);
        assert_eq!(c.icons.len(), 10);
        for ch in &c.characters {
            for (_, pose) in POSE_KEYS {
                let s = ch.frame(pose);
                assert_eq!((s.w, s.h), (16, 16), "{} {:?}", ch.id, pose);
                assert!(s.pix.iter().any(|&p| p), "{} {:?} is blank", ch.id, pose);
            }
        }
        // poses really differ from the idle drawing
        for ch in &c.characters {
            assert_ne!(ch.frame(Pose::Idle1), ch.frame(Pose::Happy), "{} happy == idle", ch.id);
            assert_ne!(ch.frame(Pose::Idle1), ch.frame(Pose::Eat), "{} eat == idle", ch.id);
        }
    }

    #[test]
    fn bad_mod_is_reported() {
        let broken = CHARACTERS_TOML
            .replace("id = \"senior\"", "id = \"elder\"")
            .replace("stage = \"senior\"", "stage = \"adult\"");
        let e = Content::parse(&broken, PROPS_TOML, GAME_TOML).err().unwrap();
        assert!(e.contains("Senior"), "{e}");
    }
}
