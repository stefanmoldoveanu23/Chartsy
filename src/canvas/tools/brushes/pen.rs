use std::fmt::{Debug};
use std::ops::{Add, Sub};
use iced::{Point, Vector};
use iced::widget::canvas::{Fill, Frame, Path};
use iced_runtime::core::Color;
use crate::canvas::style::Style;
use crate::canvas::tool::Tool;

use crate::canvas::tools::brush::Brush;

#[derive(Debug, Clone)]
pub struct Pen {
    start: Point,
    offsets: Vec<Vector>,
    style: Style,
}


impl Brush for Pen {
    fn new(start: Point, offsets: Vec<Vector>, style: Style) -> Self where Self: Sized {
        Pen { start, offsets, style }
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
    fn get_style(&self) -> Style {
        self.style.clone()
    }

    fn add_stroke_piece(point1: Point, point2: Point, frame: &mut Frame, style: Style) where Self: Sized {
        let mut radius = 2.0;
        let mut fill = Color::BLACK;
        if let Some((width, color, _, _)) = style.stroke {
            radius = width;
            fill = color;
        }

        let quad = Path::new(|builder| {
            let offset = Vector::new((45_f32).cos() * radius, (45_f32).sin() * radius);

            builder.move_to(point1.add(offset));
            builder.line_to(point2.add(offset.clone()));
            builder.line_to(point2.sub(offset.clone()));
            builder.line_to(point1.sub(offset.clone()));
            builder.close()
        });

        frame.fill(&quad, Fill::from(fill));
    }

    fn add_end(_point: Point, _frame: &mut Frame, _style: Style) where Self: Sized { }
}

impl Into<Box<dyn Tool>> for Box<Pen> {
    fn into(self) -> Box<dyn Tool> {
        self.boxed_clone()
    }
}