use std::{collections::HashMap, hash::Hash};

use iced::{
    widget::{Button, Column, Row},
    Element, Length,
};

pub struct Tabs<'a, Type, Message, Theme, Renderer>
where
    Type: 'a + Hash + Eq + Default + Copy,
    Message: 'a + Clone,
    Renderer: 'a + iced::advanced::Renderer,
{
    tabs: HashMap<
        Type,
        (
            Element<'a, Message, Theme, Renderer>,
            Element<'a, Message, Theme, Renderer>,
        ),
    >,
    selected: Type,
    width: Length,
    height: Length,
    on_select: fn(Type) -> Message,
}

impl<'a, Type, Message, Theme, Renderer> Tabs<'a, Type, Message, Theme, Renderer>
where
    Type: 'a + Hash + Eq + Default + Copy,
    Message: 'a + Clone,
    Renderer: 'a + iced::advanced::Renderer,
{
    pub fn new_with_tabs(
        tabs: impl IntoIterator<
            Item = (
                Type,
                Element<'a, Message, Theme, Renderer>,
                Element<'a, Message, Theme, Renderer>,
            ),
        >,
        on_select: fn(Type) -> Message,
    ) -> Self {
        Tabs {
            tabs: tabs
                .into_iter()
                .map(|(tab, title, content)| (tab, (title, content)))
                .collect(),
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

impl<'a, Type, Message, Theme, Renderer> From<Tabs<'a, Type, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Type: 'a + Hash + Eq + Default + Copy,
    Message: 'a + Clone,
    Theme: 'a + iced::widget::text::Catalog + iced::widget::button::Catalog,
    Renderer: 'a + iced::advanced::Renderer + iced::advanced::text::Renderer,
{
    fn from(value: Tabs<'a, Type, Message, Theme, Renderer>) -> Self {
        let (titles, contents) = value.tabs.into_iter().fold(
            (vec![], vec![]),
            |(mut titles, mut contents), (tab, (title, content))| {
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
                        Button::new(title)
                            .on_press((value.on_select)(tab))
                            .width(Length::FillPortion(1))
                            .into()
                    })
                    .collect::<Vec<Element<'a, Message, Theme, Renderer>>>(),
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
