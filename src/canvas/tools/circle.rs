use std::fmt::{Debug};
use std::sync::Arc;
use iced::{mouse, Point, Rectangle, Renderer, keyboard, Color};
use iced::advanced::graphics::core::SmolStr;
use iced::event::Status;
use iced::mouse::Cursor;
use iced::widget::canvas::{Event, Fill, Frame, Geometry, Path, Stroke};
use mongodb::bson::{Bson, doc, Document};
use crate::canvas::layer::CanvasAction;
use crate::canvas::style::Style;
use crate::serde::{Deserialize, Serialize};

use crate::canvas::tool::{Pending, Tool};

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
        style: Style,
    ) -> (Status, Option<CanvasAction>) {
        let key_s :SmolStr= SmolStr::from("S");

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
                                Some(CanvasAction::UseTool(Arc::new(Circle { center: center_clone, radius: cursor.distance(center_clone), style: style.clone() })).into())
                            }
                        }
                    }
                    _ => None
                };

                (Status::Captured, message)
            }
            Event::Keyboard(key_event) => {
                match key_event {
                    keyboard::Event::KeyPressed { key: keyboard::Key::Character(str), .. } => {
                        if str == key_s {
                            *self = CirclePending::None;

                            (Status::Captured, None)
                        } else {
                            (Status::Ignored, None)
                        }
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
        style: Style,
    ) -> Geometry {
        let mut frame = Frame::new(renderer, bounds.size());

        if let Some(cursor_position) = cursor.position_in(bounds) {
            match self {
                CirclePending::None => {}
                CirclePending::One(center) => {
                    let stroke = Path::new(|p| {
                        p.circle(*center, cursor_position.distance(*center));
                    });

                    if let Some((width, color, _, _)) = style.stroke {
                        frame.stroke(&stroke, Stroke::default().with_width(width).with_color(color));
                    }
                    if let Some((color, _)) = style.fill {
                        frame.fill(
                            &stroke,
                            Fill::from(color)
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
        if style.fill.is_none() {
            style.fill = Some((Color::TRANSPARENT, false));
        }
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
    style: Style,
}

impl Serialize for Circle {
    fn serialize(&self) -> Document {
        doc! {
            "center": self.center.serialize(),
            "radius": self.radius,
            "style": self.style.serialize(),
        }
    }
}

impl Deserialize for Circle {
    fn deserialize(document: Document) -> Self where Self: Sized {
        let mut circle = Circle {center: Point::default(), radius: 0.0, style: Style::default() };

        if let Some(Bson::Document(center)) = document.get("center") {
            circle.center = Point::deserialize(center.clone());
        }

        if let Some(Bson::Double(radius)) = document.get("radius") {
            circle.radius = *radius as f32;
        }

        if let Some(Bson::Document(style)) = document.get("style") {
            circle.style = Style::deserialize(style.clone());
        }

        circle
    }
}

impl Tool for Circle {
    fn add_to_frame(&self, frame: &mut Frame) {
        let circle = Path::new(|builder| {
            builder.circle(self.center, self.radius.clone());
        });

        if let Some((width, color, _, _)) = self.style.stroke {
            frame.stroke(&circle, Stroke::default().with_width(width).with_color(color));
        }
        if let Some((color, _)) = self.style.fill {
            frame.fill(&circle, Fill::from(color));
        }
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