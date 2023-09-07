use std::fmt::{Debug};
use std::ops::{Add, Sub};
use iced::{Point, Vector};
use iced::widget::canvas::{Fill, Frame, Path};
use crate::tool::Tool;

use crate::tools::brush::Brush;

#[derive(Debug, Clone)]
pub struct Pen {
    start: Point,
    offsets: Vec<Vector>,
}


impl Brush for Pen {
    fn new(start: Point, offsets: Vec<Vector>) -> Self where Self: Sized {
        Pen { start, offsets }
    }

    fn id() -> String {
        String::from("FountainPen")
    }

    fn get_start(&self) -> Point {
        self.start
    }

    fn get_offsets(&self) -> Vec<Vector> {
        self.offsets.clone()
    }

    fn add_stroke_piece(point1: Point, point2: Point, frame: &mut Frame) where Self: Sized {
        let quad = Path::new(|builder| {
            let offset = Vector::new((45_f32).cos() * 3.0, (45_f32).sin() * 3.0);

            builder.move_to(point1.add(offset));
            builder.line_to(point2.add(offset.clone()));
            builder.line_to(point2.sub(offset.clone()));
            builder.line_to(point1.sub(offset.clone()));
            builder.close()
        });

        frame.fill(&quad, Fill::default());
    }

    fn add_end(_point: Point, _frame: &mut Frame) where Self: Sized { }
}

impl Into<Box<dyn Tool>> for Box<Pen> {
    fn into(self) -> Box<dyn Tool> {
        self.boxed_clone()
    }
}