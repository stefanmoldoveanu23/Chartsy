use std::fmt::{Debug};
use std::sync::Arc;
use iced::{mouse, Point, Rectangle, Renderer, keyboard, Color};
use iced::event::Status;
use iced::mouse::Cursor;
use iced::widget::canvas::{Event, Fill, Frame, Geometry, Path, Stroke};
use mongodb::bson::{Bson, doc, Document};
use crate::canvas::layer::CanvasAction;
use crate::canvas::style::Style;
use crate::serde::{Deserialize, Serialize};
use crate::theme::Theme;

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
                                Some(CanvasAction::UseTool(Arc::new(Triangle { point1: point1_clone, point2: point2_clone, point3: cursor, style })).into())
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
        renderer: &Renderer<Theme>,
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
                        frame.stroke(&stroke, Stroke::default().with_width(width).with_color(color));
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
                        frame.stroke(&stroke, Stroke::default().with_width(width).with_color(color));
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
    style: Style,
}

impl Serialize for Triangle {
    fn serialize(&self) -> Document {
        doc! {
            "point1": self.point1.serialize(),
            "point2": self.point2.serialize(),
            "point3": self.point3.serialize(),
            "style": self.style.serialize(),
        }
    }
}

impl Deserialize for Triangle {
    fn deserialize(document: Document) -> Self where Self: Sized {
        let mut triangle = Triangle {point1: Point::default(), point2: Point::default(), point3: Point::default(), style: Style::default()};

        if let Some(Bson::Document(point1)) = document.get("point1") {
            triangle.point1 = Point::deserialize(point1.clone());
        }

        if let Some(Bson::Document(point2)) = document.get("point1") {
            triangle.point2 = Point::deserialize(point2.clone());
        }

        if let Some(Bson::Document(point3)) = document.get("point1") {
            triangle.point3 = Point::deserialize(point3.clone());
        }

        if let Some(Bson::Document(style)) = document.get("style") {
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
            frame.stroke(&triangle, Stroke::default().with_width(width).with_color(color));
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