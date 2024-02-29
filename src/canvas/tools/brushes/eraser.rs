use crate::canvas::style::Style;
use iced::widget::canvas::{Fill, Frame, Path};
use iced::{Color, Point, Vector};
use std::f32::consts::PI;
use std::fmt::Debug;
use std::ops::{Add, Sub};
use svg::node::element::path::Data;
use svg::node::element::Group;

use crate::canvas::tool::Tool;

use crate::canvas::tools::brush::Brush;

#[derive(Debug, Clone)]
pub struct Eraser {
    start: Point,
    offsets: Vec<Vector>,
}

impl Brush for Eraser {
    fn new(start: Point, offsets: Vec<Vector>, _style: Style) -> Self
    where
        Self: Sized,
    {
        Eraser { start, offsets }
    }

    fn id() -> String
    where
        Self: Sized,
    {
        String::from("Eraser")
    }

    fn get_start(&self) -> Point {
        self.start
    }

    fn get_offsets(&self) -> Vec<Vector> {
        self.offsets.clone()
    }
    fn get_style(&self) -> Style {
        Style::default()
    }

    fn add_stroke_piece(point1: Point, point2: Point, frame: &mut Frame, _style: Style)
    where
        Self: Sized,
    {
        let offset = point2.sub(point1);

        let angle = offset.y.atan2(offset.x) + PI / 2.0;
        let offset = Vector::new(10.0 * angle.cos(), 10.0 * angle.sin());

        let circle = Path::new(|builder| {
            builder.circle(point1, 10.0);
        });

        frame.fill(&circle, Fill::from(Color::WHITE));

        let quad = Path::new(|builder| {
            builder.move_to(point1.add(offset));
            builder.line_to(point2.add(offset.clone()));
            builder.line_to(point2.sub(offset.clone()));
            builder.line_to(point1.sub(offset.clone()));
            builder.close();
        });

        frame.fill(&quad, Fill::from(Color::WHITE));
    }

    fn add_end(point: Point, frame: &mut Frame, _style: Style)
    where
        Self: Sized,
    {
        let circle = Path::new(|builder| {
            builder.circle(point, 10.0);
        });

        frame.fill(&circle, Fill::from(Color::WHITE));
    }

    fn add_svg_stroke_piece(point1: Point, point2: Point, svg: Group, _style: Style) -> Group
    where
        Self: Sized,
    {
        let offset = point2.sub(point1);

        let angle = offset.y.atan2(offset.x) + PI / 2.0;
        let offset = Vector::new(10.0 * angle.cos(), 10.0 * angle.sin());

        let circle = svg::node::element::Circle::new()
            .set("cx", point1.x)
            .set("cy", point1.y)
            .set("r", 10.0)
            .set("fill", "#ffffffff");

        let data = Data::new()
            .move_to((point1.add(offset).x, point1.add(offset).y))
            .line_to((point2.add(offset).x, point2.add(offset).y))
            .line_to((point2.sub(offset).x, point2.sub(offset).y))
            .line_to((point1.sub(offset).x, point1.sub(offset).y))
            .close();

        let path = svg::node::element::Path::new()
            .set("fill", "#ffffffff")
            .set("d", data);

        svg.add(circle).add(path)
    }

    fn add_svg_end(point: Point, svg: Group, _style: Style) -> Group
    where
        Self: Sized,
    {
        let circle = svg::node::element::Circle::new()
            .set("cx", point.x)
            .set("cy", point.y)
            .set("radius", 10.0)
            .set("fill", "#ffffffff");

        svg.add(circle)
    }
}

impl Into<Box<dyn Tool>> for Box<Eraser> {
    fn into(self) -> Box<dyn Tool> {
        self.boxed_clone()
    }
}
