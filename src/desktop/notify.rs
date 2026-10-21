//! Desktop notifications for things the pet needs, rate-limited per kind.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use bytepet::content::Content;
use bytepet::core::title_case;
use bytepet::sim::{CallReason, Event, Game};

const GAP: Duration = Duration::from_secs(120);

pub struct Notifier {
    last: HashMap<&'static str, Instant>,
}

impl Notifier {
    pub fn new() -> Notifier {
        #[cfg(target_os = "macos")]
        {
            let _ = notify_rust::set_application("com.bytepet.desktop");
        }
        Notifier { last: HashMap::new() }
    }

    pub fn event(&mut self, ev: &Event, game: &Game, c: &Content) {
        if !game.settings.ui.notify {
            return;
        }
        let name = title_case(&game.pet.display_name(c));
        let (key, title, body): (&'static str, String, String) = match ev {
            Event::Call(CallReason::Need) => {
                ("need", format!("{name} needs you"), "Its hunger or happiness hearts are empty.".into())
            }
            Event::Call(CallReason::Lights) => ("lights", format!("{name} fell asleep"), "Turn off the lights.".into()),
            Event::Call(CallReason::Fake) => {
                ("fake", format!("{name} is calling"), "Does it really need something?".into())
            }
            Event::Sick => ("sick", format!("{name} is sick"), "Give it some medicine.".into()),
            Event::Hatch => ("hatch", "Your egg hatched!".into(), format!("Say hello to {name}.")),
            Event::Evolve(ch) => {
                ("evolve", format!("{name} grew up!"), format!("It's now a {}.", title_case(&c.name_of(ch))))
            }
            Event::Mail => ("mail", "You've got mail".into(), "Read it in Town \u{2192} Mail.".into()),
            Event::Gone => {
                ("gone", format!("{name} went home"), "It left for its home planet. A new egg is waiting.".into())
            }
            _ => return,
        };
        let now = Instant::now();
        if self.last.get(key).map(|t| now.duration_since(*t) < GAP).unwrap_or(false) {
            return;
        }
        self.last.insert(key, now);
        show(title, body);
    }
}

/// Show a notification off the UI thread (it can block briefly on D-Bus or the toast service).
fn show(title: String, body: String) {
    std::thread::spawn(move || {
        let mut n = notify_rust::Notification::new();
        n.appname("Bytepet").summary(&title).body(&body);
        #[cfg(target_os = "windows")]
        if let Some(id) = super::win_identity::app_id() {
            n.app_id(id);
        }
        let _ = n.show();
    });
}

/// A one-off notification (startup problems), regardless of the per-kind rate limit.
pub fn plain(title: &str, body: &str) {
    show(title.to_string(), body.to_string());
}
