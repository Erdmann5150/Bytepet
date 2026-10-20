//! Piezo-style square-wave beeps through cpal. The stream opens on the first beep and is
//! paused when idle, so the app doesn't keep the audio device busy.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, Stream};

use bytepet::Tune;

struct Synth {
    notes: VecDeque<(f32, u32)>, // (frequency, samples left)
    phase: f32,
    rate: f32,
    fade: u32,
}

impl Synth {
    fn next(&mut self) -> f32 {
        let Some((freq, left)) = self.notes.front_mut() else { return 0.0 };
        let f = *freq;
        *left -= 1;
        let remaining = *left;
        if remaining == 0 {
            self.notes.pop_front();
        }
        if f <= 0.0 {
            return 0.0;
        }
        self.phase = (self.phase + f / self.rate) % 1.0;
        let v = if self.phase < 0.5 { 0.12 } else { -0.12 };
        // short fade-out at the end of each note avoids clicks
        if remaining < self.fade {
            v * remaining as f32 / self.fade as f32
        } else {
            v
        }
    }
}

pub struct Audio {
    synth: Option<Arc<Mutex<Synth>>>,
    stream: Option<Stream>,
    failed: bool,
    playing_until: Option<Instant>,
}

impl Audio {
    pub fn new() -> Audio {
        Audio { synth: None, stream: None, failed: false, playing_until: None }
    }

    fn open(&mut self) -> bool {
        if self.stream.is_some() {
            return true;
        }
        if self.failed {
            return false;
        }
        let ok = (|| {
            let host = cpal::default_host();
            let device = host.default_output_device()?;
            let cfg = device.default_output_config().ok()?;
            let channels = cfg.channels() as usize;
            let rate = cfg.sample_rate().0 as f32;
            let synth =
                Arc::new(Mutex::new(Synth { notes: VecDeque::new(), phase: 0.0, rate, fade: (rate * 0.004) as u32 }));
            let s2 = synth.clone();
            let err = |e| eprintln!("bytepet: audio error: {e}");
            let stream = match cfg.sample_format() {
                SampleFormat::F32 => device.build_output_stream(
                    &cfg.config(),
                    move |data: &mut [f32], _| {
                        let mut s = s2.lock().unwrap();
                        for frame in data.chunks_mut(channels) {
                            let v = s.next();
                            frame.fill(v);
                        }
                    },
                    err,
                    None,
                ),
                SampleFormat::I16 => device.build_output_stream(
                    &cfg.config(),
                    move |data: &mut [i16], _| {
                        let mut s = s2.lock().unwrap();
                        for frame in data.chunks_mut(channels) {
                            let v = (s.next() * i16::MAX as f32) as i16;
                            frame.fill(v);
                        }
                    },
                    err,
                    None,
                ),
                SampleFormat::U16 => device.build_output_stream(
                    &cfg.config(),
                    move |data: &mut [u16], _| {
                        let mut s = s2.lock().unwrap();
                        for frame in data.chunks_mut(channels) {
                            let v = ((s.next() + 1.0) * 0.5 * u16::MAX as f32) as u16;
                            frame.fill(v);
                        }
                    },
                    err,
                    None,
                ),
                _ => return None,
            }
            .ok()?;
            Some((synth, stream))
        })();
        match ok {
            Some((synth, stream)) => {
                self.synth = Some(synth);
                self.stream = Some(stream);
                true
            }
            None => {
                self.failed = true;
                false
            }
        }
    }

    pub fn play(&mut self, t: Tune) {
        if !self.open() {
            return;
        }
        let synth = self.synth.as_ref().unwrap();
        let mut s = synth.lock().unwrap();
        let mut ms_total = 0;
        for &(f, ms) in t.notes() {
            let n = (s.rate * ms as f32 / 1000.0) as u32;
            s.notes.push_back((f as f32, n.max(1)));
            ms_total += ms;
        }
        drop(s);
        if let Some(st) = &self.stream {
            let _ = st.play();
        }
        let until = Instant::now() + std::time::Duration::from_millis(ms_total as u64 + 300);
        self.playing_until = Some(self.playing_until.map_or(until, |u| u.max(until)));
    }

    /// Call every frame: pauses the stream once everything has played.
    pub fn idle(&mut self) {
        if let (Some(u), Some(st)) = (self.playing_until, &self.stream) {
            if Instant::now() > u {
                let _ = st.pause();
                self.playing_until = None;
            }
        }
    }
}
