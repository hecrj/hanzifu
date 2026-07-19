mod character;
mod profile;
mod time;

use crate::character::Character;
use crate::profile::Profile;
use crate::time::{Duration, Instant, Time};

use iced::animation::Interpolable;
use iced::border;
use iced::keyboard;
use iced::mouse;
use iced::widget::{
    button, canvas, center, center_x, column, container, grid, pin, responsive, right, row,
    scrollable, space, stack, text,
};
use iced::window;
use iced::{
    Center, Color, Element, Fill, Font, Point, Radians, Rectangle, Renderer, Shrink, Subscription,
    Task, Theme, Vector,
};

use std::collections::BTreeMap;

fn main() -> iced::Result {
    iced::application(Hanzifu::new, Hanzifu::update, Hanzifu::view)
        .title("漢字傅")
        .subscription(Hanzifu::subscription)
        .theme(Hanzifu::theme)
        .default_font(Font::MONOSPACE)
        .run()
}

struct Hanzifu {
    characters: Vec<Character>,
    profile: Profile,
    screen: Screen,
}

enum Screen {
    Title,
    Library {
        current: Option<usize>,
        cache: canvas::Cache,
        cap: usize,
        now: Time,
    },
    Game(Game),
}

struct Game {
    score: u64,
    streak: u64,
    max_streak: u64,
    hits: BTreeMap<character::Glyph, u64>,
    cap: usize,
    targets: Vec<Target>,
    input: String,
    start: Time,
    now: Instant,
    last_target: Instant,
    paused: Duration,
    saved: bool,
    pause: Option<(Instant, Pause)>,
}

enum Pause {
    Unlocked(Vec<Character>),
}

impl Game {
    fn new(characters: &[Character], profile: &Profile) -> Self {
        let start = Time::now();

        Self {
            score: 0,
            streak: 0,
            max_streak: 0,
            hits: BTreeMap::new(),
            cap: profile.cap(characters, start.timestamp, |_| 0),
            targets: Vec::new(),
            input: String::new(),
            start,
            now: Instant::now(),
            last_target: Instant::now(),
            paused: Duration::default(),
            saved: false,
            pause: None,
        }
    }

    fn combo(&self) -> u64 {
        1 + (self.streak as f64 / 5.0).powf(0.67) as u64
    }

    fn level(&self) -> u64 {
        let duration = self.now - self.start;

        duration.as_secs() / 10
    }

    fn spawn_interval(&self) -> Duration {
        Duration::from_secs_f32((2.5 * 0.93f32.powi(self.level() as i32)).max(0.4))
    }

    fn target_duration(&self) -> Duration {
        Duration::from_secs_f32((8.0 * 0.93f32.powi(self.level() as i32)).max(1.5))
    }

    fn is_over(&self) -> bool {
        self.targets
            .iter()
            .any(|target| self.now >= target.expiration)
    }

    fn tick(&mut self, characters: &[Character], profile: &Profile, now: Instant) {
        if self.pause.is_some() {
            return;
        }

        let level = self.level();

        self.now = now - self.paused;

        if self.level() != level {
            let new_cap = profile.cap(characters, self.start.timestamp, |glyph| {
                self.hits.get(glyph).copied().unwrap_or_default()
            });

            if new_cap > self.cap {
                self.pause = Some((
                    now,
                    Pause::Unlocked(characters[self.cap + 1..=new_cap].to_vec()),
                ));
            }

            self.cap = new_cap;
        }

        if self.now - self.last_target >= self.spawn_interval() {
            const LEVELS: &[profile::Progress] = &[
                profile::Progress::Master,
                profile::Progress::Expert,
                profile::Progress::Familiar,
                profile::Progress::Learning,
            ];

            let level = match rand::random_range(0.0..=1.0) {
                ..=0.05 => 0,
                ..=0.3 => 1,
                ..=0.6 => 2,
                _ => 3,
            };

            let index = rand::random_range(..=self.cap);

            let Some((character, progress)) = LEVELS[level..]
                .iter()
                .copied()
                .filter_map(|progress| {
                    let character = characters[..=self.cap]
                        .iter()
                        .enumerate()
                        .filter_map(|(i, character)| {
                            (profile.progress(
                                character,
                                self.start.timestamp,
                                self.hits.get(&character.glyph).copied().unwrap_or_default(),
                            ) == progress)
                                .then_some(i)
                        })
                        .cycle()
                        .nth(index)?;

                    Some((character, progress))
                })
                .next()
            else {
                return;
            };

            let x = rand::random_range(0.0..=1.0);
            let y = rand::random_range(0.0..=1.0);

            self.targets.push(Target {
                character,
                progress,
                position: Point { x, y },
                start: self.now,
                expiration: self.now + self.target_duration(),
            });

            self.last_target = self.now;
        }
    }
}

