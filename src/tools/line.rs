use std::fmt::{Debug};
use iced::{mouse, Point, Rectangle, Renderer};
use iced::event::Status;
use iced::mouse::Cursor;
use iced::widget::canvas::{Event, Frame, Geometry, Path, Stroke};
use mongodb::bson::{Bson, doc, Document};
use crate::serde::{Deserialize, Serialize};
use crate::theme::Theme;

use crate::tool::{Pending, Tool};

#[derive(Clone)]
pub enum LinePending {
    None,
    One(Point),
}

impl Pending for LinePending {
    fn update(
        &mut self,
        event: Event,
        cursor: Point
    ) -> (Status, Option<Box<dyn Tool>>) {
        match event {
            Event::Mouse(mouse_event) => {
                let message = match mouse_event {
                    mouse::Event::ButtonPressed(mouse::Button::Left) => {
                        match self {
                            LinePending::None => {
                                *self = LinePending::One(cursor);
                                None
                            }
                            LinePending::One(start) => {
                                let start_clone = start.clone();

                                *self = LinePending::None;
                                Some(Box::new(Line{start:start_clone, end:cursor}).into())
                            }
                        }
                    }
                    _ => None
                };

                (Status::Captured, message)
            }
            _ => (Status::Ignored, None)
        }
    }

    fn draw(
        &self,
        renderer: &Renderer<Theme>,
        bounds: Rectangle,
        cursor: Cursor
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

                    frame.stroke(&stroke, Stroke::default().with_width(2.0));
                }
            }
        };

        frame.into_geometry()
    }

    fn id(&self) -> String {
        String::from("Line")
    }

    fn default() -> Self where Self: Sized {
        LinePending::None
    }

    fn boxed_clone(&self) -> Box<dyn Pending> {
        Box::new((*self).clone())
    }
}

#[derive(Debug, Clone)]
pub struct Line {
    start: Point,
    end: Point,
}

impl Serialize for Line {
    fn serialize(&self) -> Document {
        doc! {
            "start": self.start.serialize(),
            "end": self.end.serialize(),
        }
    }
}

impl Deserialize for Line {
    fn deserialize(document: Document) -> Self where Self: Sized {
        let mut line = Line {start: Point::default(), end: Point::default()};

        if let Some(Bson::Document(start)) = document.get("start") {
            line.start = Point::deserialize(start.clone());
        }

        if let Some(Bson::Document(end)) = document.get("end") {
            line.end = Point::deserialize(end.clone());
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

        frame.stroke(&line, Stroke::default().with_width(2.0));
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