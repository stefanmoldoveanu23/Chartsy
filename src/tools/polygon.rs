use std::fmt::{Debug};
use std::ops::{Add, Sub};
use iced::{mouse, Point, Rectangle, Renderer, keyboard, Vector, Color};
use iced::event::Status;
use iced::mouse::Cursor;
use iced::widget::canvas::{Event, Fill, Frame, Geometry, Path, Stroke, Style};

use crate::tool::{Pending, Tool};

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
    ) -> (Status, Option<Box<dyn Tool>>) {
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
                                        Some(Box::new(Polygon { first: first_clone, offsets: offsets_clone }).into())
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
            Event::Keyboard(key_event) => {
                match key_event {
                    keyboard::Event::KeyPressed { key_code: keyboard::KeyCode::S, .. } => {
                        *self = PolygonPending::None;

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
                PolygonPending::None => {}
                PolygonPending::Drawing(first, _last, offsets) => {
                    let snap = Path::new(|p| {
                        p.circle(*first, RADIUS);
                    });

                    let mut cyan_fill = Fill::default();
                    cyan_fill.style = Style::Solid(Color::from_rgba8(0, 255, 255, 0.3));
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

                    frame.stroke(&stroke, Stroke::default().with_width(2.0));
                }
            }
        };

        frame.into_geometry()
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

        frame.stroke(&polygon, Stroke::default().with_width(2.0));
    }

    fn boxed_clone(&self) -> Box<dyn Tool> {
        Box::new((*self).clone())
    }
}

impl Into<Box<dyn Tool>> for Box<Polygon> {
    fn into(self) -> Box<dyn Tool> {
        self.boxed_clone()
    }
}