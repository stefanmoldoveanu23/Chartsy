use crate::canvas::style::Style;
use crate::canvas::tool::Tool;
use iced::widget::canvas::{Frame, LineCap, LineJoin, Path, Stroke};
use iced::{Point, Vector};
use std::fmt::Debug;
use svg::node::element::path::Data;
use svg::node::element::Group;

use crate::canvas::tools::brush::Brush;

#[derive(Debug, Clone)]
pub struct Pencil {
    start: Point,
    offsets: Vec<Vector>,
    style: Style,
}

impl Brush for Pencil {
    fn new(start: Point, offsets: Vec<Vector>, style: Style) -> Self
    where
        Self: Sized,
    {
        Pencil {
            start,
            offsets,
            style,
        }
    }

    fn id() -> String
    where
        Self: Sized,
    {
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

    fn add_stroke_piece(point1: Point, point2: Point, frame: &mut Frame, style: Style)
    where
        Self: Sized,
    {
        let line = Path::new(|builder| {
            builder.move_to(point1);
            builder.line_to(point2);
        });

        if let Some((width, color, _, _)) = style.stroke {
            frame.stroke(
                &line,
                Stroke::default()
                    .with_width(width)
                    .with_color(color)
                    .with_line_cap(LineCap::Round)
                    .with_line_join(LineJoin::Round),
            );
        }
    }

    fn add_end(_point: Point, _frame: &mut Frame, _style: Style)
    where
        Self: Sized,
    {
    }

    fn add_svg_stroke_piece(point1: Point, point2: Point, svg: Group, style: Style) -> Group
    where
        Self: Sized,
    {
        let data = Data::new()
            .move_to((point1.x, point1.y))
            .line_to((point2.x, point2.y));

        let path = svg::node::element::Path::new()
            .set("stroke-width", style.get_stroke_width())
            .set("stroke", style.get_stroke_color())
            .set("stroke-linecap", "round")
            .set("stroke-linejoin", "round")
            .set("stroke-opacity", style.get_stroke_alpha())
            .set("d", data);

        svg.add(path)
    }

    fn add_svg_end(_point: Point, svg: Group, _style: Style) -> Group
    where
        Self: Sized,
    {
        svg
    }
}

impl Into<Box<dyn Tool>> for Box<Pencil> {
    fn into(self) -> Box<dyn Tool> {
        self.boxed_clone()
    }
}
