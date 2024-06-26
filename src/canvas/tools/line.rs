use crate::canvas::layer::CanvasMessage;
use crate::canvas::style::Style;
use crate::utils::serde::{Deserialize, Serialize};
use iced::event::Status;
use iced::mouse::Cursor;
use iced::widget::canvas::{Event, Frame, Geometry, Path, Stroke};
use iced::{mouse, Color, Point, Rectangle, Renderer};
use json::object::Object;
use json::JsonValue;
use mongodb::bson::{doc, Bson, Document};
use std::fmt::Debug;
use std::sync::Arc;
use svg::node::element::{self, path::Data, Group};

use crate::canvas::tool::{Pending, Tool};

#[derive(Clone)]
pub enum LinePending {
    None,
    One(Point),
}

impl Pending for LinePending {
    fn update(
        &mut self,
        event: Event,
        cursor: Point,
        style: Style,
    ) -> (Status, Option<CanvasMessage>) {
        match event {
            Event::Mouse(mouse_event) => {
                let message = match mouse_event {
                    mouse::Event::ButtonPressed(mouse::Button::Left) => match self {
                        LinePending::None => {
                            *self = LinePending::One(cursor);
                            None
                        }
                        LinePending::One(start) => {
                            let start_clone = start.clone();

                            *self = LinePending::None;
                            Some(
                                CanvasMessage::UseTool(Arc::new(Line {
                                    start: start_clone,
                                    end: cursor,
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
                LinePending::None => {}
                LinePending::One(start) => {
                    let stroke = Path::new(|p| {
                        p.move_to(*start);
                        p.line_to(cursor_position);
                    });

                    if let Some((width, color, _, _)) = style.stroke {
                        frame.stroke(
                            &stroke,
                            Stroke::default().with_width(width).with_color(color),
                        );
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

        style.fill = None;
    }

    fn id(&self) -> String {
        String::from("Line")
    }

    fn default() -> Self
    where
        Self: Sized,
    {
        LinePending::None
    }

    fn dyn_default(&self) -> Box<dyn Pending> {
        Box::new(LinePending::None)
    }

    fn boxed_clone(&self) -> Box<dyn Pending> {
        Box::new((*self).clone())
    }
}

#[derive(Debug, Clone)]
pub struct Line {
    start: Point,
    end: Point,
    style: Style,
}

impl Serialize<Document> for Line {
    fn serialize(&self) -> Document {
        doc! {
            "start": Document::from(self.start.serialize()),
            "end": Document::from(self.end.serialize()),
            "style": Document::from(self.style.serialize()),
        }
    }
}

impl Deserialize<Document> for Line {
    fn deserialize(document: &Document) -> Self
    where
        Self: Sized,
    {
        let mut line = Line {
            start: Point::default(),
            end: Point::default(),
            style: Style::default(),
        };

        if let Some(Bson::Document(start)) = document.get("start") {
            line.start = Point::deserialize(start);
        }

        if let Some(Bson::Document(end)) = document.get("end") {
            line.end = Point::deserialize(end);
        }

        if let Some(Bson::Document(style)) = document.get("style") {
            line.style = Style::deserialize(style);
        }

        line
    }
}

impl Serialize<Group> for Line {
    fn serialize(&self) -> Group {
        let data = Data::new()
            .move_to((self.start.x, self.start.y))
            .line_to((self.end.x, self.end.y))
            .close();

        let path = element::Path::new()
            .set("stroke-width", self.style.get_stroke_width())
            .set("stroke", self.style.get_stroke_color())
            .set("stroke-opacity", self.style.get_stroke_alpha())
            .set("d", data);

        Group::new().set("class", self.id()).add(path)
    }
}

impl Serialize<Object> for Line {
    fn serialize(&self) -> Object {
        let mut data = Object::new();

        data.insert("start", JsonValue::Object(self.start.serialize()));
        data.insert("end", JsonValue::Object(self.end.serialize()));
        data.insert("style", JsonValue::Object(self.style.serialize()));

        data
    }
}

impl Deserialize<Object> for Line {
    fn deserialize(document: &Object) -> Self
    where
        Self: Sized,
    {
        let mut line = Line {
            start: Point::default(),
            end: Point::default(),
            style: Style::default(),
        };

        if let Some(JsonValue::Object(start)) = document.get("start") {
            line.start = Point::deserialize(start);
        }
        if let Some(JsonValue::Object(end)) = document.get("end") {
            line.end = Point::deserialize(end);
        }
        if let Some(JsonValue::Object(style)) = document.get("style") {
            line.style = Style::deserialize(style);
        }

        line
    }
}

impl Tool for Line {
    fn add_to_frame(&self, frame: &mut Frame) {
        let line = Path::new(|builder| {
            builder.move_to(self.start);
            builder.line_to(self.end);
        });

        if let Some((width, color, _, _)) = self.style.stroke {
            frame.stroke(&line, Stroke::default().with_width(width).with_color(color));
        }
    }

    fn boxed_clone(&self) -> Box<dyn Tool> {
        Box::new((*self).clone())
    }

    fn id(&self) -> String {
        "Line".into()
    }
}

impl Into<Box<dyn Tool>> for Box<Line> {
    fn into(self) -> Box<dyn Tool> {
        self.boxed_clone()
    }
}
