use crate::canvas::layer::CanvasMessage;
use crate::canvas::style::Style;
use crate::serde::{Deserialize, Serialize};
use iced::event::Status;
use iced::mouse::Cursor;
use iced::widget::canvas::path::arc::Elliptical;
use iced::widget::canvas::{Event, Fill, Frame, Geometry, Path, Stroke};
use iced::{mouse, Color, Point, Rectangle, Renderer, Vector};
use json::object::Object;
use json::JsonValue;
use mongodb::bson::{doc, Bson, Document};
use std::fmt::Debug;
use std::ops::{Add, Sub};
use std::sync::Arc;
use svg::node::element::path::Data;
use svg::node::element::Group;

use crate::canvas::tool::{Pending, Tool};

#[derive(Clone)]
pub enum EllipsePending {
    None,
    One(Point),
    Two(Point, Point),
}

impl EllipsePending {
    fn convert_data(center: Point, point1: Point, point2: Point) -> (Point, Vector, f32) {
        let point2h: Point = if (point1.x - center.x).abs() < 1e-3 {
            Point::new(center.x, point2.y)
        } else {
            let slope1 = (center.y - point1.y) / (center.x - point1.x);
            let slope2 = -1.0 / slope1;

            let x: f32 =
                (point2.y - center.y + slope2 * center.x - slope1 * point2.x) / (slope2 - slope1);
            let y: f32 = slope2 * (x - center.x) + center.y;

            Point::new(x, y)
        };

        let radii: Vector = Vector::new(point1.distance(center), center.distance(point2h));
        let rotation = (point1.y - center.y).atan2(point1.x - center.x);

        (center, radii, rotation)
    }
}

impl Pending for EllipsePending {
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

                            let (center, radii, rotation) =
                                EllipsePending::convert_data(center_clone, point1_clone, cursor);
                            Some(
                                CanvasMessage::UseTool(Arc::new(Ellipse {
                                    center,
                                    radii,
                                    rotation,
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
                EllipsePending::None => {}
                EllipsePending::One(center) => {
                    let stroke = Path::new(|p| {
                        p.move_to((*center).sub(cursor_position.sub(*center)));
                        p.line_to(cursor_position);
                    });

                    if let Some((width, color, _, _)) = style.stroke {
                        frame.stroke(
                            &stroke,
                            Stroke::default().with_width(width).with_color(color),
                        );
                    }
                }
                EllipsePending::Two(center, point) => {
                    let stroke = Path::new(|p| {
                        let (center, radii, rotation) =
                            EllipsePending::convert_data(*center, *point, cursor_position);

                        if radii.y.abs() < 1e-3 {
                            p.move_to(center.sub((*point).sub(center)));
                            p.line_to(*point);
                        } else {
                            p.ellipse(Elliptical {
                                center,
                                radii,
                                rotation: rotation.into(),
                                start_angle: 0.0.into(),
                                end_angle: 360.0.into(),
                            });
                        }
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
        String::from("Ellipse")
    }

    fn default() -> Self
    where
        Self: Sized,
    {
        EllipsePending::None
    }

    fn dyn_default(&self) -> Box<dyn Pending> {
        Box::new(EllipsePending::None)
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
    style: Style,
}

impl Serialize<Document> for Ellipse {
    fn serialize(&self) -> Document {
        doc! {
            "center": Document::from(self.center.serialize()),
            "radii": Document::from(self.radii.serialize()),
            "rotation": self.rotation,
            "style": Document::from(self.style.serialize()),
        }
    }
}

impl Deserialize<Document> for Ellipse {
    fn deserialize(document: &Document) -> Self
    where
        Self: Sized,
    {
        let mut ellipse = Ellipse {
            center: Point::default(),
            radii: Vector::default(),
            rotation: 0.0,
            style: Style::default(),
        };

        if let Some(Bson::Document(center)) = document.get("center") {
            ellipse.center = Point::deserialize(center);
        }

        if let Some(Bson::Document(radii)) = document.get("radii") {
            ellipse.radii = Vector::deserialize(radii);
        }

        if let Some(Bson::Double(rotation)) = document.get("rotation") {
            ellipse.rotation = *rotation as f32;
        }

        if let Some(Bson::Document(style)) = document.get("style") {
            ellipse.style = Style::deserialize(style);
        }

        ellipse
    }
}

impl Serialize<Group> for Ellipse {
    fn serialize(&self) -> Group {
        let start = Point::new(
            self.center.x + self.radii.x * self.rotation.cos(),
            self.center.y + self.radii.x * self.rotation.sin(),
        );

        let end = Point::new(
            self.center.x - self.radii.x * self.rotation.cos(),
            self.center.y - self.radii.x * self.rotation.sin(),
        );

        let data = Data::new()
            .move_to((start.x, start.y))
            .elliptical_arc_to((
                self.radii.x,
                self.radii.y,
                self.rotation.to_degrees(),
                0,
                0,
                end.x,
                end.y,
            ))
            .elliptical_arc_to((
                self.radii.x,
                self.radii.y,
                self.rotation.to_degrees(),
                0,
                0,
                start.x,
                start.y,
            ));

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

impl Serialize<Object> for Ellipse {
    fn serialize(&self) -> Object {
        let mut data = Object::new();

        data.insert("center", JsonValue::Object(self.center.serialize()));
        data.insert("radii", JsonValue::Object(self.radii.serialize()));
        data.insert("rotation", JsonValue::Number(self.rotation.into()));
        data.insert("style", JsonValue::Object(self.style.serialize()));

        data
    }
}

impl Deserialize<Object> for Ellipse {
    fn deserialize(document: &Object) -> Self
    where
        Self: Sized,
    {
        let mut ellipse = Ellipse {
            center: Point::default(),
            radii: Vector::default(),
            rotation: 0.0,
            style: Style::default(),
        };

        if let Some(JsonValue::Object(center)) = document.get("center") {
            ellipse.center = Point::deserialize(center);
        }
        if let Some(JsonValue::Object(radii)) = document.get("radii") {
            ellipse.radii = Vector::deserialize(radii);
        }
        if let Some(JsonValue::Number(rotation)) = document.get("rotation") {
            ellipse.rotation = f32::from(*rotation);
        }
        if let Some(JsonValue::Object(style)) = document.get("style") {
            ellipse.style = Style::deserialize(style);
        }

        ellipse
    }
}

impl Tool for Ellipse {
    fn add_to_frame(&self, frame: &mut Frame) {
        let ellipse = Path::new(|builder| {
            if self.radii.y.abs() < 1e-3 {
                let vector = Vector::new(
                    self.clone().radii.x * self.clone().rotation.cos(),
                    self.clone().radii.x * self.clone().rotation.sin(),
                );
                builder.move_to(self.center.sub(vector));
                builder.line_to(self.center.add(vector.clone()));
            } else {
                builder.ellipse(Elliptical {
                    center: self.center,
                    radii: self.radii.clone(),
                    rotation: self.rotation.clone().into(),
                    start_angle: 0.0.into(),
                    end_angle: 360.0.into(),
                });
            }
        });

        if let Some((width, color, _, _)) = self.style.stroke {
            frame.stroke(
                &ellipse,
                Stroke::default().with_width(width).with_color(color),
            );
        }
        if let Some((color, _)) = self.style.fill {
            frame.fill(&ellipse, Fill::from(color));
        }
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
