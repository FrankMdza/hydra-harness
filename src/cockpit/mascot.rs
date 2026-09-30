//! The mascot in the cockpit's header: HydraPet's dragon, one head awake per live seat.
//!
//! The renderer is Python and stays Python. `hydra-mascot --frames <dir>` writes every pose of the
//! pet skin as a small ANSI file of half-block rows, and this module reads those files, parses the
//! few sequences the mascot emits into styled cells, and draws the pose the seats call for. The
//! parser is about forty lines because the mascot only ever writes `ESC [ 0 m`, truecolor
//! `38;2` and `48;2`, and `ESC [ K`; a crate for the whole of ANSI would be the larger of the two.
//!
//! The frames are committed under `integrations/herdr/mascot/frames/`. A change to the dragon
//! means running the command in that directory's README again.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use serde::Deserialize;

use super::snapshot::{Card, Lane, Snapshot};
use crate::live::State;

/// How many heads the dragon has, and so how many live seats wake one each.
pub const HEADS: usize = 3;

/// The poses `hydra-mascot --frames` writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Pose {
    Idle,
    Working,
    OneAsleep,
    TwoAsleep,
    AllAsleep,
    Refused,
    Meme,
    Fire,
}

impl Pose {
    pub const ALL: [Pose; 8] = [
        Pose::Idle,
        Pose::Working,
        Pose::OneAsleep,
        Pose::TwoAsleep,
        Pose::AllAsleep,
        Pose::Refused,
        Pose::Meme,
        Pose::Fire,
    ];

    /// The key of this pose in `index.json`.
    pub fn name(self) -> &'static str {
        match self {
            Pose::Idle => "idle",
            Pose::Working => "working",
            Pose::OneAsleep => "one_asleep",
            Pose::TwoAsleep => "two_asleep",
            Pose::AllAsleep => "all_asleep",
            Pose::Refused => "refused",
            Pose::Meme => "meme",
            Pose::Fire => "fire",
        }
    }
}

/// Where the committed frames live: under `HYDRA_HOME`, as `policies/` and `config/` do.
pub fn default_dir() -> PathBuf {
    crate::paths::hydra_home().join("integrations/herdr/mascot/frames")
}

/// The pose for `live` seats, waking heads left to right. Nothing live is the idle dragon, all
/// heads up, and three or more live is the working one. `AllAsleep` has no seat count that maps
/// to it: the ticket asks for idle when nothing is live, so the frame is generated and unused.
pub fn pose_for(live: usize) -> Pose {
    match live {
        0 => Pose::Idle,
        1 => Pose::TwoAsleep,
        2 => Pose::OneAsleep,
        _ => Pose::Working,
    }
}

type Row = Vec<(char, Style)>;

/// One pose as styled cells.
#[derive(Clone, Debug)]
struct Frame {
    rows: Vec<Row>,
}

/// Every pose, parsed.
#[derive(Clone, Debug)]
pub struct Frames {
    poses: HashMap<Pose, Frame>,
}

#[derive(Deserialize)]
struct Index {
    poses: HashMap<String, Entry>,
}

#[derive(Deserialize)]
struct Entry {
    file: String,
    rows: usize,
    cols: usize,
}

