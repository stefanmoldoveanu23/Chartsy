use crate::canvas::layer::CanvasAction;
use crate::canvas::style::Style;
use crate::serde::{Deserialize, Serialize};
use crate::theme::Theme;
use iced::event::Status;
use iced::mouse::Cursor;
use iced::widget::canvas::{Event, Fill, Frame, Geometry, Path, Stroke};
use iced::{keyboard, mouse, Color, Point, Rectangle, Renderer, Size};
use json::object::Object;
use json::JsonValue;
use mongodb::bson::{doc, Bson, Document};
use std::fmt::Debug;
use std::ops::Sub;
use std::sync::Arc;
use svg::node::element::Group;

use crate::canvas::tool::{Pending, Tool};

#[derive(Clone)]
pub enum RectPending {
    None,
    One(Point),
}

impl Pending for RectPending {
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
                        RectPending::None => {
                            *self = RectPending::One(cursor);
                            None
                        }
                        RectPending::One(start) => {
                            let start_clone = start.clone();

                            *self = RectPending::None;
                            Some(
                                CanvasAction::UseTool(Arc::new(Rect {
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
            Event::Keyboard(key_event) => match key_event {
                keyboard::Event::KeyPressed {
                    key_code: keyboard::KeyCode::S,
                    ..
                } => {
                    *self = RectPending::None;

                    (Status::Captured, None)
                }
                _ => (Status::Ignored, None),
            },
            _ => (Status::Ignored, None),
        }
    }

    fn draw(
        &self,
        renderer: &Renderer<Theme>,
        bounds: Rectangle,
        cursor: Cursor,
        style: Style,
    ) -> Geometry {
        let mut frame = Frame::new(renderer, bounds.size());

        if let Some(cursor_position) = cursor.position_in(bounds) {
            match self {
                RectPending::None => {}
                RectPending::One(start) => {
                    let stroke = Path::new(|p| {
                        p.rectangle(*start, Size::from(cursor_position.sub(*start)));
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
        String::from("Rectangle")
    }

    fn default() -> Self
    where
        Self: Sized,
    {
        RectPending::None
    }

    fn boxed_clone(&self) -> Box<dyn Pending> {
        Box::new((*self).clone())
    }
}

#[derive(Debug, Clone)]
pub struct Rect {
    start: Point,
    end: Point,
    style: Style,
}

impl Serialize<Document> for Rect {
    fn serialize(&self) -> Document {
        doc! {
            "start" : Document::from(self.start.serialize()),
            "end": Document::from(self.end.serialize()),
            "style": Document::from(self.style.serialize()),
        }
    }
}

impl Deserialize<Document> for Rect {
    fn deserialize(document: Document) -> Self
    where
        Self: Sized,
    {
        let mut rect = Rect {
            start: Point::default(),
            end: Point::default(),
            style: Style::default(),
        };

        if let Some(Bson::Document(start)) = document.get("start") {
            rect.start = Point::deserialize(start.clone());
        }

        if let Some(Bson::Document(end)) = document.get("end") {
            rect.end = Point::deserialize(end.clone());
        }

        if let Some(Bson::Document(style)) = document.get("style") {
            rect.style = Style::deserialize(style.clone());
        }

        rect
    }
}

impl Serialize<Group> for Rect {
    fn serialize(&self) -> Group {
        let rect = svg::node::element::Rectangle::new()
            .set("x", self.start.x.min(self.end.x))
            .set("y", self.start.y.min(self.end.y))
            .set("width", (self.start.x - self.end.x).abs())
            .set("height", (self.start.y - self.end.y).abs())
            .set("stroke-width", self.style.get_stroke_width())
            .set("stroke", self.style.get_stroke_color())
            .set("stroke-opacity", self.style.get_stroke_alpha())
            .set("fill", self.style.get_fill())
            .set("fill-opacity", self.style.get_fill_alpha())
            .set("style", "mix-blend-mode:hard-light");

        Group::new().set("class", self.id()).add(rect)
    }
}

impl Serialize<Object> for Rect {
    fn serialize(&self) -> Object {
        let mut data = Object::new();

        data.insert("start", JsonValue::Object(self.start.serialize()));
        data.insert("end", JsonValue::Object(self.end.serialize()));
        data.insert("style", JsonValue::Object(self.style.serialize()));

        data
    }
}

impl Deserialize<Object> for Rect {
    fn deserialize(document: Object) -> Self
    where
        Self: Sized,
    {
        let mut rect = Rect {
            start: Point::default(),
            end: Point::default(),
            style: Style::default(),
        };

        if let Some(JsonValue::Object(start)) = document.get("start") {
            rect.start = Point::deserialize(start.clone());
        }
        if let Some(JsonValue::Object(end)) = document.get("end") {
            rect.end = Point::deserialize(end.clone());
        }
        if let Some(JsonValue::Object(style)) = document.get("style") {
            rect.style = Style::deserialize(style.clone());
        }

        rect
    }
}

impl Tool for Rect {
    fn add_to_frame(&self, frame: &mut Frame) {
        let rect = Path::new(|builder| {
            builder.rectangle(self.start, Size::from(self.end.sub(self.start)));
        });

        if let Some((width, color, _, _)) = self.style.stroke {
            frame.stroke(&rect, Stroke::default().with_width(width).with_color(color));
        }
        if let Some((color, _)) = self.style.fill {
            frame.fill(&rect, Fill::from(color));
        }
    }

    fn boxed_clone(&self) -> Box<dyn Tool> {
        Box::new((*self).clone())
    }

    fn id(&self) -> String {
        "Rectangle".into()
    }
}

impl Into<Box<dyn Tool>> for Box<Rect> {
    fn into(self) -> Box<dyn Tool> {
        self.boxed_clone()
    }
}
