use crate::scene::Message;
use crate::utils::serde::{Deserialize, Serialize};
use crate::utils::theme::Theme;
use crate::widgets::ColorPicker;
use iced::alignment::Horizontal;
use iced::widget::{Button, Column, Slider, Text};
use iced::{Color, Command, Element, Length, Renderer};
use json::object::Object;
use json::JsonValue;
use mongodb::bson::{doc, Bson, Document};

/// A structure used to define the style of the drawn [tools](crate::canvas::tool::Tool).
///
/// Each field is an option that is locked/unlocked when switching to a
/// [pending tool](crate::canvas::tool::Pending) by the [shape_style function](crate::canvas::tool::Pending::shape_style).
#[derive(Debug, Default, Clone)]
pub struct Style {
    pub(crate) stroke: Option<(f32, Color, bool, bool)>,
    pub(crate) fill: Option<(Color, bool)>,
}

impl Style {
    /// Returns the width of the stroke.
    pub fn get_stroke_width(&self) -> f32 {
        self.stroke.map_or_else(|| 0.0, |(width, _, _, _)| width)
    }

    /// Returns the color of the stroke in #rrggbb format.
    pub fn get_stroke_color(&self) -> String {
        self.stroke.map_or_else(
            || "transparent".into(),
            |(_, color, _, _)| {
                let data = color.into_rgba8();
                format!("#{:02x?}{:02x?}{:02x?}", data[0], data[1], data[2])
            },
        )
    }

    /// Returns the transparency of the stroke.
    pub fn get_stroke_alpha(&self) -> f32 {
        self.stroke.map_or_else(
            || 0.0,
            |(_, color, _, _)| (10.0f32.powf(color.a) - 1.0) / 9.0,
        )
    }

    /// Returns the fill in #rrggbb format.
    pub fn get_fill(&self) -> String {
        self.fill.map_or_else(
            || "transparent".into(),
            |(color, _)| {
                let data = color.into_rgba8();
                format!("#{:02x?}{:02x?}{:02x?}", data[0], data[1], data[2])
            },
        )
    }

    /// Returns the transparency of the fill.
    pub fn get_fill_alpha(&self) -> f32 {
        self.fill
            .map_or_else(|| 0.0, |(color, _)| (10.0f32.powf(color.a) - 1.0) / 9.0)
    }

    /// Modifies the stroke width of the [pending tool](crate::canvas::tool::Pending).
    #[allow(dead_code)]
    pub(crate) fn stroke_width(mut self, stroke_width: impl Into<f32>) -> Self {
        if let Some((_, color, v1, v2)) = self.stroke {
            self.stroke = Some((stroke_width.into(), color, v1, v2));
        } else {
            self.stroke = Some((stroke_width.into(), Color::BLACK, false, false));
        }

        self
    }

    /// Modifies the stroke color of the [pending tool](crate::canvas::tool::Pending).
    #[allow(dead_code)]
    pub(crate) fn stroke_color(mut self, stroke_color: impl Into<Color>) -> Self {
        if let Some((width, _, v1, v2)) = self.stroke {
            self.stroke = Some((width, stroke_color.into(), v1, v2));
        } else {
            self.stroke = Some((2.0, stroke_color.into(), false, false));
        }

        self
    }

    /// Modifies the fill color of the [pending tool](crate::canvas::tool::Pending).
    #[allow(dead_code)]
    pub(crate) fn fill(mut self, fill: impl Into<Color>) -> Self {
        if let Some((_, visible)) = self.fill {
            self.fill = Some((fill.into(), visible));
        } else {
            self.fill = Some((fill.into(), false));
        }

        self
    }

    /// Updates the [Style] based on user input.
    pub(crate) fn update(&mut self, message: StyleUpdate) -> Command<Message> {
        match message {
            StyleUpdate::ToggleStrokeWidth => {
                if let Some((width, color, visible, v2)) = self.stroke {
                    self.stroke = Some((width, color, !visible, v2));
                }
            }
            StyleUpdate::StrokeWidth(width) => {
                if let Some((_, color, v1, v2)) = self.stroke {
                    self.stroke = Some((width, color, v1, v2));
                }
            }
            StyleUpdate::ToggleStrokeColor => {
                if let Some((width, color, v1, visible)) = self.stroke {
                    self.stroke = Some((width, color, v1, !visible));
                }
            }
            StyleUpdate::StrokeColor(color) => {
                if let Some((width, _, v1, v2)) = self.stroke {
                    self.stroke = Some((width, color, v1, v2));
                }
            }
            StyleUpdate::ToggleFill => {
                if let Some((color, visible)) = self.fill {
                    self.fill = Some((color, !visible));
                }
            }
            StyleUpdate::Fill(color) => {
                if let Some((_, visible)) = self.fill {
                    self.fill = Some((color, visible));
                }
            }
        }

        Command::none()
    }

