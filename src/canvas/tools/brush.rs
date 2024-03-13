use crate::canvas::layer::CanvasAction;
use crate::canvas::style::Style;
use crate::serde::{Deserialize, Serialize};
use iced::event::Status;
use iced::mouse::Cursor;
use iced::widget::canvas::{Event, Frame, Geometry};
use iced::{keyboard, mouse, Color, Point, Rectangle, Renderer, Vector};
use json::object::Object;
use json::JsonValue;
use mongodb::bson::{doc, Bson, Document};
use std::fmt::Debug;
use std::marker::PhantomData;
use std::ops::{Add, Sub};
use std::sync::Arc;
use iced::keyboard::Key;
use svg::node::element::Group;

use crate::canvas::tool::{Pending, Tool};

#[derive(Clone)]
pub enum BrushPending<BrushType>
where
    BrushType: Send + Sync + Clone + Brush,
{
    None,
    Stroking(Point, Point, Vec<Vector>),
    _PhantomVariant(PhantomData<BrushType>),
}

impl<BrushType: Send + Sync + Clone + Brush + 'static> Pending for BrushPending<BrushType>
where
    Box<BrushType>: Into<Box<dyn Tool>>,
{
    fn update(
        &mut self,
        event: Event,
        cursor: Point,
        style: Style,
    ) -> (Status, Option<CanvasAction>) {
        match event {
            Event::Mouse(mouse_event) => {
                let message = match mouse_event {
                    mouse::Event::ButtonPressed(mouse::Button::Left) => match self {
                        BrushPending::None => {
                            *self =
                                BrushPending::Stroking(cursor, cursor, vec![Vector::new(0.0, 0.0)]);

                            None
                        }
                        _ => None,
                    },
                    mouse::Event::CursorMoved { .. } => match self {
                        BrushPending::Stroking(start, last, offsets) => {
                            let mut new_offsets = offsets.clone();
                            new_offsets.push(cursor.sub(*last));

                            *self = BrushPending::Stroking(*start, cursor, new_offsets);

                            None
                        }
                        _ => None,
                    },
                    mouse::Event::ButtonReleased(mouse::Button::Left) => match self {
                        BrushPending::Stroking(start, _last, offsets) => {
                            let start_clone = start.clone();
                            let offsets_clone = offsets.clone();

                            *self = BrushPending::None;

                            Some(
                                CanvasAction::UseTool(Arc::new(BrushType::new(
                                    start_clone,
                                    offsets_clone,
                                    style,
                                )))
                                .into(),
                            )
                        }
                        _ => None,
                    },
                    _ => None,
                };

                (Status::Captured, message)
            }
            Event::Keyboard(key_event) => match key_event {
                keyboard::Event::KeyPressed {
                    key: Key::Character(key),
                    ..
                } => {
                    let value = key.as_str();
                    if value == "S" {
                        *self = BrushPending::None;

                        (Status::Captured, None)
                    } else {
                        (Status::Ignored, None)
                    }
                }
                _ => (Status::Ignored, None),
            },
            _ => (Status::Ignored, None),
        }
    }

    fn draw(
        &self,
        renderer: &Renderer,
        bounds: Rectangle,
        cursor: Cursor,
        style: Style,
    ) -> Geometry {
        let mut frame = Frame::new(renderer, bounds.size());

        if let Some(_cursor_position) = cursor.position_in(bounds) {
            match self {
                BrushPending::Stroking(start, _last, offsets) => {
                    let mut pos = *start;

                    for offset in offsets.clone() {
                        BrushType::add_stroke_piece(
                            pos,
                            pos.add(offset),
                            &mut frame,
                            style.clone(),
                        );
                        pos = pos.add(offset.clone());
                    }
                }
                _ => {}
            }
        };

        frame.into_geometry()
    }

    fn shape_style(&self, style: &mut Style) {
        if style.stroke.is_none() {
            style.stroke = Some((2.0, Color::BLACK, false, false));
        }

        style.fill = None;
    }

    fn id(&self) -> String {
        BrushType::id()
    }

    fn default() -> Self
    where
        Self: Sized,
    {
        BrushPending::None
    }

    fn boxed_clone(&self) -> Box<dyn Pending> {
        Box::new((*self).clone())
    }
}

pub trait Brush: Send + Sync + Debug {
    fn new(start: Point, offsets: Vec<Vector>, style: Style) -> Self
    where
        Self: Sized;
    fn id() -> String
    where
        Self: Sized;

