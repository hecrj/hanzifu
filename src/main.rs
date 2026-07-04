use iced::animation::Interpolable;
use iced::keyboard;
use iced::mouse;
use iced::widget::{
    button, canvas, center, center_x, column, container, grid, pin, responsive, rich_text, right,
    row, scrollable, space, span, stack, text,
};
use iced::window;
use iced::{
    Center, Color, Element, Fill, Font, Point, Radians, Rectangle, Renderer, Shrink, Subscription,
    Task, Theme, never,
};

use serde::Deserialize;

use std::time::{Duration, Instant};

fn main() -> Result<(), iced::Error> {
    iced::application(Hanzifu::new, Hanzifu::update, Hanzifu::view)
        .subscription(Hanzifu::subscription)
        .theme(Theme::CatppuccinMocha)
        .default_font(Font::MONOSPACE)
        .run()
}

struct Hanzifu {
    characters: Vec<Character>,
    screen: Screen,
}

enum Screen {
    Title,
    Library { current: Option<usize> },
    Game(Game),
}

struct Game {
    score: u64,
    hits: u64,
    streak: u64,
    targets: Vec<Target>,
    input: String,
    start: Instant,
    now: Instant,
    last_target: Instant,
}

impl Game {
    fn combo(&self) -> u64 {
        self.streak / 5 + 1
    }

    fn level(&self) -> u64 {
        let duration = self.now - self.start;

        duration.as_secs() / 30
    }

    fn spawn_interval(&self) -> Duration {
        Duration::from_secs_f32((2.5 * 0.93f32.powi(self.level() as i32)).max(0.4))
    }

    fn is_over(&self) -> bool {
        self.targets
            .iter()
            .any(|target| self.now >= target.expiration)
    }
}

#[derive(Debug, Clone)]
struct Target {
    character: usize,
    position: Point,
    start: Instant,
    expiration: Instant,
}

impl Target {
    fn color(&self, theme: &Theme, now: Instant) -> Color {
        let palette = theme.palette();

        palette
            .background
            .base
            .text
            .interpolated(palette.danger.strong.color, self.expiration_factor(now))
    }

    fn expiration_factor(&self, now: Instant) -> f32 {
        (now - self.start).as_secs_f32() / (self.expiration - self.start).as_secs_f32()
    }
}

#[derive(Debug, Clone)]
enum Message {
    Keyboard(keyboard::Event),
    Tick(Instant),
    NewGamePressed,
    LibraryPressed,
    QuitPressed,
    CharacterSelected(usize),
}

