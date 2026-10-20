//! How the device looks: body, buttons, antenna and screen, each chosen on its own, plus the
//! named presets the SHELL setting steps through.
//!
//! Every customisable part is a [`Field`] with a fixed list of options, so the settings menu,
//! the tray menu and the save file all treat them the same way.

use serde::{Deserialize, Serialize};

pub type Rgb = [u8; 3];

/// The colours the menus offer, with names short enough for the settings panel.
pub const PALETTE: [(&str, Rgb); 20] = [
    ("PINK", [236, 72, 153]),
    ("RED", [220, 38, 38]),
    ("ORANGE", [249, 115, 22]),
    ("YELLOW", [255, 214, 64]),
    ("GOLD", [212, 175, 55]),
    ("LIME", [132, 204, 22]),
    ("MINT", [110, 231, 183]),
    ("OLIVE", [112, 132, 84]),
    ("TEAL", [20, 184, 166]),
    ("BLUE", [56, 152, 236]),
    ("SKY", [125, 211, 252]),
    ("NAVY", [30, 58, 138]),
    ("PURPLE", [147, 92, 226]),
    ("ROSE", [246, 170, 200]),
    ("BROWN", [146, 96, 60]),
    ("CREAM", [244, 242, 236]),
    ("WHITE", [238, 238, 232]),
    ("SILVER", [190, 190, 200]),
    ("GRAY", [96, 100, 96]),
    ("BLACK", [25, 25, 25]),
];

const fn pal(i: usize) -> Rgb {
    PALETTE[i].1
}
const PINK: Rgb = pal(0);
const RED: Rgb = pal(1);
const ORANGE: Rgb = pal(2);
const YELLOW: Rgb = pal(3);
const GOLD: Rgb = pal(4);
const LIME: Rgb = pal(5);
const MINT: Rgb = pal(6);
const OLIVE: Rgb = pal(7);
const TEAL: Rgb = pal(8);
const BLUE: Rgb = pal(9);
const NAVY: Rgb = pal(11);
const PURPLE: Rgb = pal(12);
const ROSE: Rgb = pal(13);
const CREAM: Rgb = pal(15);
const WHITE: Rgb = pal(16);
const SILVER: Rgb = pal(17);
const GRAY: Rgb = pal(18);
const BLACK: Rgb = pal(19);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Pattern {
    Plain,
    Zebra,
    Flame,
    Sprinkles,
    Camo,
    Dots,
    Stripes,
    Hearts,
}

impl Pattern {
    pub const ALL: [Pattern; 8] = [
        Pattern::Plain,
        Pattern::Zebra,
        Pattern::Flame,
        Pattern::Sprinkles,
        Pattern::Camo,
        Pattern::Dots,
        Pattern::Stripes,
        Pattern::Hearts,
    ];
    pub fn name(self) -> &'static str {
        ["PLAIN", "ZEBRA", "FLAME", "CANDY", "CAMO", "DOTS", "STRIPE", "HEARTS"][self as usize]
    }
    /// The pattern's own colour, used when the accent is AUTO. Sprinkles and camo are
    /// several colours each, so for them this is only a stand-in.
    pub fn default_accent(self) -> Rgb {
        match self {
            Pattern::Zebra => BLACK,
            Pattern::Flame => YELLOW,
            _ => [255, 255, 255],
        }
    }
    /// Seeds the pattern's randomness. These match the shells' old positions in the list, so
    /// the original eight designs look the same as before.
    pub fn seed(self) -> u64 {
        self as u64 + 14
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ButtonShape {
    Round,
    Square,
    Gem,
    Heart,
    Star,
}

impl ButtonShape {
    pub const ALL: [ButtonShape; 5] =
        [ButtonShape::Round, ButtonShape::Square, ButtonShape::Gem, ButtonShape::Heart, ButtonShape::Star];
    pub fn name(self) -> &'static str {
        ["ROUND", "SQUARE", "GEM", "HEART", "STAR"][self as usize]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Antenna {
    Star,
    Heart,
    Moon,
    Bolt,
    Ball,
    None,
}

impl Antenna {
    pub const ALL: [Antenna; 6] =
        [Antenna::Star, Antenna::Heart, Antenna::Moon, Antenna::Bolt, Antenna::Ball, Antenna::None];
    pub fn name(self) -> &'static str {
        ["STAR", "HEART", "MOON", "BOLT", "BALL", "NONE"][self as usize]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Corners {
    Round,
    Soft,
    Square,
}

impl Corners {
    pub const ALL: [Corners; 3] = [Corners::Round, Corners::Soft, Corners::Square];
    pub fn name(self) -> &'static str {
        ["ROUND", "SOFT", "SQUARE"][self as usize]
    }
    /// Corner radius of the faceplate, in design units.
    pub fn radius(self) -> f32 {
        [36.0, 18.0, 4.0][self as usize]
    }
    /// How far the faceplate's sides come in. The shell narrows towards the top, so the
    /// sharper the corners, the further in they must sit to stay on it.
    pub fn inset(self) -> f32 {
        [0.0, 8.0, 14.0][self as usize]
    }
}

/// The LCD's colours: the glass behind the dots, an unlit dot and a lit one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Lcd {
    Classic,
    Gray,
    Blue,
    Amber,
    Pink,
}

impl Lcd {
    pub const ALL: [Lcd; 5] = [Lcd::Classic, Lcd::Gray, Lcd::Blue, Lcd::Amber, Lcd::Pink];
    pub fn name(self) -> &'static str {
        ["CLASSIC", "GRAY", "BLUE", "AMBER", "PINK"][self as usize]
    }
    /// (background, unlit dot, lit dot)
    pub fn colors(self) -> (Rgb, Rgb, Rgb) {
        match self {
            Lcd::Classic => (crate::layout::LCD_BG, crate::layout::LCD_OFF, crate::layout::LCD_ON),
            Lcd::Gray => ([196, 198, 192], [186, 188, 182], [34, 34, 38]),
            Lcd::Blue => ([150, 190, 220], [140, 180, 210], [18, 34, 64]),
            Lcd::Amber => ([222, 186, 102], [212, 176, 94], [66, 36, 12]),
            Lcd::Pink => ([236, 190, 206], [226, 180, 196], [70, 20, 44]),
        }
    }
}

