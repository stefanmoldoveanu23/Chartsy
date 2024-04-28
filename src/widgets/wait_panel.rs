use iced::alignment::{Horizontal, Vertical};
use iced::{Alignment, Background, Color, Element, Length, Pixels};
use iced::widget::{Column, Container, Text};
use crate::icons::{Icon, ICON};

/// A widget that blocks user input. Displays a custom text.
pub struct WaitPanel<'a>
{
    /// The width of the [panel](WaitPanel).
    width: Length,

    /// The height of the [panel](WaitPanel).
    height: Length,

    /// The custom text to be displayed in the center of the [panel](WaitPanel).
    text: &'a str,

    /// The [styling](Appearance) of the [panel](WaitPanel).
    style: Appearance
}

impl<'a> WaitPanel<'a>
{
    /// Creates a new panel.
    pub fn new(text: &'a str) -> Self {
        WaitPanel {
            width: Length::Fill,
            height: Length::Fill,
            text,
            style: Appearance::default(),
        }
    }
}

impl<'a, Message, Theme, Renderer> From<WaitPanel<'a>> for Element<'a, Message, Theme, Renderer>
where
    Message: 'a + Clone,
    Theme: 'a + iced::widget::text::StyleSheet<Style=crate::theme::text::Text>
            + iced::widget::container::StyleSheet<Style=crate::theme::container::Container>,
    Renderer: 'a + iced::advanced::Renderer + iced::advanced::text::Renderer<Font=iced::Font>
{
    fn from(value: WaitPanel<'a>) -> Self {
        Container::new(
            Column::with_children(vec![
                Text::new(value.text)
                    .size(value.style.text_size)
                    .style(crate::theme::text::Text::Custom(value.style.text_color))
                    .into(),
                Text::new(Icon::Loading.to_string())
                    .font(ICON)
                    .size(value.style.text_size)
                    .style(crate::theme::text::Text::Custom(value.style.text_color))
                    .into()
            ])
                .spacing(10.0)
                .align_items(Alignment::Center)
        )
            .width(value.width)
            .height(value.height)
            .style(crate::theme::container::Container::Panel(value.style.background))
            .align_x(Horizontal::Center)
            .align_y(Vertical::Center)
            .into()
    }
}

/// The styling of a [WaitPanel].
pub struct Appearance {
    /// The background of the [WaitPanel].
    background: Background,

    /// The color of the text displayed in the [WaitPanel].
    text_color: Color,

    /// The size of the text displayed in the [WaitPanel].
    text_size: Pixels,
}

impl Default for Appearance {
    fn default() -> Self {
        let mut background = crate::theme::pallete::BACKGROUND;
        background.a = 0.2;

        Appearance {
            background: Background::Color(background),
            text_color: crate::theme::pallete::BACKGROUND,
            text_size: Pixels(20.0)
        }
    }
}

impl Appearance {
    /// Sets the background for the [panel](WaitPanel).
    pub fn background(mut self, background: impl Into<Background>) -> Self {
        self.background = background.into();

        self
    }

    /// Sets the text color for the [panel](WaitPanel).
    pub fn text_color(mut self, text_color: impl Into<Color>) -> Self {
        self.text_color = text_color.into();

        self
    }

    /// Sets the text size for the [panel](WaitPanel).
    pub fn text_size(mut self, text_size: impl Into<Pixels>) -> Self {
        self.text_size = text_size.into();

        self
    }
}