impl Frames {
    /// Reads `index.json` and each file it names, and refuses a directory whose files do not have
    /// the rows and columns the index says or that lacks a pose.
    pub fn load(dir: &Path) -> Result<Self> {
        let path = dir.join("index.json");
        let index: Index = serde_json::from_slice(
            &std::fs::read(&path).with_context(|| format!("reading {}", path.display()))?,
        )
        .with_context(|| format!("parsing {}", path.display()))?;
        let mut poses = HashMap::new();
        for pose in Pose::ALL {
            let Some(entry) = index.poses.get(pose.name()) else {
                bail!("{} names no pose {}", path.display(), pose.name());
            };
            if entry.file.contains(['/', '\\']) || entry.file.starts_with('.') {
                bail!(
                    "pose {}: {:?} is not a plain file name",
                    pose.name(),
                    entry.file
                );
            }
            let file = dir.join(&entry.file);
            let text = std::fs::read_to_string(&file)
                .with_context(|| format!("reading {}", file.display()))?;
            let rows = parse_ansi(&text);
            let cols = rows.first().map_or(0, Vec::len);
            if rows.len() != entry.rows || rows.iter().any(|row| row.len() != entry.cols) {
                bail!(
                    "pose {}: the index says {} rows of {} columns, {} has {} rows of {}",
                    pose.name(),
                    entry.rows,
                    entry.cols,
                    file.display(),
                    rows.len(),
                    cols
                );
            }
            poses.insert(pose, Frame { rows });
        }
        Ok(Self { poses })
    }

    /// Columns and rows of a pose.
    pub fn size(&self, pose: Pose) -> (u16, u16) {
        let frame = &self.poses[&pose];
        (
            frame.rows.first().map_or(0, Vec::len) as u16,
            frame.rows.len() as u16,
        )
    }

    /// Paints `pose` at the top left of `area`, clipped to it.
    pub fn draw(&self, pose: Pose, area: Rect, buf: &mut Buffer) {
        for (dy, row) in self.poses[&pose].rows.iter().enumerate() {
            if dy as u16 >= area.height {
                break;
            }
            for (dx, (ch, style)) in row.iter().enumerate() {
                if dx as u16 >= area.width {
                    break;
                }
                buf[(area.x + dx as u16, area.y + dy as u16)]
                    .set_char(*ch)
                    .set_style(*style);
            }
        }
    }
}

/// The half-block rows of a mascot frame as (character, style) cells. Anything but colour and
/// reset is skipped, which is `ESC [ K`.
fn parse_ansi(text: &str) -> Vec<Row> {
    let mut rows = Vec::new();
    for line in text.lines().filter(|line| !line.is_empty()) {
        let (mut fg, mut bg) = (Color::Reset, Color::Reset);
        let mut row = Row::new();
        let mut chars = line.chars();
        while let Some(ch) = chars.next() {
            if ch != '\x1b' {
                row.push((ch, Style::new().fg(fg).bg(bg)));
                continue;
            }
            if chars.next() != Some('[') {
                continue;
            }
            let mut params = String::new();
            let end = loop {
                match chars.next() {
                    Some(c) if c.is_ascii_digit() || c == ';' => params.push(c),
                    other => break other,
                }
            };
            if end != Some('m') {
                continue;
            }
            let codes: Vec<u8> = params.split(';').map(|p| p.parse().unwrap_or(0)).collect();
            let mut at = 0;
            while at < codes.len() {
                match (codes[at], codes.get(at + 1), codes.get(at + 4)) {
                    (0, ..) => (fg, bg) = (Color::Reset, Color::Reset),
                    (38 | 48, Some(2), Some(_)) => {
                        let rgb = Color::Rgb(codes[at + 2], codes[at + 3], codes[at + 4]);
                        if codes[at] == 38 {
                            fg = rgb;
                        } else {
                            bg = rgb;
                        }
                        at += 4;
                    }
                    _ => {}
                }
                at += 1;
            }
        }
        rows.push(row);
    }
    rows
}

/// The dragon and what it remembers between two snapshots, so a seat that ended since the last
/// read shows once and a history the cockpit opened on shows nothing.
#[derive(Clone, Debug)]
pub struct Mascot {
    frames: Frames,
    pose: Pose,
    extra: usize,
    live: usize,
    /// The session the cockpit's chat runs, once it has named it. Build ticket 177.
    session: Option<String>,
    /// The session's seat whose end made this read's flash, and its outcome.
    ended: Option<(String, String)>,
    /// Run states and waiting requests of the last read; `None` before the first.
    seen: Option<(HashMap<String, State>, HashSet<String>)>,
}