/// The whole look of the device. A `None` colour is AUTO: it follows the body colour (or,
/// for the accent, the pattern's own colours).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(default)]
pub struct Look {
    pub body: Rgb,
    pub pattern: Pattern,
    pub accent: Option<Rgb>,
    pub gloss: bool,
    pub buttons: Option<Rgb>,
    pub button_shape: ButtonShape,
    /// The A, B and C printed under the buttons.
    pub letters: bool,
    pub antenna: Antenna,
    pub antenna_color: Option<Rgb>,
    /// The key ring at the top of the shell.
    pub ring: bool,
    /// The panel the screen and icons sit on.
    pub face: Rgb,
    /// The rim around that panel.
    pub edge: Option<Rgb>,
    /// The frame right around the LCD.
    pub frame: Rgb,
    pub corners: Corners,
    pub lcd: Lcd,
    /// The menu icons around the screen.
    pub icons: Option<Rgb>,
    /// The icon that is selected.
    pub selected: Option<Rgb>,
    /// The attention icon while the pet is calling.
    pub alert: Option<Rgb>,
    /// The BYTEPET printed under the faceplate.
    pub name: Option<Rgb>,
}

/// The icons' colours when they are AUTO: faint when unlit, dark when lit.
pub const ICON_OFF: Rgb = [200, 198, 190];
pub const ICON_ON: Rgb = [40, 42, 40];

impl Look {
    /// A plain shell of one colour with everything else as it always was.
    pub const fn plain(body: Rgb) -> Look {
        Look {
            body,
            pattern: Pattern::Plain,
            accent: None,
            gloss: true,
            buttons: None,
            button_shape: ButtonShape::Round,
            letters: true,
            antenna: Antenna::Star,
            antenna_color: None,
            ring: true,
            face: CREAM,
            edge: None,
            frame: GRAY,
            corners: Corners::Round,
            lcd: Lcd::Classic,
            icons: None,
            selected: None,
            alert: None,
            name: None,
        }
    }
    const fn with_pattern(mut self, p: Pattern) -> Look {
        self.pattern = p;
        self
    }

    pub fn button_color(&self) -> Rgb {
        self.buttons.unwrap_or(self.body)
    }
    pub fn accent_color(&self) -> Rgb {
        self.accent.unwrap_or(self.pattern.default_accent())
    }
    pub fn icon_color(&self) -> Rgb {
        self.icons.unwrap_or(ICON_OFF)
    }
    pub fn selected_color(&self) -> Rgb {
        self.selected.unwrap_or(ICON_ON)
    }
    /// AUTO follows the selected colour.
    pub fn alert_color(&self) -> Rgb {
        self.alert.unwrap_or(self.selected_color())
    }
    /// AUTO is lighter than the body, or darker on a pale one.
    pub fn name_color(&self) -> Rgb {
        use crate::layout::{darken, lighten, luma};
        self.name.unwrap_or(if luma(self.body) > 0.75 { darken(self.body, 0.4) } else { lighten(self.body, 0.6) })
    }

