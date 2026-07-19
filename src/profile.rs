use crate::character::{self, Character};

use jiff::Timestamp;
use serde::{Deserialize, Serialize};
use tokio::fs;

use std::collections::BTreeMap;
use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use iced::Theme;
use iced::theme::palette;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Profile {
    games: Vec<Game>,
}

impl Profile {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn load() -> Result<Profile, Error> {
        let data = fs::read_to_string(Self::path()?).await?;
        let history = ron::from_str(&data)?;

        Ok(history)
    }

    pub fn save(&self) -> impl Future<Output = Result<(), Error>> + 'static {
        let history = self.clone();

        async move {
            let path = Self::path()?;

            if let Some(directory) = path.parent() {
                fs::create_dir_all(directory).await?;
            }

            let data = ron::to_string(&history).expect("history must be serializable");
            fs::write(path, data).await?;

            Ok(())
        }
    }

    pub fn push(&mut self, game: Game) {
        self.games.push(game);
    }

    pub fn progress(&self, character: &Character, at: Timestamp, extra_hits: u64) -> Progress {
        self.checkpoints(character, at, extra_hits)
            .last()
            .map(|progression| progression.progress)
            .unwrap_or(Progress::Learning)
    }

    pub fn checkpoints(
        &self,
        character: &Character,
        at: Timestamp,
        extra_hits: u64,
    ) -> impl Iterator<Item = Checkpoint<'_>> {
        const INTERVAL: Duration = Duration::from_secs(60 * 60 * 24 * 14); // 2 weeks

        let recent_games = self.games.iter().rev().take_while(move |game| {
            let seconds = (at - game.finished_at).get_seconds();

            seconds >= 0 && Duration::from_secs(seconds as u64) <= INTERVAL
        });

        let mut hits = extra_hits;
        let mut misses = 0;

        self.games[self.games.len() - recent_games.count()..]
            .iter()
            .map(move |game| {
                let miss = game.miss == character.glyph;

                hits += game.hits.get(&character.glyph).copied().unwrap_or_default();

                if miss {
                    misses += 1;
                }

                let minimum_hits = match character.difficulty {
                    character::Difficulty::Easy => 2,
                    character::Difficulty::Normal => 4,
                    character::Difficulty::Hard => 8,
                    character::Difficulty::Extreme => 10,
                };

                let hit_rate = hits as f32 / (hits + misses).max(1) as f32;

                let progress = match hit_rate {
                    _ if hits < minimum_hits => Progress::Learning,
                    0.98.. if hits > minimum_hits * 5 => Progress::Master,
                    0.8.. if hits > minimum_hits * 3 => Progress::Expert,
                    0.7..0.8 => Progress::Familiar,
                    _ => Progress::Learning,
                };

                Checkpoint {
                    game,
                    progress,
                    hit_rate: HitRate(hit_rate),
                    hits,
                    misses,
                    miss,
                }
            })
    }

    pub fn cap(
        &self,
        characters: &[Character],
        at: Timestamp,
        extra_hits: impl Fn(&character::Glyph) -> u64,
    ) -> usize {
        characters
            .iter()
            .position(|character| {
                self.progress(character, at, extra_hits(&character.glyph)) < Progress::Expert
            })
            .unwrap_or(characters.len() - 1)
            .max(2)
    }

    fn path() -> Result<PathBuf, Error> {
        Ok(data_dir()?.join("games.ron"))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Checkpoint<'a> {
    pub game: &'a Game,
    pub progress: Progress,
    pub hit_rate: HitRate,
    pub hits: u64,
    pub misses: u64,
    pub miss: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum Progress {
    #[default]
    Learning,
    Familiar,
    Expert,
    Master,
}

impl Progress {
    pub fn swatch(self, theme: &Theme) -> palette::Swatch {
        let palette = theme.palette();

        match self {
            Progress::Learning => palette.danger,
            Progress::Familiar => palette.warning,
            Progress::Expert => palette.primary,
            Progress::Master => palette.success,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default)]
pub struct HitRate(f32);

impl From<HitRate> for f32 {
    fn from(hit_rate: HitRate) -> Self {
        hit_rate.0
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum Error {
    IoFailed(Arc<io::Error>),
    LoadFailed(ron::error::SpannedError),
    DirectoryNotFound,
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Self::IoFailed(Arc::new(error))
    }
}

impl From<ron::error::SpannedError> for Error {
    fn from(error: ron::error::SpannedError) -> Self {
        Self::LoadFailed(error)
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Game {
    pub score: u64,
    pub max_streak: u64,
    pub hits: BTreeMap<character::Glyph, u64>,
    pub miss: character::Glyph,
    pub finished_at: Timestamp,
    pub duration: Duration,
}

fn data_dir() -> Result<PathBuf, Error> {
    let Some(project) = directories::ProjectDirs::from("", "hecrj", "Hanzifu") else {
        return Err(Error::DirectoryNotFound);
    };

    Ok(project.data_dir().to_path_buf())
}
