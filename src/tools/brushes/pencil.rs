use std::fmt::{Debug};
use iced::{Point, Vector};
use iced::widget::canvas::{Frame, Path, Stroke};
use crate::tool::Tool;

use crate::tools::brush::Brush;

#[derive(Debug, Clone)]
pub struct Pencil {
    start: Point,
    offsets: Vec<Vector>,
}


impl Brush for Pencil {
    fn new(start: Point, offsets: Vec<Vector>) -> Self where Self: Sized {
        Pencil {start, offsets}
    }

    fn id() -> String where Self: Sized {
        String::from("Pencil")
    }

    fn get_start(&self) -> Point {
        self.start
    }

    fn get_offsets(&self) -> Vec<Vector> {
        self.offsets.clone()
    }

    fn add_stroke_piece(point1: Point, point2: Point, frame: &mut Frame) where Self: Sized {
        let line = Path::new(|builder| {
            builder.move_to(point1);
            builder.line_to(point2);
        });

        frame.stroke(&line, Stroke::default().with_width(2.0));
    }

    fn add_end(_point: Point, _frame: &mut Frame) where Self: Sized { }
}

impl Into<Box<dyn Tool>> for Box<Pencil> {
    fn into(self) -> Box<dyn Tool> {
        self.boxed_clone()
    }
}