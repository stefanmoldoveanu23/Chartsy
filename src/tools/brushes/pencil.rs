use std::fmt::{Debug};
use iced::{Point, Vector};
use iced::widget::canvas::path::Builder;
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

    fn get_start(&self) -> Point {
        self.start
    }

    fn get_offsets(&self) -> Vec<Vector> {
        self.offsets.clone()
    }

    fn add_stroke_piece(_point1: Point, point2: Point, builder: &mut Builder) where Self: Sized {
        builder.line_to(point2);
    }
}

impl Into<Box<dyn Tool>> for Box<Pencil> {
    fn into(self) -> Box<dyn Tool> {
        self.boxed_clone()
    }
}