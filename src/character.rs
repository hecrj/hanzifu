use serde::{Deserialize, Serialize};

use iced::border;
use iced::theme::palette;
use iced::widget::{column, container, rich_text, row, span, text};
use iced::{Center, Element, Fill, Font, Theme, never};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Character {
    pub glyph: Glyph,
    pub pinyin: Pinyin,
    pub zhuyin: Zhuyin,
    pub meanings: Vec<Meaning>,
    pub difficulty: Difficulty,
}

impl Character {
    pub fn details<Message: 'static>(
        &self,
        highlight: Option<palette::Swatch>,
    ) -> Element<'_, Message> {
        column![
            row![
                text(&self.glyph)
                    .size(120)
                    .font(Font::DEFAULT)
                    .line_height(1.0)
                    .color_maybe(highlight.map(|swatch| swatch.base.color)),
                column![
                    text(&self.pinyin).size(50).line_height(1.0),
                    text(&self.zhuyin)
                        .size(30)
                        .line_height(1.0)
                        .color_maybe(highlight.map(|swatch| swatch.weak.color))
                ]
                .spacing(10)
            ]
            .spacing(10)
            .align_y(Center),
            Meaning::view(&self.meanings),
        ]
        .align_x(Center)
        .spacing(10)
        .into()
    }

    pub fn card<Message: 'static>(
        &self,
        highlight: Option<palette::Swatch>,
    ) -> Element<'_, Message> {
        container(self.details(highlight))
            .style(move |theme| {
                let Some(swatch) = highlight else {
                    return container::bordered_box(theme);
                };

                container::Style {
                    text_color: Some(swatch.base.color),
                    background: Some(swatch.weak.color.scale_alpha(0.05).into()),
                    border: border::rounded(2)
                        .width(1)
                        .color(swatch.weak.color.scale_alpha(0.7)),
                    ..container::Style::default()
                }
            })
            .center_x(Fill)
            .padding(10)
            .into()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Glyph(String);

impl Glyph {
    pub fn default_unlock() -> Self {
        Self(String::from("目"))
    }
}

impl<'a> text::IntoFragment<'a> for &'a Glyph {
    fn into_fragment(self) -> text::Fragment<'a> {
        self.0.as_str().into()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct Pinyin(String);

impl<'a> text::IntoFragment<'a> for &'a Pinyin {
    fn into_fragment(self) -> text::Fragment<'a> {
        self.0.as_str().into()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct Zhuyin(String);

impl<'a> text::IntoFragment<'a> for &'a Zhuyin {
    fn into_fragment(self) -> text::Fragment<'a> {
        self.0.as_str().into()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct Meaning(String);

impl Meaning {
    pub fn matches(&self, candidate: &str) -> bool {
        candidate.to_lowercase() == self.0
    }

    fn view<Message: 'static>(meanings: &[Self]) -> Element<'_, Message> {
        rich_text({
            let mut spans = vec![];

            for (i, meaning) in meanings.iter().enumerate() {
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
        .into()
    }
}

impl<'a> text::IntoFragment<'a> for &'a Meaning {
    fn into_fragment(self) -> text::Fragment<'a> {
        self.0.as_str().into()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub enum Difficulty {
    Easy,
    Normal,
    Hard,
    Extreme,
}