    /// Returns an interactable settings section for the [Style].
    pub(crate) fn view<'a>(&self) -> Element<'a, StyleUpdate, Theme, Renderer> {
        let mut column: Vec<Element<'a, StyleUpdate, Theme, Renderer>> = vec![];

        let get_button_style = |condition: bool| {
            if condition {
                iced::widget::button::primary
            } else {
                iced::widget::button::secondary
            }
        };

        /*let get_text_style = |condition: bool| {
            if condition {
                theme::text::Text::Dark
            } else {
                theme::text::Text::Light
            }
        };*/

        if let Some((width, color, visibility_width, visibility_color)) = self.stroke {
            column.push(
                Button::new(
                    Text::new("Stroke width")
                        //.style(get_text_style(visibility_width))
                        .horizontal_alignment(Horizontal::Center),
                )
                .on_press(StyleUpdate::ToggleStrokeWidth)
                .style(get_button_style(visibility_width))
                .width(Length::Fill)
                .into(),
            );
            if visibility_width {
                column.push(Slider::new(1.0..=5.0, width, StyleUpdate::StrokeWidth).into());
            }

            column.push(
                Button::new(
                    Text::new("Stroke color")
                        //.style(get_text_style(visibility_color))
                        .horizontal_alignment(Horizontal::Center),
                )
                .on_press(StyleUpdate::ToggleStrokeColor)
                .style(get_button_style(visibility_color))
                .width(Length::Fill)
                .into(),
            );
            if visibility_color {
                let picker = ColorPicker::new(color.r, color.g, color.b, StyleUpdate::StrokeColor);
                column.push(picker.into());
            }
        }

        if let Some((color, visibility)) = self.fill {
            column.push(
                Button::new(
                    Text::new("Fill")
                        //.style(get_text_style(visibility))
                        .horizontal_alignment(Horizontal::Center),
                )
                .on_press(StyleUpdate::ToggleFill)
                .style(get_button_style(visibility))
                .width(Length::Fill)
                .into(),
            );

            if visibility {
                let picker = ColorPicker::new(color.r, color.g, color.b, StyleUpdate::Fill);
                column.push(picker.into());
            }
        }

        Column::with_children(column)
            .padding(8.0)
            .spacing(10.0)
            .into()
    }
}

/// An enum of possible modifications a user can make to the [Style].
#[derive(Clone)]
pub enum StyleUpdate {
    ToggleStrokeWidth,
    StrokeWidth(f32),
    ToggleStrokeColor,
    StrokeColor(Color),
    ToggleFill,
    Fill(Color),
}

impl Serialize<Document> for Style {
    fn serialize(&self) -> Document {
        let mut document = doc! {};

        if let Some((width, color, _, _)) = self.stroke {
            document.insert(
                "stroke",
                doc! { "width": width, "color": Document::from(color.serialize()) },
            );
        };

        if let Some((color, _)) = self.fill {
            document.insert("fill", Document::from(color.serialize()));
        }

        document
    }
}

impl Deserialize<Document> for Style {
    fn deserialize(document: &Document) -> Self
    where
        Self: Sized,
    {
        let mut style: Style = Style::default();

        if let Some(Bson::Document(stroke)) = document.get("stroke") {
            let mut stroke_width = 2.0;
            let mut stroke_color = Color::BLACK;

            if let Some(Bson::Double(width)) = stroke.get("width") {
                stroke_width = *width as f32;
            }

            if let Some(Bson::Document(color)) = stroke.get("color") {
                stroke_color = Color::deserialize(color);
            }

            style.stroke = Some((stroke_width, stroke_color, false, false));
        }

        if let Some(Bson::Document(fill)) = document.get("fill") {
            style.fill = Some((Color::deserialize(fill), false));
        }

        style
    }
}

impl Serialize<Object> for Style {
    fn serialize(&self) -> Object {
        let mut data = Object::new();

        if let Some((width, color, _, _)) = self.stroke {
            let mut stroke = Object::new();
            stroke.insert("width", JsonValue::Number(width.into()));
            stroke.insert("color", JsonValue::Object(color.serialize()));

            data.insert("stroke", JsonValue::Object(stroke));
        };

        if let Some((color, _)) = self.fill {
            data.insert("fill", JsonValue::Object(color.serialize()));
        }

        data
    }
}

impl Deserialize<Object> for Style {
    fn deserialize(document: &Object) -> Self
    where
        Self: Sized,
    {
        let mut style = Style::default();

        if let Some(JsonValue::Object(stroke)) = document.get("stroke") {
            let mut width = 0.0;
            let mut color = Color::default();

            if let Some(JsonValue::Number(width_value)) = stroke.get("width") {
                width = f32::from(*width_value);
            }
            if let Some(JsonValue::Object(color_value)) = stroke.get("color") {
                color = Color::deserialize(color_value);
            }

            style.stroke = Some((width, color, false, false));
        }

        if let Some(JsonValue::Object(fill)) = document.get("fill") {
            style.fill = Some((Color::deserialize(fill), false));
        }

        style
    }
}
