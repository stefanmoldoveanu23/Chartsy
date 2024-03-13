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
use svg::node::element::path::Data;
use svg::node::element::Group;

use crate::canvas::tool::{Pending, Tool};

#[derive(Clone)]
pub enum TrianglePending {
    None,
    One(Point),
    Two(Point, Point),
}

impl Pending for TrianglePending {
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
                        TrianglePending::None => {
                            *self = TrianglePending::One(cursor);
                            None
                        }
                        TrianglePending::One(start) => {
                            *self = TrianglePending::Two(*start, cursor);
                            None
                        }
                        TrianglePending::Two(point1, point2) => {
                            let point1_clone = point1.clone();
                            let point2_clone = point2.clone();

                            *self = TrianglePending::None;
                            Some(
                                CanvasAction::UseTool(Arc::new(Triangle {
                                    point1: point1_clone,
                                    point2: point2_clone,
                                    point3: cursor,
                                    style,
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
                        *self = TrianglePending::None;

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
                TrianglePending::None => {}
                TrianglePending::One(point1) => {
                    let stroke = Path::new(|p| {
                        p.move_to(*point1);
                        p.line_to(cursor_position);
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
                TrianglePending::Two(point1, point2) => {
                    let stroke = Path::new(|p| {
                        p.move_to(*point1);
                        p.line_to(*point2);
                        p.line_to(cursor_position);
                        p.line_to(*point1);
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
        String::from("Triangle")
    }

    fn default() -> Self
    where
        Self: Sized,
    {
        TrianglePending::None
    }

    fn boxed_clone(&self) -> Box<dyn Pending> {
        Box::new((*self).clone())
    }
}

#[derive(Debug, Clone)]
pub struct Triangle {
    point1: Point,
    point2: Point,
    point3: Point,
    style: Style,
}

impl Serialize<Document> for Triangle {
    fn serialize(&self) -> Document {
        doc! {
            "point1": Document::from(self.point1.serialize()),
            "point2": Document::from(self.point2.serialize()),
            "point3": Document::from(self.point3.serialize()),
            "style": Document::from(self.style.serialize()),
        }
    }
}

impl Deserialize<Document> for Triangle {
    fn deserialize(document: Document) -> Self
    where
        Self: Sized,
    {
        let mut triangle = Triangle {
            point1: Point::default(),
            point2: Point::default(),
            point3: Point::default(),
            style: Style::default(),
        };

        if let Some(Bson::Document(point1)) = document.get("point1") {
            triangle.point1 = Point::deserialize(point1.clone());
        }

        if let Some(Bson::Document(point2)) = document.get("point2") {
            triangle.point2 = Point::deserialize(point2.clone());
        }

        if let Some(Bson::Document(point3)) = document.get("point3") {
            triangle.point3 = Point::deserialize(point3.clone());
        }

        if let Some(Bson::Document(style)) = document.get("style") {
            triangle.style = Style::deserialize(style.clone());
        }

        triangle
    }
}

impl Serialize<Group> for Triangle {
    fn serialize(&self) -> Group {
        let data = Data::new()
            .move_to((self.point1.x, self.point1.y))
            .line_to((self.point2.x, self.point2.y))
            .line_to((self.point3.x, self.point3.y))
            .close();

        let path = svg::node::element::Path::new()
            .set("stroke-width", self.style.get_stroke_width())
            .set("stroke", self.style.get_stroke_color())
            .set("stroke-opacity", self.style.get_stroke_alpha())
            .set("fill", self.style.get_fill())
            .set("fill-opacity", self.style.get_fill_alpha())
            .set("style", "mix-blend-mode:hard-light")
            .set("d", data);

        Group::new().set("class", self.id()).add(path)
    }
}

impl Serialize<Object> for Triangle {
    fn serialize(&self) -> Object {
        let mut data = Object::new();

        data.insert("point1", JsonValue::Object(self.point1.serialize()));
        data.insert("point2", JsonValue::Object(self.point2.serialize()));
        data.insert("point3", JsonValue::Object(self.point3.serialize()));
        data.insert("style", JsonValue::Object(self.style.serialize()));

        data
    }
}

impl Deserialize<Object> for Triangle {
    fn deserialize(document: Object) -> Self
    where
        Self: Sized,
    {
        let mut triangle = Triangle {
            point1: Point::default(),
            point2: Point::default(),
            point3: Point::default(),
            style: Style::default(),
        };

        if let Some(JsonValue::Object(point1)) = document.get("point1") {
            triangle.point1 = Point::deserialize(point1.clone());
        }
        if let Some(JsonValue::Object(point2)) = document.get("point2") {
            triangle.point2 = Point::deserialize(point2.clone());
        }
        if let Some(JsonValue::Object(point3)) = document.get("point3") {
            triangle.point3 = Point::deserialize(point3.clone());
        }
        if let Some(JsonValue::Object(style)) = document.get("style") {
            triangle.style = Style::deserialize(style.clone());
        }

        triangle
    }
}

impl Tool for Triangle {
    fn add_to_frame(&self, frame: &mut Frame) {
        let triangle = Path::new(|builder| {
            builder.move_to(self.point1);
            builder.line_to(self.point2);
            builder.line_to(self.point3);
            builder.close();
        });

        if let Some((width, color, _, _)) = self.style.stroke {
            frame.stroke(
                &triangle,
                Stroke::default().with_width(width).with_color(color),
            );
        }
        if let Some((color, _)) = self.style.fill {
            frame.fill(&triangle, Fill::from(color));
        }
    }

    fn boxed_clone(&self) -> Box<dyn Tool> {
        Box::new((*self).clone())
    }

    fn id(&self) -> String {
        "Triangle".into()
    }
}

impl Into<Box<dyn Tool>> for Box<Triangle> {
    fn into(self) -> Box<dyn Tool> {
        self.boxed_clone()
    }
}
