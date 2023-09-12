use std::fmt::{Debug};
use iced::{mouse, Point, Rectangle, Renderer, keyboard};
use iced::event::Status;
use iced::mouse::Cursor;
use iced::widget::canvas::{Event, Frame, Geometry, Path, Stroke};
use mongodb::bson::{Bson, doc, Document};
use crate::serde::{Deserialize, Serialize};

use crate::tool::{Pending, Tool};

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
    ) -> (Status, Option<Box<dyn Tool>>) {
        match event {
            Event::Mouse(mouse_event) => {
                let message = match mouse_event {
                    mouse::Event::ButtonPressed(mouse::Button::Left) => {
                        match self {
                            CirclePending::None => {
                                *self = CirclePending::One(cursor);
                                None
                            }
                            CirclePending::One(center) => {
                                let center_clone = center.clone();

                                *self = CirclePending::None;
                                Some(Box::new(Circle { center: center_clone, radius: cursor.distance(center_clone) }).into())
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
                        *self = CirclePending::None;

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
        renderer: &Renderer,
        bounds: Rectangle,
        cursor: Cursor,
    ) -> Geometry {
        let mut frame = Frame::new(renderer, bounds.size());

        if let Some(cursor_position) = cursor.position_in(bounds) {
            match self {
                CirclePending::None => {}
                CirclePending::One(center) => {
                    let stroke = Path::new(|p| {
                        p.circle(*center, cursor_position.distance(*center));
                    });

                    frame.stroke(&stroke, Stroke::default().with_width(2.0));
                }
            }
        };

        frame.into_geometry()
    }

    fn id(&self) -> String {
        String::from("Circle")
    }

    fn default() -> Self where Self: Sized {
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
}

impl Serialize for Circle {
    fn serialize(&self) -> Document {
        doc! {
            "center": self.center.serialize(),
            "radius": self.radius,
        }
    }
}

impl Deserialize for Circle {
    fn deserialize(document: Document) -> Self where Self: Sized {
        let mut circle = Circle {center: Point::default(), radius: 0.0};

        if let Some(Bson::Document(center)) = document.get("center") {
            circle.center = Point::deserialize(center.clone());
        }

        if let Some(Bson::Double(radius)) = document.get("radius") {
            circle.radius = radius.clone() as f32;
        }

        circle
    }
}

impl Tool for Circle {
    fn add_to_frame(&self, frame: &mut Frame) {
        let circle = Path::new(|builder| {
            builder.circle(self.center, self.radius.clone());
        });

        frame.stroke(&circle, Stroke::default().with_width(2.0));
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