use std::fmt::{Debug};
use iced::{Point, Vector};
use iced::widget::canvas::{Frame, Path, Stroke};
use crate::canvas::style::Style;
use crate::canvas::tool::Tool;

use crate::canvas::tools::brush::Brush;

#[derive(Debug, Clone)]
pub struct Pencil {
    start: Point,
    offsets: Vec<Vector>,
    style: Style,
}


impl Brush for Pencil {
    fn new(start: Point, offsets: Vec<Vector>, style: Style) -> Self where Self: Sized {
        Pencil {start, offsets, style}
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

    fn get_style(&self) -> Style {
        self.style.clone()
    }

    fn add_stroke_piece(point1: Point, point2: Point, frame: &mut Frame, style: Style) where Self: Sized {
        let line = Path::new(|builder| {
            builder.move_to(point1);
            builder.line_to(point2);
        });

        if let Some((width, color, _, _)) = style.stroke {
            frame.stroke(&line, Stroke::default().with_width(width).with_color(color));
        }
    }

    fn add_end(_point: Point, _frame: &mut Frame, _style: Style) where Self: Sized { }
}

impl Into<Box<dyn Tool>> for Box<Pencil> {
    fn into(self) -> Box<dyn Tool> {
        self.boxed_clone()
    }
}