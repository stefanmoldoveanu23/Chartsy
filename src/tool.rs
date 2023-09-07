use std::fmt::Debug;
use iced::{mouse, Point, Rectangle, Renderer};
use iced::widget::canvas::{event, Event, Frame, Geometry};

pub trait Tool: Debug+Send+Sync {
    fn add_to_frame(&self, frame: &mut Frame);
    fn boxed_clone(&self) -> Box<dyn Tool>;
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