impl Hanzifu {
    pub fn new() -> Self {
        let characters: Vec<Character> = ron::from_str(include_str!("../data/characters.ron"))
            .expect("characters must be deserializable");

        Self {
            characters,
            screen: Screen::Title,
        }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Keyboard(event) => {
                match &mut self.screen {
                    Screen::Library {
                        current: Some(current),
                    } => {
                        if let keyboard::Event::KeyPressed { modified_key, .. } = event {
                            match modified_key.as_ref() {
                                keyboard::Key::Named(keyboard::key::Named::ArrowLeft) => {
                                    if *current > 0 {
                                        *current -= 1;
                                    } else {
                                        *current = self.characters.len() - 1;
                                    }
                                }
                                keyboard::Key::Named(keyboard::key::Named::ArrowRight) => {
                                    *current += 1;

                                    if *current >= self.characters.len() {
                                        *current = 0;
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    Screen::Game(game) => match event {
                        keyboard::Event::KeyPressed {
                            modified_key: keyboard::Key::Named(keyboard::key::Named::Backspace),
                            ..
                        } => {
                            let mut characters = game.input.chars();
                            let _ = characters.next_back();

                            game.input = characters.collect();
                        }
                        keyboard::Event::KeyPressed {
                            modified_key: keyboard::Key::Named(keyboard::key::Named::Enter),
                            ..
                        } => {
                            if let Some(target) = game.targets.iter().position(|target| {
                                self.characters[target.character]
                                    .meanings
                                    .iter()
                                    .any(|meaning| meaning.0 == game.input)
                            }) {
                                game.hits += 1;
                                game.streak += 1;
                                game.score += game.combo();

                                let _ = game.targets.remove(target);
                            } else {
                                game.streak = 0;
                            }

                            game.input.clear();
                        }
                        keyboard::Event::KeyPressed {
                            text: Some(text), ..
                        } if text.is_ascii() => {
                            game.input.push_str(&text);
                        }
                        _ => {}
                    },
                    Screen::Title | Screen::Library { current: None } => {}
                }

                Task::none()
            }
            Message::Tick(now) => {
                let Screen::Game(game) = &mut self.screen else {
                    return Task::none();
                };

                if game.is_over() {
                    return Task::none();
                }

                game.now = now;

                if game.now - game.last_target >= game.spawn_interval() {
                    let character = rand::random_range(..self.characters.len());

                    let x = rand::random_range(0.0..1.0);
                    let y = rand::random_range(0.0..1.0);

                    game.targets.push(Target {
                        character,
                        position: Point { x, y },
                        start: game.now,
                        expiration: game.now + Duration::from_secs(5),
                    });

                    game.last_target = game.now;
                }

                Task::none()
            }
            Message::NewGamePressed => {
                self.screen = Screen::Game(Game {
                    score: 0,
                    streak: 0,
                    hits: 0,
                    targets: Vec::new(),
                    input: String::new(),
                    start: Instant::now(),
                    now: Instant::now(),
                    last_target: Instant::now(),
                });

                Task::none()
            }
            Message::LibraryPressed => {
                self.screen = Screen::Library { current: None };

                Task::none()
            }
            Message::QuitPressed => iced::exit(),
            Message::CharacterSelected(i) => {
                let Screen::Library { current } = &mut self.screen else {
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
            Screen::Library { current: None } => container(
                scrollable(
                    grid(self.characters.iter().enumerate().map(|(i, character)| {
                        button(
                            container(character.view())
                                .style(container::bordered_box)
                                .center_x(Fill)
                                .padding(10),
                        )
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
            } => {
                let character = self.characters[*current].view();

                let total_characters = text!("{} / {}", *current + 1, self.characters.len());

                column![center(character), right(total_characters)]
                    .padding(10)
                    .spacing(10)
                    .into()
            }
            Screen::Game(game) => {
                let board = responsive(|size| {
                    stack(game.targets.iter().rev().map(|target| {
                        let character = &self.characters[target.character];

                        pin(stack![
                            container(
                                text(&character.glyph)
                                    .font(Font::DEFAULT)
                                    .size(120)
                                    .line_height(1.0)
                                    .color(target.color(&Theme::CatppuccinMocha, game.now))
                            )
                            .padding(30)
                        ]
                        .push_under(
                            canvas(Expiration {
                                target,
                                now: game.now,
                            })
                            .width(Fill)
                            .height(Fill),
                        ))
                        .x((size.width - 180.0) * target.position.x)
                        .y((size.height - 180.0) * target.position.y)
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
                            center(text("Game Over").size(120).style(text::danger))
                        ]
                        .into()
                    } else {
                        Element::from(board)
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
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct Character {
    glyph: Glyph,
    pinyin: Pinyin,
    bopomofo: Bopomofo,
    meanings: Vec<Meaning>,
    difficulty: Difficulty,
}

impl Character {
    pub fn view(&self) -> Element<'_, Message> {
        column![
            row![
                text(&self.glyph)
                    .size(120)
                    .font(Font::DEFAULT)
                    .line_height(1.0),
                column![
                    text(&self.pinyin).size(50).line_height(1.0),
                    text(&self.bopomofo)
                        .size(30)
                        .style(|theme: &Theme| text::Style {
                            color: Some(theme.palette().secondary.base.color)
                        })
                        .line_height(1.0)
                ]
                .spacing(10)
            ]
            .spacing(10)
            .align_y(Center),
            rich_text({
                let mut spans = vec![];

                for (i, meaning) in self.meanings.iter().enumerate() {
                    if i > 0 {
                        spans.push(
                            span(", ").color(Theme::CatppuccinMocha.palette().secondary.base.color),
                        );
                    }

                    spans.push(span(meaning));
                }

                spans
            })
            .on_link_click(never)
            .size(20)
        ]
        .align_x(Center)
        .spacing(10)
        .into()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
struct Glyph(String);

impl<'a> text::IntoFragment<'a> for &'a Glyph {
    fn into_fragment(self) -> text::Fragment<'a> {
        self.0.as_str().into()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
struct Pinyin(String);

impl<'a> text::IntoFragment<'a> for &'a Pinyin {
    fn into_fragment(self) -> text::Fragment<'a> {
        self.0.as_str().into()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
struct Bopomofo(String);

impl<'a> text::IntoFragment<'a> for &'a Bopomofo {
    fn into_fragment(self) -> text::Fragment<'a> {
        self.0.as_str().into()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
struct Meaning(String);

impl<'a> text::IntoFragment<'a> for &'a Meaning {
    fn into_fragment(self) -> text::Fragment<'a> {
        self.0.as_str().into()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
enum Difficulty {
    Easy,
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

        frame.stroke(
            &arc,
            canvas::Stroke {
                style: canvas::Style::Solid(self.target.color(theme, self.now)),
                width: 10.0,
                line_cap: canvas::LineCap::Round,
                ..canvas::Stroke::default()
            },
        );

        vec![frame.into_geometry()]
    }
}
