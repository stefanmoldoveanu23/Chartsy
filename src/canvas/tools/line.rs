use std::fmt::{Debug};
use std::sync::Arc;
use iced::{Color, mouse, Point, Rectangle, Renderer};
use iced::event::Status;
use iced::mouse::Cursor;
use iced::widget::canvas::{Event, Frame, Geometry, Path, Stroke};
use mongodb::bson::{Bson, doc, Document};
use svg::node::element::{self, path::Data};
use crate::canvas::layer::CanvasAction;
use crate::canvas::style::Style;
use crate::serde::{Deserialize, Serialize};
use crate::theme::Theme;

use crate::canvas::tool::{Pending, Tool};

#[derive(Clone)]
pub enum LinePending {
    None,
    One(Point),
}

impl Pending for LinePending {
    fn update(
        &mut self,
        event: Event,
        cursor: Point,
        style: Style,
    ) -> (Status, Option<CanvasAction>) {
        match event {
            Event::Mouse(mouse_event) => {
                let message = match mouse_event {
                    mouse::Event::ButtonPressed(mouse::Button::Left) => {
                        match self {
                            LinePending::None => {
                                *self = LinePending::One(cursor);
                                None
                            }
                            LinePending::One(start) => {
                                let start_clone = start.clone();

                                *self = LinePending::None;
                                Some(CanvasAction::UseTool(Arc::new(Line{start:start_clone, end:cursor, style})).into())
                            }
                        }
                    }
                    _ => None
                };

                (Status::Captured, message)
            }
            _ => (Status::Ignored, None)
        }
    }

    fn draw(
        &self,
        renderer: &Renderer<Theme>,
        bounds: Rectangle,
        cursor: Cursor,
        style: Style,
    ) -> Geometry {
        let mut frame = Frame::new(renderer, bounds.size());

        if let Some(cursor_position) = cursor.position_in(bounds) {
            match self {
                LinePending::None => {}
                LinePending::One(start) => {
                    let stroke = Path::new(|p| {
                        p.move_to(*start);
                        p.line_to(cursor_position);
                    });

                    if let Some((width, color, _, _)) = style.stroke {
                        frame.stroke(&stroke, Stroke::default().with_width(width).with_color(color));
                    }
                }
            }
        };

        frame.into_geometry()
    }

    fn shape_style(&self, style: &mut Style) {
        if style.stroke.is_none() {
            style.stroke = Some((2.0, Color::BLACK, false, false));
        }

        if style.fill.is_none() {
            style.fill = Some((Color::TRANSPARENT, false));
        }
    }

    fn id(&self) -> String {
        String::from("Line")
    }

    fn default() -> Self where Self: Sized {
        LinePending::None
    }

    fn boxed_clone(&self) -> Box<dyn Pending> {
        Box::new((*self).clone())
    }
}

#[derive(Debug, Clone)]
pub struct Line {
    start: Point,
    end: Point,
    style: Style,
}

impl Serialize for Line {
    fn serialize(&self) -> Document {
        doc! {
            "start": self.start.serialize(),
            "end": self.end.serialize(),
            "style": self.style.serialize(),
        }
    }
}

impl Deserialize for Line {
    fn deserialize(document: Document) -> Self where Self: Sized {
        let mut line = Line { start: Point::default(), end: Point::default(), style: Style::default() };

        if let Some(Bson::Document(start)) = document.get("start") {
            line.start = Point::deserialize(start.clone());
        }

        if let Some(Bson::Document(end)) = document.get("end") {
            line.end = Point::deserialize(end.clone());
        }

        if let Some(Bson::Document(style)) = document.get("style") {
            line.style = Style::deserialize(style.clone());
        }

        line
    }
}

impl Tool for Line {
    fn add_to_frame(&self, frame: &mut Frame) {
        let line = Path::new(|builder| {
            builder.move_to(self.start);
            builder.line_to(self.end);
        });

        println!("Trying to draw a line!");
        if let Some((width, color, _, _)) = self.style.stroke {
            println!("Drew a line!");
            frame.stroke(&line, Stroke::default().with_width(width).with_color(color));
        }
    }

    fn add_to_svg(&self, svg: svg::Document) -> svg::Document {
        let data = Data::new()
            .move_to((self.start.x, self.start.y))
            .line_to((self.end.x, self.end.y))
            .close();

        let path = element::Path::new()
            .set("fill", "none")
            .set("fill-opacity", self.style.get_fill_alpha())
            .set("stroke-width", self.style.get_stroke_width())
            .set("stroke", self.style.get_stroke_color())
            .set("stroke-opacity", self.style.get_stroke_alpha())
            .set("d", data);

        svg.add(path)
    }

    fn boxed_clone(&self) -> Box<dyn Tool> {
        Box::new((*self).clone())
    }

    fn id(&self) -> String {
        "Line".into()
    }
}

impl Into<Box<dyn Tool>> for Box<Line> {
    fn into(self) -> Box<dyn Tool> {
        self.boxed_clone()
    }
}