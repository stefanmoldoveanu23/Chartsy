use iced::{
    alignment::Horizontal,
    widget::{
        button::{Status, Style},
        Button, Column, Row, Text,
    },
    Element, Length,
};

use crate::utils::theme;

pub struct Tabs<'a, Type, Message, Theme, Renderer>
where
    Type: 'a + Eq + Default + Copy,
    Message: 'a + Clone,
    Renderer: 'a + iced::advanced::Renderer,
{
    tabs: Vec<(Type, String, Element<'a, Message, Theme, Renderer>)>,
    selected: Type,
    width: Length,
    height: Length,
    on_select: fn(Type) -> Message,
}

impl<'a, Type, Message, Theme, Renderer> Tabs<'a, Type, Message, Theme, Renderer>
where
    Type: 'a + Eq + Default + Copy,
    Message: 'a + Clone,
    Renderer: 'a + iced::advanced::Renderer,
{
    pub fn new_with_tabs(
        tabs: impl IntoIterator<Item = (Type, String, Element<'a, Message, Theme, Renderer>)>,
        on_select: fn(Type) -> Message,
    ) -> Self {
        Tabs {
            tabs: tabs.into_iter().collect(),
            selected: Type::default(),
            width: Length::Shrink,
            height: Length::Shrink,
            on_select,
        }
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();

        self
    }

    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();

        self
    }

    pub fn selected(mut self, selected: Type) -> Self {
        self.selected = selected;

        self
    }
}

impl<'a, Type, Message, Renderer> From<Tabs<'a, Type, Message, theme::Theme, Renderer>>
    for Element<'a, Message, theme::Theme, Renderer>
where
    Type: 'a + Eq + Default + Copy,
    Message: 'a + Clone,
    Renderer: 'a + iced::advanced::Renderer + iced::advanced::text::Renderer,
{
    fn from(value: Tabs<'a, Type, Message, theme::Theme, Renderer>) -> Self {
        let (titles, contents) = value.tabs.into_iter().fold(
            (vec![], vec![]),
            |(mut titles, mut contents), (tab, title, content)| {
                titles.push((tab, title));
                contents.push((tab, content));
                (titles, contents)
            },
        );

        Column::with_children(vec![
            Row::with_children(
                titles
                    .into_iter()
                    .map(|(tab, title)| {
                        let style: fn(&theme::Theme, Status) -> Style = if tab == value.selected {
                            theme::button::primary_tab
                        } else {
                            theme::button::secondary_tab
                        };

                        Button::new(
                            Text::new(title)
                                .width(Length::Fill)
                                .horizontal_alignment(Horizontal::Center),
                        )
                        .on_press((value.on_select)(tab))
                        .width(Length::FillPortion(1))
                        .style(style)
                        .into()
                    })
                    .collect::<Vec<Element<'a, Message, theme::Theme, Renderer>>>(),
            )
            .width(Length::Fill)
            .into(),
            contents
                .into_iter()
                .find_map(|(tag, content)| {
                    if tag == value.selected {
                        Some(content)
                    } else {
                        None
                    }
                })
                .unwrap(),
        ])
        .width(value.width)
        .height(value.height)
        .into()
    }
}