#[derive(Debug, Clone)]
struct Target {
    character: usize,
    progress: profile::Progress,
    position: Point,
    start: Instant,
    expiration: Instant,
}

impl Target {
    fn view<'a>(&'a self, characters: &'a [Character], now: Instant) -> Element<'a, Message> {
        let character = &characters[self.character];
        let color = self.color(&Theme::CatppuccinMocha, now);

        stack![
            column![
                text(&character.glyph)
                    .font(Font::DEFAULT)
                    .size(120)
                    .line_height(1.0)
                    .color(color),
                text(&character.pinyin)
                    .size(30)
                    .line_height(1.0)
                    .color(color),
            ]
            .align_x(Center)
            .spacing(10)
            .padding(50)
        ]
        .push_under(
            canvas(Expiration { target: self, now })
                .width(Fill)
                .height(Fill),
        )
        .into()
    }

    fn color(&self, theme: &Theme, now: Instant) -> Color {
        self.progress.swatch(theme).base.color.interpolated(
            theme.palette().danger.strong.color,
            self.expiration_factor(now),
        )
    }

    fn expiration_factor(&self, now: Instant) -> f32 {
        (now - self.start).as_secs_f32() / (self.expiration - self.start).as_secs_f32()
    }
}

#[derive(Debug, Clone)]
enum Message {
    ProfileLoaded(Result<Profile, profile::Error>),
    ProfileSaved(Result<(), profile::Error>),
    Keyboard(keyboard::Event),
    Tick(Instant),
    NewGamePressed,
    LibraryPressed,
    QuitPressed,
    ContinuePressed,
    CharacterSelected(usize),
}

