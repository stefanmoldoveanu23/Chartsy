use std::fmt::{Debug};
use iced::{mouse, Point, Rectangle, Renderer, keyboard};
use iced::event::Status;
use iced::mouse::Cursor;
use iced::widget::canvas::{Event, Frame, Geometry, Path, Stroke};

use crate::tool::{Pending, Tool};

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
    ) -> (Status, Option<Box<dyn Tool>>) {
        match event {
            Event::Mouse(mouse_event) => {
                let message = match mouse_event {
                    mouse::Event::ButtonPressed(mouse::Button::Left) => {
                        match self {
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
                                Some(Box::new(Triangle { point1: point1_clone, point2: point2_clone, point3: cursor }).into())
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
                        *self = TrianglePending::None;

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
                TrianglePending::None => {}
                TrianglePending::One(point1) => {
                    let stroke = Path::new(|p| {
                        p.move_to(*point1);
                        p.line_to(cursor_position);
                    });

                    frame.stroke(&stroke, Stroke::default().with_width(2.0));
                }
                TrianglePending::Two(point1, point2) => {
                    let stroke = Path::new(|p| {
                        p.move_to(*point1);
                        p.line_to(*point2);
                        p.line_to(cursor_position);
                        p.line_to(*point1);
                    });

                    frame.stroke(&stroke, Stroke::default().with_width(2.0));
                }
            }
        };

        frame.into_geometry()
    }

    fn id(&self) -> String {
        String::from("Triangle")
    }

    fn default() -> Self where Self: Sized {
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
}

impl Tool for Triangle {
    fn add_to_frame(&self, frame: &mut Frame) {
        let triangle = Path::new(|builder| {
            builder.move_to(self.point1);
            builder.line_to(self.point2);
            builder.line_to(self.point3);
            builder.close();
        });

        frame.stroke(&triangle, Stroke::default().with_width(2.0));
    }

    fn boxed_clone(&self) -> Box<dyn Tool> {
        Box::new((*self).clone())
    }
}

impl Into<Box<dyn Tool>> for Box<Triangle> {
    fn into(self) -> Box<dyn Tool> {
        self.boxed_clone()
    }
}