    /// The index of this part's current option, or `None` when the save holds a value the
    /// menus don't offer (a colour typed into the file by hand, say).
    pub fn get(&self, f: Field) -> Option<usize> {
        let color = |c: Rgb| PALETTE.iter().position(|p| p.1 == c);
        let auto = |c: Option<Rgb>| match c {
            None => Some(0),
            Some(c) => color(c).map(|i| i + 1),
        };
        let flag = |b: bool| Some(b as usize);
        match f {
            Field::Body => color(self.body),
            Field::Pattern => Some(self.pattern as usize),
            Field::Accent => auto(self.accent),
            Field::Gloss => flag(self.gloss),
            Field::Buttons => auto(self.buttons),
            Field::ButtonShape => Some(self.button_shape as usize),
            Field::Letters => flag(self.letters),
            Field::Antenna => Some(self.antenna as usize),
            Field::AntennaColor => auto(self.antenna_color),
            Field::Ring => flag(self.ring),
            Field::Face => color(self.face),
            Field::Edge => auto(self.edge),
            Field::Frame => color(self.frame),
            Field::Corners => Some(self.corners as usize),
            Field::Lcd => Some(self.lcd as usize),
            Field::Icons => auto(self.icons),
            Field::Selected => auto(self.selected),
            Field::Alert => auto(self.alert),
            Field::Name => auto(self.name),
        }
    }

    /// Set a part to one of its options (out-of-range indexes wrap).
    pub fn set(&mut self, f: Field, i: usize) {
        let i = i % f.count();
        let color = PALETTE[i % PALETTE.len()].1;
        let auto = (i > 0).then(|| PALETTE[i - 1].1);
        match f {
            Field::Body => self.body = color,
            Field::Pattern => self.pattern = Pattern::ALL[i],
            Field::Accent => self.accent = auto,
            Field::Gloss => self.gloss = i == 1,
            Field::Buttons => self.buttons = auto,
            Field::ButtonShape => self.button_shape = ButtonShape::ALL[i],
            Field::Letters => self.letters = i == 1,
            Field::Antenna => self.antenna = Antenna::ALL[i],
            Field::AntennaColor => self.antenna_color = auto,
            Field::Ring => self.ring = i == 1,
            Field::Face => self.face = color,
            Field::Edge => self.edge = auto,
            Field::Frame => self.frame = color,
            Field::Corners => self.corners = Corners::ALL[i],
            Field::Lcd => self.lcd = Lcd::ALL[i],
            Field::Icons => self.icons = auto,
            Field::Selected => self.selected = auto,
            Field::Alert => self.alert = auto,
            Field::Name => self.name = auto,
        }
    }

    /// Move a part `dir` options along its list, wrapping round.
    pub fn step(&mut self, f: Field, dir: i32) {
        let n = f.count() as i32;
        let i = match self.get(f) {
            Some(i) => (i as i32 + dir).rem_euclid(n),
            None => 0,
        };
        self.set(f, i as usize);
    }

    /// What the menus show for a part: its option's name, or CUSTOM.
    pub fn value_name(&self, f: Field) -> &'static str {
        self.get(f).map_or("CUSTOM", |i| f.option_name(i))
    }

    /// Every part chosen at random.
    pub fn random(rng: &mut crate::rng::Pcg32) -> Look {
        let mut look = Look::default();
        for f in Field::ALL {
            look.set(f, rng.below(f.count()));
        }
        // keep the extras that look odd when missing, and the icons and name readable
        look.gloss = true;
        look.letters = true;
        for f in Part::Icons.fields() {
            look.set(*f, 0);
        }
        look
    }
}

impl Default for Look {
    fn default() -> Self {
        PRESETS[0].look
    }
}

/// The parts the menus group the fields under.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Part {
    Body,
    Buttons,
    Antenna,
    Screen,
    Icons,
}

impl Part {
    pub const ALL: [Part; 5] = [Part::Body, Part::Buttons, Part::Antenna, Part::Screen, Part::Icons];
    pub fn name(self) -> &'static str {
        ["BODY", "BUTTONS", "ANTENNA", "SCREEN", "ICONS"][self as usize]
    }
    pub fn fields(self) -> &'static [Field] {
        match self {
            Part::Body => &[Field::Body, Field::Pattern, Field::Accent, Field::Gloss],
            Part::Buttons => &[Field::Buttons, Field::ButtonShape, Field::Letters],
            Part::Antenna => &[Field::Antenna, Field::AntennaColor, Field::Ring],
            Part::Screen => &[Field::Face, Field::Edge, Field::Frame, Field::Corners, Field::Lcd],
            Part::Icons => &[Field::Icons, Field::Selected, Field::Alert, Field::Name],
        }
    }
}