impl Mascot {
    pub fn new(frames: Frames) -> Self {
        Self {
            frames,
            pose: Pose::Idle,
            extra: 0,
            live: 0,
            session: None,
            ended: None,
            seen: None,
        }
    }

    pub fn pose(&self) -> Pose {
        self.pose
    }

    /// Live seats past the three heads.
    pub fn extra(&self) -> usize {
        self.extra
    }

    /// From the next read on, counts only the session `run_id` and the runs whose
    /// `parent_run_id` is it. Without a call every live run is a seat, as build 152 drew it.
    pub fn watch(&mut self, run_id: &str) {
        if self.session.as_deref() != Some(run_id) {
            self.session = Some(run_id.to_string());
        }
    }

    /// With a session, the seat that ended not ok at the last read and its outcome.
    pub fn ended(&self) -> Option<(&str, &str)> {
        self.ended
            .as_ref()
            .map(|(run_id, outcome)| (run_id.as_str(), outcome.as_str()))
    }

    /// Whether `card` is one of the seats the heads count.
    fn counts(&self, card: &Card) -> bool {
        self.session.as_ref().is_none_or(|session| {
            card.run_id == *session || card.parent_run_id.as_ref() == Some(session)
        })
    }

    /// Live seats.
    pub fn live(&self) -> usize {
        self.live
    }

    /// Columns and rows of the dragon.
    pub fn size(&self) -> (u16, u16) {
        self.frames.size(Pose::Idle)
    }

    /// Takes one read. A seat that went from live to ended `gated` gives the refused pose for
    /// this read, one that ended any other way but ok gives the meme head, and a request that
    /// left the waiting lane gives fire, in that order of precedence. Otherwise the heads awake
    /// are the live seats. With a session watched, a run outside it neither wakes a head nor
    /// flashes.
    pub fn observe(&mut self, snapshot: &Snapshot) {
        if !snapshot.read {
            return;
        }
        self.live = snapshot
            .cards
            .iter()
            .filter(|card| card.state == State::Live && self.counts(card))
            .count();
        self.extra = self.live.saturating_sub(HEADS);
        let mut flash = None;
        let mut ended = None;
        if let Some((states, waiting)) = &self.seen {
            for card in &snapshot.cards {
                if states.get(&card.run_id) != Some(&State::Live)
                    || card.state == State::Live
                    || !self.counts(card)
                {
                    continue;
                }
                let pose = match (card.lane(), card.outcome.as_deref()) {
                    (_, Some("gated")) => Pose::Refused,
                    (Lane::EndedNotOk, _) => Pose::Meme,
                    _ => continue,
                };
                if flash.is_none() || pose == Pose::Refused {
                    flash = Some(pose);
                    ended = Some(card);
                }
            }
            let now: HashSet<&str> = snapshot
                .requests
                .iter()
                .map(|r| r.request_id.as_str())
                .collect();
            if flash.is_none() && waiting.iter().any(|id| !now.contains(id.as_str())) {
                flash = Some(Pose::Fire);
            }
        }
        self.ended = ended.filter(|_| self.session.is_some()).map(|card| {
            let outcome = match (&card.outcome, card.state) {
                (Some(outcome), _) => outcome.clone(),
                (None, State::Abandoned) => "abandoned".to_string(),
                (None, _) => "no outcome".to_string(),
            };
            (card.run_id.clone(), outcome)
        });
        self.pose = flash.unwrap_or_else(|| pose_for(self.live));
        self.seen = Some((
            snapshot
                .cards
                .iter()
                .map(|card| (card.run_id.clone(), card.state))
                .collect(),
            snapshot
                .requests
                .iter()
                .map(|r| r.request_id.clone())
                .collect(),
        ));
    }

    /// Paints the current pose at the top left of `area`.
    pub fn draw(&self, area: Rect, buf: &mut Buffer) {
        self.frames.draw(self.pose, area, buf);
    }
}
