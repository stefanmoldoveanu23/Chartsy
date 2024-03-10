<<<<<<< Updated upstream
use std::fmt::{Debug};
use std::ops::{Add, Sub};
use std::sync::Arc;
use iced::{mouse, Point, Rectangle, Renderer, keyboard, Vector, Color};
use iced::advanced::graphics::core::SmolStr;
use iced::event::Status;
use iced::mouse::Cursor;
use iced::widget::canvas::{Event, Fill, Frame, Geometry, Path, Stroke};
use mongodb::bson::{Bson, doc, Document};
use crate::canvas::layer::CanvasAction;
use crate::canvas::style::Style;
use crate::serde::{Deserialize, Serialize};
=======
use crate::canvas::layer::CanvasAction;
use crate::canvas::style::Style;
use crate::serde::{Deserialize, Serialize};
use iced::event::Status;
use iced::mouse::Cursor;
use iced::widget::canvas::{Event, Fill, Frame, Geometry, Path, Stroke};
use iced::{keyboard, mouse, Color, Point, Rectangle, Renderer, Vector};
use json::object::Object;
use json::JsonValue;
use mongodb::bson::{doc, Bson, Document};
use std::fmt::Debug;
use std::ops::{Add, Sub};
use std::sync::Arc;
use std::str::FromStr;
use iced_style::core::SmolStr;
use svg::node::element::Group;
>>>>>>> Stashed changes

use crate::canvas::tool::{Pending, Tool};

const RADIUS: f32 = 10.0;

#[derive(Clone)]
pub enum PolygonPending {
    None,
    Drawing(Point, Point, Vec<Vector>),
}

impl Pending for PolygonPending {
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
                            PolygonPending::None => {
                                *self = PolygonPending::Drawing(cursor, cursor, vec![Vector::new(0.0, 0.0)]);
                                None
                            }
                            PolygonPending::Drawing(first, last, offsets) => {
                                if cursor.distance(*last) == 0.0 {
                                    None
                                } else {
                                    let first_clone = first.clone();
                                    let last_clone = last.clone();
                                    let mut offsets_clone = offsets.clone();

                                    if cursor.distance(first_clone) < RADIUS {
                                        offsets_clone.push(first_clone.sub(last_clone));
                                        *self = PolygonPending::None;
                                        Some(CanvasAction::UseTool(Arc::new(Polygon { first: first_clone, offsets: offsets_clone, style })).into())
                                    } else {
                                        offsets_clone.push(cursor.sub(last_clone));
                                        *self = PolygonPending::Drawing(first_clone, cursor, offsets_clone);
                                        None
                                    }
                                }
                            }
                        }
                    }
                    _ => None
                };

                (Status::Captured, message)
            }
<<<<<<< Updated upstream
            Event::Keyboard(key_event) => {
                match key_event {
                    keyboard::Event::KeyPressed { key: keyboard::Key::Character(str), .. } => {
                        if str == key_s {
                            *self = PolygonPending::None;

                            (Status::Captured, None)
                        } else {
                            (Status::Ignored, None)
                        }
                    }
                    _ => (Status::Ignored, None)
=======
            Event::Keyboard(key_event) => match key_event {
                keyboard::Event::KeyPressed {
                    key: keyboard::Key::Character(char),
                    ..
                } => {
                    if char == SmolStr::from_str("S").unwrap() {
                        *self = PolygonPending::None;

                        (Status::Captured, None)
                    } else {
                        (Status::Ignored, None)
                    }
>>>>>>> Stashed changes
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
                PolygonPending::None => {}
                PolygonPending::Drawing(first, _last, offsets) => {
                    let snap = Path::new(|p| {
                        p.circle(*first, RADIUS);
                    });

                    let cyan_fill = Fill::from(Color::from_rgba8(0, 255, 255, 0.3));
                    frame.fill(&snap, cyan_fill);

                    let stroke = Path::new(|p| {
                        p.move_to(*first);

                        let mut pos : Point = *first;
                        for offset in offsets {
                            pos = pos.add(offset.clone());
                            p.line_to(pos);
                        }

                        if cursor_position.distance(*first) >= RADIUS {
                            p.line_to(cursor_position);
                        }
                        p.line_to(*first);
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
        String::from("Polygon")
    }

    fn default() -> Self where Self: Sized {
        PolygonPending::None
    }

    fn boxed_clone(&self) -> Box<dyn Pending> {
        Box::new((*self).clone())
    }
}

#[derive(Debug, Clone)]
pub struct Polygon {
    first: Point,
    offsets: Vec<Vector>,
    style: Style,
}

impl Serialize for Polygon {
    fn serialize(&self) -> Document {
        doc! {
            "first": self.first.serialize(),
            "offsets": self.offsets.iter().map(|offset| {offset.serialize()}).collect::<Vec<Document>>().as_slice(),
            "style": self.style.serialize(),
        }
    }
}

impl Deserialize for Polygon {
    fn deserialize(document: Document) -> Self where Self: Sized {
        let mut polygon = Polygon {first: Point::default(), offsets: vec![], style: Style::default()};

        if let Some(Bson::Document(first)) = document.get("first") {
            polygon.first = Point::deserialize(first.clone());
        }

        if let Some(Bson::Array(offsets)) = document.get("offsets") {
            for offset in offsets {
                if let Bson::Document(offset) = offset {
                    polygon.offsets.push(Vector::deserialize(offset.clone()));
                }
            }
        }

        if let Some(Bson::Document(style)) = document.get("style") {
            polygon.style = Style::deserialize(style.clone());
        }

        polygon
    }
}

impl Tool for Polygon {
    fn add_to_frame(&self, frame: &mut Frame) {
        let polygon = Path::new(|builder| {
            builder.move_to(self.first);

            let mut pos = self.first;
            for offset in self.offsets.clone() {
                pos = pos.add(offset);
                builder.line_to(pos);
            }
        });

        if let Some((width, color, _, _)) = self.style.stroke {
            frame.stroke(&polygon, Stroke::default().with_width(width).with_color(color));
        }
        if let Some((color, _)) = self.style.fill {
            frame.fill(&polygon, Fill::from(color));
        }
    }

    fn boxed_clone(&self) -> Box<dyn Tool> {
        Box::new((*self).clone())
    }

    fn id(&self) -> String {
        "Polygon".into()
    }
}

impl Into<Box<dyn Tool>> for Box<Polygon> {
    fn into(self) -> Box<dyn Tool> {
        self.boxed_clone()
    }
}