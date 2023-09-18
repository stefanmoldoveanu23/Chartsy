use std::fmt::{Debug};
use std::ops::{Add, Sub};
use iced::{mouse, Point, Rectangle, Renderer, keyboard, Vector};
use iced::event::Status;
use iced::mouse::Cursor;
use iced::widget::canvas::{Event, Frame, Geometry, Path, Stroke};
use iced::widget::canvas::path::arc::Elliptical;
use mongodb::bson::{Bson, doc, Document};
use crate::serde::{Deserialize, Serialize};
use crate::theme::Theme;

use crate::tool::{Pending, Tool};

#[derive(Clone)]
pub enum EllipsePending {
    None,
    One(Point),
    Two(Point, Point),
}

impl EllipsePending {
    fn convert_data(center: Point, point1: Point, point2: Point) -> (Point, Vector, f32) {
        let point2h :Point=
            if (point1.x - center.x).abs() < 1e-3 {
                Point::new(center.x, point2.y)
            } else {
                let slope1 = (center.y - point1.y) / (center.x - point1.x);
                let slope2 = -1.0 / slope1;

                let x :f32= (point2.y - center.y + slope2 * center.x - slope1 * point2.x) / (slope2 - slope1);
                let y :f32= slope2 * (x - center.x) + center.y;

                Point::new(x, y)
            };

        let radii :Vector= Vector::new(point1.distance(center), center.distance(point2h));
        let rotation = (point1.y - center.y).atan2(point1.x - center.x);

        (center, radii, rotation)
    }
}

impl Pending for EllipsePending {
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
                            EllipsePending::None => {
                                *self = EllipsePending::One(cursor);
                                None
                            }
                            EllipsePending::One(start) => {
                                *self = EllipsePending::Two(*start, cursor);
                                None
                            }
                            EllipsePending::Two(center, point1) => {
                                let center_clone = *center;
                                let point1_clone = *point1;

                                *self = EllipsePending::None;

                                let (center, radii, rotation) = EllipsePending::convert_data(center_clone, point1_clone, cursor);
                                Some(Box::new(Ellipse { center, radii, rotation }).into())
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
                        *self = EllipsePending::None;

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
                EllipsePending::None => {}
                EllipsePending::One(center) => {
                    let stroke = Path::new(|p| {
                        p.move_to((*center).sub(cursor_position.sub(*center)));
                        p.line_to(cursor_position);
                    });

                    frame.stroke(&stroke, Stroke::default().with_width(2.0));
                }
                EllipsePending::Two(center, point) => {
                    let stroke = Path::new(|p| {
                        let (center, radii, rotation) = EllipsePending::convert_data(*center, *point, cursor_position);

                        if radii.y.abs() < 1e-3 {
                            p.move_to(center.sub((*point).sub(center)));
                            p.line_to(*point);
                        } else {
                            p.ellipse(Elliptical {
                                center,
                                radii,
                                rotation,
                                start_angle: 0.0,
                                end_angle: 360.0,
                            });
                        }
                    });

                    frame.stroke(&stroke, Stroke::default().with_width(2.0));
                }
            }
        };

        frame.into_geometry()
    }

    fn id(&self) -> String {
        String::from("Ellipse")
    }

    fn default() -> Self where Self: Sized {
        EllipsePending::None
    }

    fn boxed_clone(&self) -> Box<dyn Pending> {
        Box::new((*self).clone())
    }
}

#[derive(Debug, Clone)]
pub struct Ellipse {
    center: Point,
    radii: Vector,
    rotation: f32,
}

impl Serialize for Ellipse {
    fn serialize(&self) -> Document {
        doc! {
            "center": self.center.serialize(),
            "radii": self.radii.serialize(),
            "rotation": self.rotation,
        }
    }
}

impl Deserialize for Ellipse {
    fn deserialize(document: Document) -> Self where Self: Sized {
        let mut ellipse = Ellipse {center: Point::default(), radii: Vector::default(), rotation: 0.0};

        if let Some(Bson::Document(center)) = document.get("center") {
            ellipse.center = Point::deserialize(center.clone());
        }

        if let Some(Bson::Document(radii)) = document.get("radii") {
            ellipse.radii = Vector::deserialize(radii.clone());
        }

        if let Some(Bson::Double(rotation)) = document.get("rotation") {
            ellipse.rotation = rotation.clone() as f32;
        }

        ellipse
    }
}

impl Tool for Ellipse {
    fn add_to_frame(&self, frame: &mut Frame) {
        let ellipse = Path::new(|builder| {
            if self.radii.y.abs() < 1e-3 {
                let vector = Vector::new(self.clone().radii.x * self.clone().rotation.cos(), self.clone().radii.x * self.clone().rotation.sin());
                builder.move_to(self.center.sub(vector));
                builder.line_to(self.center.add(vector.clone()));
            } else {
                builder.ellipse(Elliptical{
                    center: self.center,
                    radii: self.radii.clone(),
                    rotation: self.rotation.clone(),
                    start_angle: 0.0,
                    end_angle: 360.0,
                });
            }
        });

        frame.stroke(&ellipse, Stroke::default().with_width(2.0));
    }

    fn boxed_clone(&self) -> Box<dyn Tool> {
        Box::new((*self).clone())
    }

    fn id(&self) -> String {
        "Ellipse".into()
    }
}

impl Into<Box<dyn Tool>> for Box<Ellipse> {
    fn into(self) -> Box<dyn Tool> {
        self.boxed_clone()
    }
}