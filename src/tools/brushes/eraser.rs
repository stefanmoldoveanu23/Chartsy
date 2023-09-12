use std::f32::consts::PI;
use std::fmt::{Debug};
use std::ops::{Add, Sub};
use iced::{Color, Point, Vector};
use iced::widget::canvas::{Fill, Frame, Path, Style};
use iced::widget::canvas::fill::Rule;

use crate::tool::Tool;

use crate::tools::brush::Brush;

#[derive(Debug, Clone)]
pub struct Eraser {
    start: Point,
    offsets: Vec<Vector>,
}


impl Brush for Eraser {
    fn new(start: Point, offsets: Vec<Vector>) -> Self where Self: Sized {
        Eraser { start, offsets }
    }

    fn id() -> String where Self: Sized {
        String::from("Eraser")
    }

    fn get_start(&self) -> Point {
        self.start
    }

    fn get_offsets(&self) -> Vec<Vector> {
        self.offsets.clone()
    }

    fn add_stroke_piece(point1: Point, point2: Point, frame: &mut Frame) where Self: Sized {
        let offset = point2.sub(point1);

        let angle = offset.y.atan2(offset.x) + PI / 2.0;
        let offset = Vector::new(10.0 * angle.cos(), 10.0 * angle.sin());

        let circle = Path::new(|builder| {
            builder.circle(point1, 10.0);
        });

        frame.fill(&circle, Fill {style: Style::Solid(Color::WHITE), rule: Rule::NonZero});

        let quad = Path::new(|builder| {
            builder.move_to(point1.add(offset));
            builder.line_to(point2.add(offset.clone()));
            builder.line_to(point2.sub(offset.clone()));
            builder.line_to(point1.sub(offset.clone()));
            builder.close();
        });

        frame.fill(&quad, Fill {style: Style::Solid(Color::WHITE), rule: Rule::NonZero});
    }

    fn add_end(point: Point, frame: &mut Frame) where Self: Sized {
        let circle = Path::new(|builder| {
            builder.circle(point, 10.0);
        });

        frame.fill(&circle, Fill {style: Style::Solid(Color::WHITE), rule: Rule::NonZero});
    }
}

impl Into<Box<dyn Tool>> for Box<Eraser> {
    fn into(self) -> Box<dyn Tool> {
        self.boxed_clone()
    }
}