impl Hanzifu {
    pub fn new() -> (Self, Task<Message>) {
        let characters: Vec<Character> = ron::from_str(include_str!("../data/characters.ron"))
            .expect("characters must be deserializable");

        (
            Self {
                characters,
                profile: Profile::new(),
                screen: Screen::Title,
            },
            Task::perform(Profile::load(), Message::ProfileLoaded),
        )
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ProfileLoaded(Ok(history)) => {
                self.profile = history;

                Task::none()
            }
            Message::ProfileLoaded(Err(error)) => {
                dbg!(error);

                Task::none()
            }
            Message::ProfileSaved(result) => {
                let _ = dbg!(result);

                Task::none()
            }
            Message::Keyboard(event) => {
                match &mut self.screen {
                    Screen::Library { current: None, .. } => {
                        if let keyboard::Event::KeyPressed {
                            modified_key: keyboard::Key::Named(keyboard::key::Named::Escape),
                            ..
                        } = event
                        {
                            self.screen = Screen::Title;
                        }
                    }
                    Screen::Library {
                        current: Some(current),
                        cache,
                        ..
                    } => {
                        if let keyboard::Event::KeyPressed { modified_key, .. } = event {
                            match modified_key.as_ref() {
                                keyboard::Key::Named(keyboard::key::Named::ArrowLeft) => {
                                    if *current > 0 {
                                        *current -= 1;
                                    } else {
                                        *current = self.characters.len() - 1;
                                    }

                                    cache.clear();
                                }
                                keyboard::Key::Named(keyboard::key::Named::ArrowRight) => {
                                    *current += 1;

                                    if *current >= self.characters.len() {
                                        *current = 0;
                                    }

                                    cache.clear();
                                }
                                keyboard::Key::Named(keyboard::key::Named::Escape) => {
                                    self.open_library();
                                }
                                _ => {}
                            }
                        }
                    }
                    Screen::Game(game) if game.is_over() => {}
                    Screen::Game(game) => match event {
                        keyboard::Event::KeyPressed {
                            modified_key: keyboard::Key::Named(keyboard::key::Named::Backspace),
                            ..
                        } if game.pause.is_none() => {
                            let mut characters = game.input.chars();
                            let _ = characters.next_back();

                            game.input = characters.collect();
                        }
                        keyboard::Event::KeyPressed {
                            modified_key: keyboard::Key::Named(keyboard::key::Named::Enter),
                            ..
                        } if game.pause.is_none() => {
                            let input = game.input.trim();

                            if let Some(target) = game.targets.iter().position(|target| {
                                self.characters[target.character]
                                    .meanings
                                    .iter()
                                    .any(|meaning| meaning.matches(input))
                            }) {
                                game.streak += 1;
                                game.max_streak = game.max_streak.max(game.streak);
                                game.score += game.combo();

                                let target = game.targets.remove(target);

                                *game
                                    .hits
                                    .entry(self.characters[target.character].glyph.clone())
                                    .or_insert(0) += 1;
                            } else {
                                game.streak = 0;
                            }

                            game.input.clear();
                        }
                        keyboard::Event::KeyPressed {
                            modified_key: keyboard::Key::Named(keyboard::key::Named::Space),
                            ..
                        } if game.pause.is_some() => {
                            return self.update(Message::ContinuePressed);
                        }
                        keyboard::Event::KeyPressed {
                            text: Some(text), ..
                        } if game.pause.is_none() && text.is_ascii() => {
                            game.input.push_str(&text);
                        }
                        _ => {}
                    },
                    Screen::Title => {}
                }

                Task::none()
            }
            Message::Tick(now) => {
                let Screen::Game(game) = &mut self.screen else {
                    return Task::none();
                };

                if game.is_over() {
                    if game.saved {
                        return Task::none();
                    }

                    let miss = &self.characters[game.targets.first().unwrap().character];

                    self.profile.push(profile::Game {
                        score: game.score,
                        max_streak: game.max_streak,
                        hits: game.hits.clone(),
                        miss: miss.glyph.clone(),
                        finished_at: jiff::Timestamp::now(),
                        duration: game.now - game.start,
                    });

                    game.saved = true;

                    return Task::perform(self.profile.save(), Message::ProfileSaved);
                }

                game.tick(&self.characters, &self.profile, now);

                Task::none()
            }
            Message::NewGamePressed => {
                self.screen = Screen::Game(Game::new(&self.characters, &self.profile));

                Task::none()
            }
            Message::LibraryPressed => {
                self.open_library();

                Task::none()
            }
            Message::QuitPressed => iced::exit(),
            Message::ContinuePressed => {
                let Screen::Game(game) = &mut self.screen else {
                    return Task::none();
                };

                let Some((paused_at, _)) = &game.pause else {
                    return Task::none();
                };

                game.paused += Instant::now() - *paused_at;
                game.pause = None;

                Task::none()
            }
            Message::CharacterSelected(i) => {
                let Screen::Library { current, .. } = &mut self.screen else {
                    return Task::none();
                };

                *current = Some(i);

                Task::none()
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        match &self.screen {
            Screen::Title => {
                let choice = |label| button(text(label).size(30).center().width(Fill)).width(200);

                let menu = column![
                    choice("New Game").on_press(Message::NewGamePressed),
                    choice("Library")
                        .style(button::secondary)
                        .on_press(Message::LibraryPressed),
                    choice("Quit")
                        .style(button::danger)
                        .on_press(Message::QuitPressed),
                ]
                .width(Shrink)
                .spacing(20);

                center(
                    column![text("漢字傅").size(80).font(Font::DEFAULT), menu]
                        .spacing(20)
                        .align_x(Center),
                )
                .into()
            }
            Screen::Library {
                current: None,
                cap,
                now,
                ..
            } => container(
                scrollable(
                    grid(self.characters.iter().enumerate().map(|(i, character)| {
                        button(character.card(if i <= *cap {
                            Some(
                                self.profile
                                    .progress(character, now.timestamp, 0)
                                    .swatch(&self.theme()),
                            )
                        } else {
                            Some(self.theme().palette().secondary)
                        }))
                        .padding(0)
                        .style(button::text)
                        .on_press(Message::CharacterSelected(i))
                        .into()
                    }))
                    .fluid(400)
                    .height(Shrink)
                    .spacing(10),
                )
                .spacing(10),
            )
            .padding(10)
            .into(),
            Screen::Library {
                current: Some(current),
                now,
                cache,
                ..
            } => {
                let character = &self.characters[*current];
                let total_characters = text!("{} / {}", *current + 1, self.characters.len());

                struct Progress<'a> {
                    character: &'a Character,
                    profile: &'a Profile,
                    now: Time,
                    cache: &'a canvas::Cache,
                }

                impl canvas::Program<Message> for Progress<'_> {
                    type State = ();

                    fn draw(
                        &self,
                        _state: &Self::State,
                        renderer: &Renderer,
                        theme: &Theme,
                        bounds: Rectangle,
                        _cursor: mouse::Cursor,
                    ) -> Vec<canvas::Geometry> {
                        vec![self.cache.draw(renderer, bounds.size(), |frame| {
                            const PADDING: f32 = 40.0;

                            let palette = theme.palette();
                            let width = frame.width() - PADDING;
                            let height = frame.height() - PADDING;

                            frame.translate(Vector::new(PADDING, PADDING) / 2.0);

                            let checkpoints: Vec<_> = self
                                .profile
                                .checkpoints(self.character, self.now.timestamp, 0)
                                .collect();

                            let max_hits = checkpoints
                                .iter()
                                .map(|checkpoint| checkpoint.hits)
                                .next_back()
                                .unwrap_or_default()
                                .max(1);

                            let x_scale = width / checkpoints.len().saturating_sub(1) as f32;
                            let y_scale_hits = height / max_hits as f32;

                            let mut checkpoints = checkpoints
                                .into_iter()
                                .enumerate()
                                .map(|(i, checkpoint)| {
                                    let y = height * (1.0 - f32::from(checkpoint.hit_rate));
                                    let position = Point::new(i as f32 * x_scale, y);

                                    (position, checkpoint)
                                })
                                .peekable();

                            while let Some((position, checkpoint)) = checkpoints.next() {
                                let color = checkpoint.progress.swatch(theme).base.color;

                                frame.fill(&canvas::Path::circle(position, 3.0), color);

                                let hits_position = Point::new(
                                    position.x - 1.5,
                                    height - checkpoint.hits as f32 * y_scale_hits,
                                );

                                if let Some((next_position, next_checkpoint)) = checkpoints.peek() {
                                    let next_color =
                                        next_checkpoint.progress.swatch(theme).base.color;

                                    let gradient =
                                        canvas::gradient::Linear::new(position, *next_position)
                                            .add_stop(0.0, color)
                                            .add_stop(1.0, next_color);

                                    let next_hits_position = Point::new(
                                        next_position.x - 1.5,
                                        height - next_checkpoint.hits as f32 * y_scale_hits,
                                    );

                                    frame.stroke(
                                        &canvas::Path::line(hits_position, next_hits_position),
                                        canvas::Stroke {
                                            style: canvas::Style::Solid(
                                                palette.secondary.base.color,
                                            ),
                                            ..canvas::Stroke::default()
                                        },
                                    );

                                    frame.stroke(
                                        &canvas::Path::line(position, *next_position),
                                        canvas::Stroke {
                                            style: canvas::Style::Gradient(
                                                canvas::Gradient::Linear(gradient),
                                            ),
                                            width: 2.0,
                                            ..canvas::Stroke::default()
                                        },
                                    );
                                }

                                if checkpoint.game.miss == self.character.glyph {
                                    let cross = {
                                        const RADIUS: f32 = 3.0;

                                        let mut path = canvas::path::Builder::new();

                                        path.move_to(hits_position + Vector::new(-RADIUS, -RADIUS));
                                        path.line_to(hits_position + Vector::new(RADIUS, RADIUS));

                                        path.move_to(hits_position + Vector::new(RADIUS, -RADIUS));
                                        path.line_to(hits_position + Vector::new(-RADIUS, RADIUS));

                                        path.build()
                                    };

                                    frame.stroke(
                                        &cross,
                                        canvas::Stroke {
                                            style: canvas::Style::Solid(palette.danger.base.color),
                                            line_cap: canvas::LineCap::Round,
                                            width: 2.0,
                                            ..canvas::Stroke::default()
                                        },
                                    );
                                }
                            }
                        })]
                    }
                }

                let (progress, hits, misses, hit_rate) = self
                    .profile
                    .checkpoints(character, now.timestamp, 0)
                    .last()
                    .map(|checkpoint| {
                        (
                            checkpoint.progress,
                            checkpoint.hits,
                            checkpoint.misses,
                            checkpoint.hit_rate,
                        )
                    })
                    .unwrap_or_default();

                let theme = self.theme();
                let palette = theme.palette();

                column![
                    center(
                        column![
                            character.details(None),
                            container(
                                canvas(Progress {
                                    character,
                                    cache,
                                    profile: &self.profile,
                                    now: *now,
                                })
                                .width(Fill)
                                .height(Fill.max(200))
                            )
                            .style(container::bordered_box)
                            .padding(10),
                            column![
                                text!("{:.1}%", f32::from(hit_rate) * 100.0)
                                    .size(40)
                                    .color(progress.swatch(&theme).base.color)
                                    .line_height(1.0),
                                row![
                                    text(hits).color(palette.success.base.color).size(20),
                                    text(misses).color(palette.danger.base.color).size(20),
                                ]
                                .spacing(10)
                            ]
                            .spacing(10)
                            .align_x(Center)
                        ]
                        .width(Fill.max(600))
                        .spacing(10)
                        .align_x(Center)
                    ),
                    right(total_characters)
                ]
                .padding(10)
                .spacing(10)
                .into()
            }
            Screen::Game(game) => {
                let board = responsive(|size| {
                    stack(game.targets.iter().rev().map(|target| {
                        pin(target.view(&self.characters, game.now))
                            .x((size.width - 220.0) * target.position.x)
                            .y((size.height - 260.0) * target.position.y)
                            .into()
                    }))
                    .width(Fill)
                    .height(Fill)
                });

                let input = text(&game.input).size(60);

                column![
                    row![
                        text!("Level {}", game.level() + 1).size(30),
                        space::horizontal(),
                        (game.combo() > 1).then(|| text!("x{}", game.combo())),
                        text(game.score).size(30)
                    ]
                    .spacing(10)
                    .align_y(Center),
                    if game.is_over() {
                        stack![
                            board,
                            center(
                                column![
                                    text("Game Over").size(50).style(text::danger),
                                    scrollable(
                                        grid(game.targets.iter().map(|target| {
                                            self.characters[target.character].card(None)
                                        }))
                                        .spacing(10)
                                        .height(Shrink)
                                        .fluid(400)
                                    )
                                    .spacing(10)
                                    .height(Shrink.max(600)),
                                    button(text("Restart").size(30).width(Fill).center())
                                        .width(200)
                                        .on_press(Message::NewGamePressed),
                                ]
                                .spacing(10)
                                .align_x(Center)
                            )
                        ]
                    } else {
                        stack![
                            board,
                            game.pause.as_ref().map(|pause| match pause {
                                (_, Pause::Unlocked(characters)) => {
                                    center(
                                        container(
                                            column![
                                                text!(
                                                    "New Character{} Unlocked!",
                                                    if characters.len() == 1 { "" } else { "s" }
                                                )
                                                .size(50)
                                                .style(text::primary),
                                                scrollable(
                                                    column(
                                                        characters.iter().map(|character| {
                                                            character.card(None)
                                                        })
                                                    )
                                                    .width(400)
                                                    .spacing(10)
                                                )
                                                .spacing(20)
                                                .height(Shrink.max(600)),
                                                button(
                                                    text("Continue").size(30).width(Fill).center()
                                                )
                                                .width(200)
                                                .on_press(Message::ContinuePressed),
                                            ]
                                            .spacing(10)
                                            .align_x(Center),
                                        )
                                        .padding(20)
                                        .style(|_theme| {
                                            container::Style {
                                                background: Some(
                                                    Color::BLACK.scale_alpha(0.8).into(),
                                                ),
                                                border: border::rounded(10),
                                                ..container::Style::default()
                                            }
                                        }),
                                    )
                                }
                            })
                        ]
                    },
                    center_x(input)
                ]
                .padding(10)
                .spacing(10)
                .into()
            }
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        let keyboard = keyboard::listen().map(Message::Keyboard);

        let tick = if let Screen::Game { .. } = &self.screen {
            window::frames().map(Message::Tick)
        } else {
            Subscription::none()
        };

        Subscription::batch([keyboard, tick])
    }

