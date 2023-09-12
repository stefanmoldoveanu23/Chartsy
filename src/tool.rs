use std::fmt::Debug;
use iced::{mouse, Point, Rectangle, Renderer};
use iced::widget::canvas::{event, Event, Frame, Geometry};
use mongodb::bson::{Bson, Document};
use crate::serde::{Deserialize, Serialize};
use crate::tools::{line::Line, rect::Rect, triangle::Triangle, polygon::Polygon, circle::Circle, ellipse::Ellipse};
use crate::tools::brushes::{eraser::Eraser, pencil::Pencil, pen::Pen, airbrush::Airbrush};

pub trait Tool: Debug+Send+Sync+Serialize+Deserialize {

    fn add_to_frame(&self, frame: &mut Frame);
    fn boxed_clone(&self) -> Box<dyn Tool>;
    fn id(&self) -> String;
}

pub fn get_deserialized(document: Document) -> Option<Box<dyn Tool>> {
    if let Some(Bson::String(name)) = document.get("name") {
        match &name[..] {
            "Line" => Some(Box::new(Line::deserialize(document))),
            "Rect" => Some(Box::new(Rect::deserialize(document))),
            "Triangle" => Some(Box::new(Triangle::deserialize(document))),
            "Polygon" => Some(Box::new(Polygon::deserialize(document))),
            "Circle" => Some(Box::new(Circle::deserialize(document))),
            "Ellipse" => Some(Box::new(Ellipse::deserialize(document))),
            "FountainPen" => Some(Box::new(Pen::deserialize(document))),
            "Pencil" => Some(Box::new(Pencil::deserialize(document))),
            "Airbrush" => Some(Box::new(Airbrush::deserialize(document))),
            "Eraser" => Some(Box::new(Eraser::deserialize(document))),
            _ => None
        }
    } else {
        None
    }
}

impl Clone for Box<dyn Tool> {
    fn clone(&self) -> Self {
        self.boxed_clone()
    }
}

pub trait Pending: Send+Sync {
    fn update(
        &mut self,
        event: Event,
        cursor: Point,
    ) -> (event::Status, Option<Box<dyn Tool>>);

    fn draw(
        &self,
        renderer: &Renderer,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Geometry;

    fn id(&self) -> String;

    fn default() -> Self where Self:Sized;
    fn boxed_clone(&self) -> Box<dyn Pending>;
}

impl Clone for Box<dyn Pending> {
    fn clone(&self) -> Self {
        self.boxed_clone()
    }
}