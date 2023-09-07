use std::fmt::{Debug};
use std::marker::PhantomData;
use std::ops::{Add, Sub};
use iced::{mouse, Point, Rectangle, Renderer, keyboard, Vector};
use iced::event::Status;
use iced::mouse::Cursor;
use iced::widget::canvas::{Event, Frame, Geometry};

use crate::tool::{Pending, Tool};

#[derive(Clone)]
pub enum BrushPending<BrushType>
where BrushType: Send+Sync+Clone+Brush {
    None,
    Stroking (Point, Point, Vec<Vector>),
    _PhantomVariant (PhantomData<BrushType>),
}

impl<BrushType: Send+Sync+Clone+Brush+'static> Pending for BrushPending<BrushType>
where Box<BrushType>: Into<Box<dyn Tool>> {
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
                            BrushPending::None => {
                                *self = BrushPending::Stroking(
                                    cursor,
                                    cursor,
                                    vec![Vector::new(0.0, 0.0)],
                                );

                                None
                            }
                            _ => None
                        }
                    }
                    mouse::Event::CursorMoved {..} => {
                        match self {
                            BrushPending::Stroking(start, last, offsets) => {
                                let mut new_offsets = offsets.clone();
                                new_offsets.push(cursor.sub(*last));

                                *self = BrushPending::Stroking(
                                    *start,
                                    cursor,
                                    new_offsets,
                                );

                                None
                            }
                            _ => None
                        }
                    }
                    mouse::Event::ButtonReleased(mouse::Button::Left) => {
                        match self {
                            BrushPending::Stroking(start, _last, offsets) => {
                                let start_clone = start.clone();
                                let offsets_clone = offsets.clone();

                                *self = BrushPending::None;

                                Some(Box::new(BrushType::new(start_clone, offsets_clone)).into())
                            }
                            _ => None
                        }
                    }
                    _ => None
                };

                (Status::Captured, message)
            }
            Event::Keyboard(key_event) => {
                match key_event {
                    keyboard::Event::KeyPressed { key_code: keyboard::KeyCode::S, .. } => {
                        *self = BrushPending::None;

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

        if let Some(_cursor_position) = cursor.position_in(bounds) {
            match self {
                BrushPending::Stroking(start, _last, offsets) => {
                    let mut pos = *start;

                    for offset in offsets.clone() {
                        BrushType::add_stroke_piece(pos, pos.add(offset), &mut frame);
                        pos = pos.add(offset.clone());
                    }
                }
                _ => {}
            }
        };

        frame.into_geometry()
    }

    fn id(&self) -> String {
        BrushType::id()
    }

    fn default() -> Self where Self: Sized {
        BrushPending::None
    }

    fn boxed_clone(&self) -> Box<dyn Pending> {
        Box::new((*self).clone())
    }
}

pub trait Brush: Send+Sync+Debug {
    fn new(start: Point, offsets: Vec<Vector>) -> Self where Self:Sized;
    fn id() -> String where Self:Sized;

    fn get_start(&self) -> Point;
    fn get_offsets(&self) -> Vec<Vector>;

    fn add_stroke_piece(point1: Point, point2: Point, frame: &mut Frame) where Self:Sized;
}

impl<BrushType> Tool for BrushType
where BrushType: Brush+Clone+'static {
    fn add_to_frame(&self, frame: &mut Frame) {
        let mut pos = self.get_start();

        for offset in self.get_offsets() {
            BrushType::add_stroke_piece(pos, pos.add(offset), frame);
            pos = pos.add(offset.clone());
        }
    }

    fn boxed_clone(&self) -> Box<dyn Tool> {
        Box::new((*self).clone())
    }
}