    fn theme(&self) -> Theme {
        Theme::CatppuccinMocha
    }

    fn open_library(&mut self) {
        let now = Time::now();

        self.screen = Screen::Library {
            current: None,
            cap: self.profile.cap(&self.characters, now.timestamp, |_| 0),
            cache: canvas::Cache::new(),
            now,
        };
    }
}

struct Expiration<'a> {
    target: &'a Target,
    now: Instant,
}

impl<Message> canvas::Program<Message> for Expiration<'_> {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        use std::f32::consts::PI;

        let mut frame = canvas::Frame::new(renderer, bounds.size());

        let arc = {
            let mut builder = canvas::path::Builder::new();
            let factor = self.target.expiration_factor(self.now);

            builder.arc(canvas::path::Arc {
                center: frame.center(),
                radius: frame.width().min(frame.height()) / 2.0 - 5.0,
                start_angle: Radians(PI / 2.0),
                end_angle: Radians(PI / 2.0 - 2.0 * PI * (1.0 - factor)),
            });

            builder.build()
        };

        let color = self.target.color(theme, self.now);

        frame.stroke(
            &arc,
            canvas::Stroke {
                style: canvas::Style::Solid(color),
                width: 10.0,
                line_cap: canvas::LineCap::Round,
                ..canvas::Stroke::default()
            },
        );

        vec![frame.into_geometry()]
    }
}
