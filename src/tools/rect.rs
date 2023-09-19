use std::fmt::{Debug};
use std::ops::Sub;
use iced::{mouse, Point, Rectangle, Renderer, keyboard, Size};
use iced::event::Status;
use iced::mouse::Cursor;
use iced::widget::canvas::{Event, Frame, Geometry, Path, Stroke};
use mongodb::bson::{Bson, doc, Document};
use crate::canvas::canvas::CanvasAction;
use crate::serde::{Deserialize, Serialize};
use crate::theme::Theme;

use crate::tool::{Pending, Tool};

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
    ) -> (Status, Option<CanvasAction>) {
        match event {
            Event::Mouse(mouse_event) => {
                let message = match mouse_event {
                    mouse::Event::ButtonPressed(mouse::Button::Left) => {
                        match self {
                            RectPending::None => {
                                *self = RectPending::One(cursor);
                                None
                            }
                            RectPending::One(start) => {
                                let start_clone = start.clone();

                                *self = RectPending::None;
                                Some(CanvasAction::UseTool(Box::new(Rect { start: start_clone, end: cursor })).into())
                            }
                        }
                    }
                    _ => None
                };

                (Status::Captured, message)
            }
            Event::Keyboard(key_event) => {
                match key_event {
                    keyboard::Event::KeyPressed { key_code: keyboard::KeyCode::S, .. } => {
                        *self = RectPending::None;

                        (Status::Captured, None)
                    }
                    _ => (Status::Ignored, None)
                }
            }
            _ => (Status::Ignored, None)
        }
    }

    fn draw(
        &self,
        renderer: &Renderer<Theme>,
        bounds: Rectangle,
        cursor: Cursor,
    ) -> Geometry {
        let mut frame = Frame::new(renderer, bounds.size());

        if let Some(cursor_position) = cursor.position_in(bounds) {
            match self {
                RectPending::None => {}
                RectPending::One(start) => {
                    let stroke = Path::new(|p| {
                        p.rectangle(*start, Size::from(cursor_position.sub(*start)));
                    });

                    frame.stroke(&stroke, Stroke::default().with_width(2.0));
                }
            }
        };

        frame.into_geometry()
    }

    fn id(&self) -> String {
        String::from("Rectangle")
    }

    fn default() -> Self where Self: Sized {
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
}

impl Serialize for Rect {
    fn serialize(&self) -> Document {
        doc! {
            "start" : self.start.serialize(),
            "end": self.end.serialize(),
        }
    }
}

impl Deserialize for Rect {
    fn deserialize(document: Document) -> Self where Self: Sized {
        let mut rect = Rect {start: Point::default(), end: Point::default()};

        if let Some(Bson::Document(start)) = document.get("start") {
            rect.start = Point::deserialize(start.clone());
        }

        if let Some(Bson::Document(end)) = document.get("end") {
            rect.end = Point::deserialize(end.clone());
        }

        rect
    }
}

impl Tool for Rect {
    fn add_to_frame(&self, frame: &mut Frame) {
        let rect = Path::new(|builder| {
            builder.rectangle(self.start, Size::from(self.end.sub(self.start)));
        });

        frame.stroke(&rect, Stroke::default().with_width(2.0));
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