/// One customisable setting.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Field {
    Body,
    Pattern,
    Accent,
    Gloss,
    Buttons,
    ButtonShape,
    Letters,
    Antenna,
    AntennaColor,
    Ring,
    Face,
    Edge,
    Frame,
    Corners,
    Lcd,
    Icons,
    Selected,
    Alert,
    Name,
}

#[derive(Clone, Copy)]
enum Kind {
    Color,
    AutoColor,
    Flag,
    Named(fn(usize) -> &'static str, usize),
}

impl Field {
    pub const ALL: [Field; 19] = [
        Field::Body,
        Field::Pattern,
        Field::Accent,
        Field::Gloss,
        Field::Buttons,
        Field::ButtonShape,
        Field::Letters,
        Field::Antenna,
        Field::AntennaColor,
        Field::Ring,
        Field::Face,
        Field::Edge,
        Field::Frame,
        Field::Corners,
        Field::Lcd,
        Field::Icons,
        Field::Selected,
        Field::Alert,
        Field::Name,
    ];

    fn kind(self) -> Kind {
        match self {
            Field::Body | Field::Face | Field::Frame => Kind::Color,
            Field::Accent
            | Field::Buttons
            | Field::AntennaColor
            | Field::Edge
            | Field::Icons
            | Field::Selected
            | Field::Alert
            | Field::Name => Kind::AutoColor,
            Field::Gloss | Field::Letters | Field::Ring => Kind::Flag,
            Field::Pattern => Kind::Named(|i| Pattern::ALL[i].name(), Pattern::ALL.len()),
            Field::ButtonShape => Kind::Named(|i| ButtonShape::ALL[i].name(), ButtonShape::ALL.len()),
            Field::Antenna => Kind::Named(|i| Antenna::ALL[i].name(), Antenna::ALL.len()),
            Field::Corners => Kind::Named(|i| Corners::ALL[i].name(), Corners::ALL.len()),
            Field::Lcd => Kind::Named(|i| Lcd::ALL[i].name(), Lcd::ALL.len()),
        }
    }

    /// The tray menu's name for it.
    pub fn title(self) -> &'static str {
        match self {
            Field::Body | Field::Buttons | Field::AntennaColor => "Color",
            Field::Pattern => "Pattern",
            Field::Accent => "Pattern color",
            Field::Gloss => "Shine",
            Field::ButtonShape | Field::Antenna => "Shape",
            Field::Letters => "A B C letters",
            Field::Ring => "Key ring",
            Field::Face => "Faceplate",
            Field::Edge => "Faceplate rim",
            Field::Frame => "Screen frame",
            Field::Corners => "Corners",
            Field::Lcd => "Screen color",
            Field::Icons => "Icon color",
            Field::Selected => "Selected icon",
            Field::Alert => "Alert icon",
            Field::Name => "BYTEPET text",
        }
    }

