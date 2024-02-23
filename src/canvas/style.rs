use iced::{Color, Element, Renderer, Command, Length};
use iced::widget::{Button, Column, Slider};
use mongodb::bson::{Bson, doc, Document};
use crate::scene::Message;
use crate::serde::{Deserialize, Serialize};
use crate::theme::Theme;
use crate::color_picker::ColorPicker;

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
    pub fn get_stroke_width(&self) -> f32
    {
        self.stroke.map_or_else(|| 0.0, |(width, _, _, _)| width)
    }

    /// Returns the color of the stroke in #rrggbb format.
    pub fn get_stroke_color(&self) -> String
    {
        self.stroke.map_or_else(|| "transparent".into(), |(_, color, _, _)| {
            let data = color.into_rgba8();
            format!("#{:02x?}{:02x?}{:02x?}", data[0], data[1], data[2])
        })
    }

    /// Returns the transparency of the stroke.
    pub fn get_stroke_alpha(&self) -> f32
    {
        self.stroke.map_or_else(|| 0.0, |(_, color, _, _)| {
            (10.0f32.powf(color.a) - 1.0) / 9.0
        })
    }

    /// Returns the fill in #rrggbb format.
    pub fn get_fill(&self) -> String
    {
        self.fill.map_or_else(|| "transparent".into(), |(color, _)| {
            let data = color.into_rgba8();
            format!("#{:02x?}{:02x?}{:02x?}", data[0], data[1], data[2])
        })
    }

    /// Returns the transparency of the fill.
    pub fn get_fill_alpha(&self) -> f32
    {
        self.fill.map_or_else(|| 0.0, |(color, _)| {
            (10.0f32.powf(color.a) - 1.0) / 9.0
        })
    }

    /// Modifies the stroke width of the [pending tool](crate::canvas::tool::Pending).
    pub(crate) fn stroke_width(mut self, stroke_width: impl Into<f32>) -> Self {
        if let Some((_, color, v1, v2)) = self.stroke {
            self.stroke = Some((stroke_width.into(), color, v1, v2));
        } else {
            self.stroke = Some((stroke_width.into(), Color::BLACK, false, false));
        }

        self
    }

    /// Modifies the stroke color of the [pending tool](crate::canvas::tool::Pending).
    pub(crate) fn stroke_color(mut self, stroke_color: impl Into<Color>) -> Self {
        if let Some((width, _, v1, v2)) = self.stroke {
            self.stroke = Some((width, stroke_color.into(), v1, v2));
        } else {
            self.stroke = Some((2.0, stroke_color.into(), false, false));
        }

        self
    }

    /// Modifies the fill color of the [pending tool](crate::canvas::tool::Pending).
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
    pub(crate) fn view<'a>(&self) -> Element<'a, StyleUpdate, Renderer<Theme>> {
        let mut column :Vec<Element<'a, StyleUpdate, Renderer<Theme>>>= vec![];

        if let Some((width, color, visibility_width, visibility_color)) = self.stroke {
            column.push(Button::new("Stroke width").on_press(StyleUpdate::ToggleStrokeWidth).into());
            if visibility_width {
                column.push(Slider::new(1.0..=5.0, width, StyleUpdate::StrokeWidth).into());
            }

            column.push(Button::new("Stroke color").on_press(StyleUpdate::ToggleStrokeColor).into());
            if visibility_color {
                let picker = ColorPicker::new(StyleUpdate::StrokeColor).color(color);
                column.push(picker.into());
            }
        }

        if let Some((color, visibility)) = self.fill {
            column.push(Button::new("Fill").on_press(StyleUpdate::ToggleFill).into());
            if visibility {
                let picker = ColorPicker::new(StyleUpdate::Fill).color(color);
                column.push(picker.into());
            }
        }

        Column::with_children(column).width(Length::Fixed(200.0)).into()
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

impl Serialize for Style {
    fn serialize(&self) -> Document {
        let mut document = doc!{};

        if let Some((width, color, _, _)) = self.stroke {
            document.insert("stroke", doc!{ "width": width, "color": color.serialize() });
        };

        if let Some((color, _)) = self.fill {
            document.insert("fill", color.serialize());
        }

        document
    }
}

impl Deserialize for Style {
    fn deserialize(document: Document) -> Self where Self: Sized {
        let mut style :Style= Style::default();

        if let Some(Bson::Document(stroke)) = document.get("stroke") {
            let mut stroke_width = 2.0;
            let mut stroke_color = Color::BLACK;

            if let Some(Bson::Double(width)) = stroke.get("width") {
                stroke_width = *width as f32;
            }

            if let Some(Bson::Document(color)) = stroke.get("color") {
                stroke_color = Color::deserialize(color.clone());
            }

            style.stroke = Some((stroke_width, stroke_color, false, false));
        }

        if let Some(Bson::Document(fill)) = document.get("fill") {
            style.fill = Some((Color::deserialize(fill.clone()), false));
        }

        style
    }
}