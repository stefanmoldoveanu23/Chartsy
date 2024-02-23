use std::fmt::Debug;
use std::sync::Arc;
use iced::{mouse, Point, Rectangle, Renderer};
use iced::widget::canvas::{event, Event, Frame, Geometry};
use mongodb::bson::{Bson, Document};
use svg::node::element::Group;
use crate::canvas::layer::CanvasAction;
use crate::canvas::style::Style;
use crate::serde::{Deserialize, Serialize};
use crate::theme::Theme;
use crate::canvas::tools::{line::Line, rect::Rect, triangle::Triangle, polygon::Polygon, circle::Circle, ellipse::Ellipse};
use crate::canvas::tools::brushes::{eraser::Eraser, pencil::Pencil, pen::Pen, airbrush::Airbrush};

/// Any tool that can be used on the [canvas](crate::canvas::canvas::Canvas).
pub trait Tool: Debug+Send+Sync+Serialize+Deserialize {
    /// Adds the [Tool] to the given [Frame].
    fn add_to_frame(&self, frame: &mut Frame);

    /// Adds the [Tool] to the given [Group].
    fn add_to_svg(&self, svg: Group) -> Group;

    /// Creates a clone of the [Tool] and encloses it into a [Box].
    fn boxed_clone(&self) -> Box<dyn Tool>;

    /// Returns a unique identifier for the [Tool].
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
            "Rectangle" => Some((Arc::new(Rect::deserialize(document)), layer)),
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

/// A version of a [Tool] to be used for easily marking its drawing progress.
/// It is advised to be implemented as an enum where each variant represents a state in the shaping
/// of the [Tool], as it is intended to be used as the State type for the canvas'
/// [Program](iced::widget::canvas::Program).
///
/// # Example
/// ```no_run
/// enum Triangle {
///     None,
///     One(Point),
///     Two(Point, Point),
/// }
///
/// impl Pending for Triangle {
///     ...
/// }
/// ```
///
pub trait Pending: Send+Sync {
    /// Handles an [Event] on the [canvas](crate::canvas::canvas::Canvas). To be used in the
    /// Programs' [update function](iced::widget::canvas::Program::update).
    fn update(
        &mut self,
        event: Event,
        cursor: Point,
        style: Style,
    ) -> (event::Status, Option<CanvasAction>);

    /// Draws the [pending tool](Pending) on the [canvas](crate::canvas::canvas::Canvas). To be
    /// used in the Programs' [draw function](iced::widget::canvas::Program::draw).
    fn draw(
        &self,
        renderer: &Renderer<Theme>,
        bounds: Rectangle,
        cursor: mouse::Cursor,
        style: Style,
    ) -> Geometry;

    /// Modifies the given [Style] to make available or unavailable settings as necessary.
    fn shape_style(&self, style: &mut Style);

    /// Returns a unique identifier for the [pending tool](Pending).
    fn id(&self) -> String;

    /// Returns a default version of the [pending tool](Pending).
    fn default() -> Self where Self:Sized;

    /// Returns a clone of the [pending tool](Pending) enclosed in a [Box].
    fn boxed_clone(&self) -> Box<dyn Pending>;
}

impl Clone for Box<dyn Pending> {
    fn clone(&self) -> Self {
        self.boxed_clone()
    }
}