    fn get_start(&self) -> Point;
    fn get_offsets(&self) -> Vec<Vector>;
    fn get_style(&self) -> Style;

    fn add_stroke_piece(point1: Point, point2: Point, frame: &mut Frame, style: Style)
    where
        Self: Sized;
    fn add_end(point: Point, frame: &mut Frame, style: Style)
    where
        Self: Sized;

    fn add_svg_stroke_piece(point1: Point, point2: Point, svg: Group, style: Style) -> Group
    where
        Self: Sized;
    fn add_svg_end(point: Point, svg: Group, style: Style) -> Group
    where
        Self: Sized;
}

impl<BrushType> Serialize<Document> for BrushType
where
    BrushType: Brush + Clone + 'static,
{
    fn serialize(&self) -> Document {
        doc! {
            "start": Document::from(self.get_start().serialize()),
            "offsets": self.get_offsets().iter().map(|offset| {offset.serialize()}).collect::<Vec<Document>>().as_slice(),
            "style": Document::from(self.get_style().serialize()),
        }
    }
}

impl<BrushType> Deserialize<Document> for BrushType
where
    BrushType: Brush + Clone + 'static,
{
    fn deserialize(document: Document) -> Self
    where
        Self: Sized,
    {
        let mut brush_start: Point = Point::default();
        let mut brush_offsets: Vec<Vector> = vec![];
        let mut brush_style: Style = Style::default();

        if let Some(Bson::Document(start)) = document.get("start") {
            brush_start = Point::deserialize(start.clone());
        }

        if let Some(Bson::Array(offsets)) = document.get("offsets") {
            for offset in offsets {
                if let Bson::Document(offset) = offset {
                    brush_offsets.push(Vector::deserialize(offset.clone()));
                }
            }
        }

        if let Some(Bson::Document(style)) = document.get("style") {
            brush_style = Style::deserialize(style.clone());
        }

        BrushType::new(brush_start, brush_offsets, brush_style)
    }
}

impl<BrushType> Serialize<Group> for BrushType
where
    BrushType: Brush + Clone + 'static,
{
    fn serialize(&self) -> Group {
        let mut pos = self.get_start();

        let mut ret = Group::new().set("class", BrushType::id());

        for offset in self.get_offsets() {
            ret = BrushType::add_svg_stroke_piece(pos, pos.add(offset), ret, self.get_style());
            pos = pos.add(offset.clone());
        }

        BrushType::add_svg_end(pos, ret, self.get_style())
    }
}

impl<BrushType> Serialize<Object> for BrushType
where
    BrushType: Brush + Clone + 'static,
{
    fn serialize(&self) -> Object {
        let mut data = Object::new();

        data.insert("start", JsonValue::Object(self.get_start().serialize()));
        data.insert(
            "offsets",
            JsonValue::Array(
                self.get_offsets()
                    .iter()
                    .map(|offset| JsonValue::Object(offset.serialize()))
                    .collect(),
            ),
        );
        data.insert("style", JsonValue::Object(self.get_style().serialize()));

        data
    }
}

impl<BrushType> Deserialize<Object> for BrushType
where
    BrushType: Brush + Clone + 'static,
{
    fn deserialize(document: Object) -> Self
    where
        Self: Sized,
    {
        let mut brush_start = Point::default();
        let mut brush_offsets: Vec<Vector> = vec![];
        let mut brush_style = Style::default();

        if let Some(JsonValue::Object(start)) = document.get("start") {
            brush_start = Point::deserialize(start.clone());
        }
        if let Some(JsonValue::Array(offsets)) = document.get("offsets") {
            for offset in offsets {
                if let JsonValue::Object(offset) = offset {
                    brush_offsets.push(Vector::deserialize(offset.clone()));
                }
            }
        }
        if let Some(JsonValue::Object(style)) = document.get("style") {
            brush_style = Style::deserialize(style.clone());
        }

        BrushType::new(brush_start, brush_offsets, brush_style)
    }
}

impl<BrushType> Tool for BrushType
where
    BrushType: Brush + Clone + 'static,
{
    fn add_to_frame(&self, frame: &mut Frame) {
        let mut pos = self.get_start();

        for offset in self.get_offsets() {
            BrushType::add_stroke_piece(pos, pos.add(offset), frame, self.get_style());
            pos = pos.add(offset.clone());
        }

        BrushType::add_end(pos, frame, self.get_style());
    }

    fn boxed_clone(&self) -> Box<dyn Tool> {
        Box::new((*self).clone())
    }

    fn id(&self) -> String {
        BrushType::id()
    }
}
