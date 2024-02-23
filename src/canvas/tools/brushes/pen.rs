use std::fmt::{Debug};
use std::ops::{Add, Sub};
use iced::{Point, Vector};
use iced::widget::canvas::{Fill, Frame, Path};
use iced_runtime::core::Color;
use svg::node::element::Group;
use svg::node::element::path::Data;
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
            let offset = Vector::new(45_f32.cos() * radius, 45_f32.sin() * radius);

            builder.move_to(point1.add(offset));
            builder.line_to(point2.add(offset.clone()));
            builder.line_to(point2.sub(offset.clone()));
            builder.line_to(point1.sub(offset.clone()));
            builder.close()
        });

        frame.fill(&quad, Fill::from(fill));
    }

    fn add_end(_point: Point, _frame: &mut Frame, _style: Style) where Self: Sized { }

    fn add_svg_stroke_piece(point1: Point, point2: Point, svg: Group, style: Style) -> Group where Self: Sized {
        let radius = style.get_stroke_width();

        let offset = Vector::new((45_f32).cos() * radius, (45_f32).sin() * radius);

        let data = Data::new()
            .move_to((point1.add(offset).x, point1.add(offset).y))
            .line_to((point2.add(offset).x, point2.add(offset).y))
            .line_to((point2.sub(offset).x, point2.sub(offset).y))
            .line_to((point1.sub(offset).x, point1.sub(offset).y))
            .close();

        let path = svg::node::element::Path::new()
            .set("fill", style.get_stroke_color())
            .set("fill-opacity", style.get_stroke_alpha())
            .set("style", "mix-blend-mode:hard-light")
            .set("d", data);

        svg.add(path)
    }

    fn add_svg_end(_point: Point, svg: Group, _style: Style) -> Group where Self: Sized { svg }
}

impl Into<Box<dyn Tool>> for Box<Pen> {
    fn into(self) -> Box<dyn Tool> {
        self.boxed_clone()
    }
}