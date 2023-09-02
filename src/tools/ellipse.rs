use std::fmt::{Debug};
use std::ops::{Add, Sub};
use iced::{mouse, Point, Rectangle, Renderer, keyboard, Vector};
use iced::event::Status;
use iced::mouse::Cursor;
use iced::widget::canvas::{Event, Frame, Geometry, Path, Stroke};
use iced::widget::canvas::path::arc::Elliptical;
use iced::widget::canvas::path::Builder;

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
                Point::new(center.clone().x, point2.y)
            } else {
                let slope1 = (center.y - point1.y) / (center.clone().x - point1.clone().x);
                let slope2 = -1.0 / slope1;

                let x :f32= (point2.y - center.clone().y + slope2 * center.clone().x - slope1.clone() * point2.x) / (slope2.clone() - slope1.clone());
                let y :f32= slope2.clone() * (x - center.clone().x) + center.clone().y;

                Point::new(x.clone(), y)
            };

        let radii :Vector= Vector::new(point1.distance(center.clone()), center.distance(point2h));
        let rotation = (point1.clone().y - center.clone().y).atan2(point1.clone().x - center.clone().x);

        (center.clone(), radii, rotation)
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
                                let center_clone = center.clone();
                                let point1_clone = point1.clone();

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
        renderer: &Renderer,
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

impl Tool for Ellipse {
    fn add_to_path(&self, builder: &mut Builder) {
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
    }

    fn boxed_clone(&self) -> Box<dyn Tool> {
        Box::new((*self).clone())
    }
}

impl Into<Box<dyn Tool>> for Box<Ellipse> {
    fn into(self) -> Box<dyn Tool> {
        self.boxed_clone()
    }
}