    /// The label on the settings panel, within its part's page.
    pub fn label(self) -> &'static str {
        match self {
            Field::Body | Field::Buttons | Field::AntennaColor => "COLOR",
            Field::Pattern => "PATTERN",
            Field::Accent => "ACCENT",
            Field::Gloss => "GLOSS",
            Field::ButtonShape | Field::Antenna => "SHAPE",
            Field::Letters => "LETTERS",
            Field::Ring => "RING",
            Field::Face => "FACE",
            Field::Edge => "EDGE",
            Field::Frame => "FRAME",
            Field::Corners => "CORNERS",
            Field::Lcd => "LCD",
            Field::Icons => "ICONS",
            Field::Selected => "SELECT",
            Field::Alert => "ALERT",
            Field::Name => "NAME",
        }
    }

    pub fn count(self) -> usize {
        match self.kind() {
            Kind::Color => PALETTE.len(),
            Kind::AutoColor => PALETTE.len() + 1,
            Kind::Flag => 2,
            Kind::Named(_, n) => n,
        }
    }

    pub fn option_name(self, i: usize) -> &'static str {
        match self.kind() {
            Kind::Color => PALETTE[i % PALETTE.len()].0,
            Kind::AutoColor if i == 0 => "AUTO",
            Kind::AutoColor => PALETTE[(i - 1) % PALETTE.len()].0,
            Kind::Flag => ["OFF", "ON"][i % 2],
            Kind::Named(name, n) => name(i % n),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Preset {
    pub name: &'static str,
    pub look: Look,
}

const fn preset(name: &'static str, look: Look) -> Preset {
    Preset { name, look }
}

pub const PRESETS: [Preset; 12] = [
    preset("BERRY", Look::plain(PINK)),
    preset("LIME", Look::plain(LIME)),
    preset("OCEAN", Look::plain(BLUE)),
    preset("GRAPE", Look::plain(PURPLE)),
    preset("ZEBRA", Look { buttons: Some(PINK), ..Look::plain(WHITE).with_pattern(Pattern::Zebra) }),
    preset("FLAME", Look::plain(ORANGE).with_pattern(Pattern::Flame)),
    preset("DONUT", Look::plain(ROSE).with_pattern(Pattern::Sprinkles)),
    preset("CAMO", Look::plain(OLIVE).with_pattern(Pattern::Camo)),
    preset(
        "COSMIC",
        Look {
            buttons: Some(GOLD),
            antenna: Antenna::Moon,
            antenna_color: Some(GOLD),
            edge: Some(BLACK),
            lcd: Lcd::Blue,
            ..Look::plain(NAVY).with_pattern(Pattern::Dots)
        },
    ),
    preset(
        "SWEET",
        Look {
            buttons: Some(RED),
            button_shape: ButtonShape::Heart,
            antenna: Antenna::Heart,
            antenna_color: Some(RED),
            lcd: Lcd::Pink,
            ..Look::plain(ROSE).with_pattern(Pattern::Hearts)
        },
    ),
    preset(
        "MINT",
        Look {
            buttons: Some(TEAL),
            button_shape: ButtonShape::Gem,
            antenna: Antenna::Ball,
            antenna_color: Some(TEAL),
            ..Look::plain(MINT).with_pattern(Pattern::Stripes)
        },
    ),
    preset(
        "RETRO",
        Look {
            buttons: Some(RED),
            button_shape: ButtonShape::Square,
            antenna: Antenna::Bolt,
            antenna_color: Some(RED),
            ring: false,
            edge: Some(BLACK),
            frame: BLACK,
            corners: Corners::Square,
            lcd: Lcd::Amber,
            ..Look::plain(SILVER)
        },
    ),
];

/// The name the menus show for a look: the preset it was last set to, another preset it
/// happens to match, or CUSTOM.
pub fn preset_name(look: &Look, last: u32) -> &'static str {
    let last = &PRESETS[last as usize % PRESETS.len()];
    if last.look == *look {
        return last.name;
    }
    PRESETS.iter().find(|p| p.look == *look).map_or("CUSTOM", |p| p.name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_option_round_trips() {
        for f in Field::ALL {
            let mut look = Look::default();
            for i in 0..f.count() {
                look.set(f, i);
                assert_eq!(look.get(f), Some(i), "{f:?} option {i}");
                assert!(!f.option_name(i).is_empty());
            }
            // stepping goes round the whole list and comes back
            let start = look;
            for _ in 0..f.count() {
                look.step(f, 1);
            }
            assert_eq!(look, start, "{f:?}");
            look.step(f, -1);
            look.step(f, 1);
            assert_eq!(look, start, "{f:?}");
        }
    }

    #[test]
    fn every_field_is_on_exactly_one_page() {
        for f in Field::ALL {
            let n = Part::ALL.iter().filter(|p| p.fields().contains(&f)).count();
            assert_eq!(n, 1, "{f:?}");
        }
    }

    #[test]
    fn presets_use_palette_colours_and_distinct_names() {
        for p in &PRESETS {
            for f in Field::ALL {
                assert!(p.look.get(f).is_some(), "{} {f:?} is not a menu option", p.name);
            }
            assert_eq!(PRESETS.iter().filter(|q| q.name == p.name).count(), 1);
            assert_eq!(PRESETS.iter().filter(|q| q.look == p.look).count(), 1, "{} duplicates a preset", p.name);
        }
        let mut look = PRESETS[3].look;
        assert_eq!(preset_name(&look, 3), "GRAPE");
        assert_eq!(preset_name(&look, 0), "GRAPE");
        look.step(Field::Ring, 1);
        assert_eq!(preset_name(&look, 3), "CUSTOM");
    }

    #[test]
    fn a_hand_typed_colour_shows_as_custom_and_steps_back_onto_the_palette() {
        let mut look = Look { body: [1, 2, 3], ..Look::default() };
        assert_eq!(look.value_name(Field::Body), "CUSTOM");
        look.step(Field::Body, 1);
        assert_eq!(look.body, PALETTE[0].1);
    }
}
