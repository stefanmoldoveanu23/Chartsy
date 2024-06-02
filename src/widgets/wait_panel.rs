use crate::utils::icons::{Icon, ICON};
use iced::alignment::{Horizontal, Vertical};
use iced::widget::{Column, Container, Text};
use iced::{Alignment, Element, Length, Pixels};

/// A widget that blocks user input. Displays a custom text.
pub struct WaitPanel {
    /// The width of the [panel](WaitPanel).
    width: Length,

    /// The height of the [panel](WaitPanel).
    height: Length,

    /// The custom text to be displayed in the center of the [panel](WaitPanel).
    text: String,

    /// The [styling](Appearance) of the [panel](WaitPanel).
    style: Appearance,
}

impl WaitPanel {
    /// Creates a new panel.
    pub fn new(text: impl Into<String>) -> Self {
        WaitPanel {
            width: Length::Fill,
            height: Length::Fill,
            text: text.into(),
            style: Appearance::default(),
        }
    }

    /// Sets the width of the [wait panel](WaitPanel).
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();

        self
    }

    /// Sets the height of the [wait panel](WaitPanel).
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();

        self
    }

    /// Sets the style of the [wait panel](WaitPanel).
    pub fn style(mut self, style: impl Into<Appearance>) -> Self {
        self.style = style.into();

        self
    }
}

impl<'a, Message, Theme, Renderer> From<WaitPanel> for Element<'a, Message, Theme, Renderer>
where
    Message: 'a + Clone,
    Theme: 'a + iced::widget::text::Catalog + iced::widget::container::Catalog,
    Renderer: 'a + iced::advanced::Renderer + iced::advanced::text::Renderer<Font = iced::Font>,
{
    fn from(value: WaitPanel) -> Self {
        Container::new(
            Column::with_children(vec![
                Text::new(value.text)
                    .size(value.style.text_size)
                    .horizontal_alignment(Horizontal::Center)
                    .vertical_alignment(Vertical::Center)
                    .into(),
                Text::new(Icon::Loading.to_string())
                    .font(ICON)
                    .size(value.style.text_size)
                    .into(),
            ])
            .spacing(10.0)
            .align_items(Alignment::Center),
        )
        .width(value.width)
        .height(value.height)
        .align_x(Horizontal::Center)
        .align_y(Vertical::Center)
        .into()
    }
}

/// The styling of a [WaitPanel].
pub struct Appearance {
    /// The size of the text displayed in the [WaitPanel].
    text_size: Pixels,
}

impl Default for Appearance {
    fn default() -> Self {
        Appearance {
            text_size: Pixels(20.0),
        }
    }
}

impl Appearance {
    /// Sets the text size for the [panel](WaitPanel).
    pub fn text_size(mut self, text_size: impl Into<Pixels>) -> Self {
        self.text_size = text_size.into();

        self
    }
}
