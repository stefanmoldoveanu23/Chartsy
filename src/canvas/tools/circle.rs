use crate::canvas::layer::CanvasAction;
use crate::canvas::style::Style;
use crate::serde::{Deserialize, Serialize};
use iced::event::Status;
use iced::mouse::Cursor;
use iced::widget::canvas::{Event, Fill, Frame, Geometry, Path, Stroke};
use iced::{keyboard, mouse, Color, Point, Rectangle, Renderer};
use json::object::Object;
use json::JsonValue;
use mongodb::bson::{doc, Bson, Document};
use std::fmt::Debug;
use std::sync::Arc;
use iced::keyboard::Key;
use svg::node::element::Group;

use crate::canvas::tool::{Pending, Tool};

#[derive(Clone)]
pub enum CirclePending {
    None,
    One(Point),
}

impl Pending for CirclePending {
    fn update(
        &mut self,
        event: Event,
        cursor: Point,
        style: Style,
    ) -> (Status, Option<CanvasAction>) {
        match event {
            Event::Mouse(mouse_event) => {
                let message = match mouse_event {
                    mouse::Event::ButtonPressed(mouse::Button::Left) => match self {
                        CirclePending::None => {
                            *self = CirclePending::One(cursor);
                            None
                        }
                        CirclePending::One(center) => {
                            let center_clone = center.clone();

                            *self = CirclePending::None;
                            Some(
                                CanvasAction::UseTool(Arc::new(Circle {
                                    center: center_clone,
                                    radius: cursor.distance(center_clone),
                                    style: style.clone(),
                                }))
                                .into(),
                            )
                        }
                    },
                    _ => None,
                };

                (Status::Captured, message)
            }
            Event::Keyboard(key_event) => match key_event {
                keyboard::Event::KeyPressed {
                    key: Key::Character(key),
                    ..
                } => {
                    let value = key.as_str();
                    if value == "S" {
                        *self = CirclePending::None;

                        (Status::Captured, None)
                    } else {
                        (Status::Ignored, None)
                    }
                }
                _ => (Status::Ignored, None),
            },
            _ => (Status::Ignored, None),
        }
    }

    fn draw(
        &self,
        renderer: &Renderer,
        bounds: Rectangle,
        cursor: Cursor,
        style: Style,
    ) -> Geometry {
        let mut frame = Frame::new(renderer, bounds.size());

        if let Some(cursor_position) = cursor.position_in(bounds) {
            match self {
                CirclePending::None => {}
                CirclePending::One(center) => {
                    let stroke = Path::new(|p| {
                        p.circle(*center, cursor_position.distance(*center));
                    });

                    if let Some((width, color, _, _)) = style.stroke {
                        frame.stroke(
                            &stroke,
                            Stroke::default().with_width(width).with_color(color),
                        );
                    }
                    if let Some((color, _)) = style.fill {
                        frame.fill(&stroke, Fill::from(color));
                    }
                }
            }
        };

        frame.into_geometry()
    }

    fn shape_style(&self, style: &mut Style) {
        if style.stroke.is_none() {
            style.stroke = Some((2.0, Color::BLACK, false, false));
        }
        if style.fill.is_none() {
            style.fill = Some((Color::TRANSPARENT, false));
        }
    }

    fn id(&self) -> String {
        String::from("Circle")
    }

    fn default() -> Self
    where
        Self: Sized,
    {
        CirclePending::None
    }

    fn boxed_clone(&self) -> Box<dyn Pending> {
        Box::new((*self).clone())
    }
}

#[derive(Debug, Clone)]
pub struct Circle {
    center: Point,
    radius: f32,
    style: Style,
}

impl Serialize<Document> for Circle {
    fn serialize(&self) -> Document {
        doc! {
            "center": Document::from(self.center.serialize()),
            "radius": self.radius,
            "style": Document::from(self.style.serialize()),
        }
    }
}

impl Deserialize<Document> for Circle {
    fn deserialize(document: &Document) -> Self
    where
        Self: Sized,
    {
        let mut circle = Circle {
            center: Point::default(),
            radius: 0.0,
            style: Style::default(),
        };

        if let Some(Bson::Document(center)) = document.get("center") {
            circle.center = Point::deserialize(center);
        }

        if let Some(Bson::Double(radius)) = document.get("radius") {
            circle.radius = *radius as f32;
        }

        if let Some(Bson::Document(style)) = document.get("style") {
            circle.style = Style::deserialize(style);
        }

        circle
    }
}

impl Serialize<Group> for Circle {
    fn serialize(&self) -> Group {
        let circle = svg::node::element::Circle::new()
            .set("cx", self.center.x)
            .set("cy", self.center.y)
            .set("r", self.radius)
            .set("stroke-width", self.style.get_stroke_width())
            .set("stroke", self.style.get_stroke_color())
            .set("stroke-opacity", self.style.get_stroke_alpha())
            .set("fill", self.style.get_fill())
            .set("fill-opacity", self.style.get_fill_alpha())
            .set("style", "mix-blend-mode:hard-light");

        Group::new().set("class", self.id()).add(circle)
    }
}

impl Serialize<Object> for Circle {
    fn serialize(&self) -> Object {
        let mut data = Object::new();

        data.insert("center", JsonValue::Object(self.center.serialize()));
        data.insert("radius", JsonValue::Number(self.radius.into()));
        data.insert("style", JsonValue::Object(self.style.serialize()));

        data
    }
}

impl Deserialize<Object> for Circle {
    fn deserialize(document: &Object) -> Self
    where
        Self: Sized,
    {
        let mut circle = Circle {
            center: Point::default(),
            radius: 0.0,
            style: Style::default(),
        };

        if let Some(JsonValue::Object(center)) = document.get("center") {
            circle.center = Point::deserialize(center);
        }
        if let Some(JsonValue::Number(radius)) = document.get("radius") {
            circle.radius = f32::from(*radius);
        }
        if let Some(JsonValue::Object(style)) = document.get("style") {
            circle.style = Style::deserialize(style);
        }

        circle
    }
}

impl Tool for Circle {
    fn add_to_frame(&self, frame: &mut Frame) {
        let circle = Path::new(|builder| {
            builder.circle(self.center, self.radius.clone());
        });

        if let Some((width, color, _, _)) = self.style.stroke {
            frame.stroke(
                &circle,
                Stroke::default().with_width(width).with_color(color),
            );
        }
        if let Some((color, _)) = self.style.fill {
            frame.fill(&circle, Fill::from(color));
        }
    }

    fn boxed_clone(&self) -> Box<dyn Tool> {
        Box::new((*self).clone())
    }

    fn id(&self) -> String {
        "Circle".into()
    }
}

impl Into<Box<dyn Tool>> for Box<Circle> {
    fn into(self) -> Box<dyn Tool> {
        self.boxed_clone()
    }
}
