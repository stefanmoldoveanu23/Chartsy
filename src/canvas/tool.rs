use std::fmt::Debug;
use std::sync::Arc;
use iced::{mouse, Point, Rectangle, Renderer};
use iced::widget::canvas::{event, Event, Frame, Geometry};
use mongodb::bson::{Bson, Document};
use crate::canvas::layer::CanvasAction;
use crate::canvas::style::Style;
use crate::serde::{Deserialize, Serialize};
use crate::theme::Theme;
use crate::canvas::tools::{line::Line, rect::Rect, triangle::Triangle, polygon::Polygon, circle::Circle, ellipse::Ellipse};
use crate::canvas::tools::brushes::{eraser::Eraser, pencil::Pencil, pen::Pen, airbrush::Airbrush};

pub trait Tool: Debug+Send+Sync+Serialize+Deserialize {
    fn add_to_frame(&self, frame: &mut Frame);

    fn boxed_clone(&self) -> Box<dyn Tool>;

    fn id(&self) -> String;
}

pub fn get_deserialized(document: Document) -> Option<(Arc<dyn Tool>, usize)> {
    let mut layer :usize= 0;
    if let Some(Bson::Int32(layer_count)) = document.get("layer") {
        layer = *layer_count as usize;
    }

    if let Some(Bson::String(name)) = document.get("name") {
        match &name[..] {
            "Line" => Some((Arc::new(Line::deserialize(document)), layer)),
            "Rect" => Some((Arc::new(Rect::deserialize(document)), layer)),
            "Triangle" => Some((Arc::new(Triangle::deserialize(document)), layer)),
            "Polygon" => Some((Arc::new(Polygon::deserialize(document)), layer)),
            "Circle" => Some((Arc::new(Circle::deserialize(document)), layer)),
            "Ellipse" => Some((Arc::new(Ellipse::deserialize(document)), layer)),
            "FountainPen" => Some((Arc::new(Pen::deserialize(document)), layer)),
            "Pencil" => Some((Arc::new(Pencil::deserialize(document)), layer)),
            "Airbrush" => Some((Arc::new(Airbrush::deserialize(document)), layer)),
            "Eraser" => Some((Arc::new(Eraser::deserialize(document)), layer)),
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
        style: Style,
    ) -> (event::Status, Option<CanvasAction>);

    fn draw(
        &self,
        renderer: &Renderer<Theme>,
        bounds: Rectangle,
        cursor: mouse::Cursor,
        style: Style,
    ) -> Geometry;

    fn shape_style(&self, style: &mut Style);

    fn id(&self) -> String;

    fn default() -> Self where Self:Sized;

    fn boxed_clone(&self) -> Box<dyn Pending>;
}

impl Clone for Box<dyn Pending> {
    fn clone(&self) -> Self {
        self.boxed_clone()
    }
}