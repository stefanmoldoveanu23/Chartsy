use std::sync::Arc;
use iced::{Color, event, keyboard, Point, Rectangle, Renderer};
use iced::advanced::mouse;
use iced::mouse::Cursor;
use iced::widget::canvas::{self, fill::Rule};
<<<<<<< Updated upstream
use mongodb::bson::Document;
use mongodb::results::InsertManyResult;
use crate::canvas::style::{Style, StyleUpdate};
use crate::theme::Theme;
use crate::canvas::tool::{Pending, Tool};
=======
use iced::{event, keyboard, Color, Point, Rectangle, Renderer};
use json::JsonValue;
use std::sync::Arc;
use std::str::FromStr;
use iced::keyboard::Key;
use iced_style::core::SmolStr;
>>>>>>> Stashed changes

pub struct Layer<'a> {
    pub(crate) state: Option<&'a canvas::Cache>,
    pub(crate) tools: &'a [Arc<dyn Tool>],
    pub(crate) current_tool: &'a Box<dyn Pending>,
    pub(crate) style: &'a Style,
    pub active: bool,
}

<<<<<<< Updated upstream
impl<'a> canvas::Program<CanvasAction, Theme, Renderer> for Layer<'a>
{
=======
impl<'a> canvas::Program<CanvasAction, Theme, Renderer> for Layer<'a> {
>>>>>>> Stashed changes
    type State = Option<Box<dyn Pending>>;

    fn update(
        &self,
        state: &mut Self::State,
        event: canvas::Event,
        bounds: Rectangle,
        cursor: Cursor,
    ) -> (event::Status, Option<CanvasAction>) {
        if !self.active {
            return (event::Status::Ignored, None);
        }

<<<<<<< Updated upstream
        if let canvas::Event::Keyboard(ref event) = event {
            match event {
                keyboard::Event::KeyPressed {key, modifiers, ..} => {
                    if *key == keyboard::Key::Character("Z".into()) && *modifiers == keyboard::Modifiers::CTRL {
                        return (event::Status::Captured, Some(CanvasAction::Undo))
                    } else if *key == keyboard::Key::Character("S".into()) && *modifiers == keyboard::Modifiers::CTRL {
                        return (event::Status::Captured, Some(CanvasAction::Save))
                    } else if *key == keyboard::Key::Character("Y".into()) && *modifiers == keyboard::Modifiers::CTRL {
                        return (event::Status::Captured, Some(CanvasAction::Redo))
=======
        if let canvas::Event::Keyboard(event) = event.clone() {
            match event {
                keyboard::Event::KeyPressed {
                    key: Key::Character(char),
                    modifiers,
                    ..
                } => {
                    if char == SmolStr::from_str("Z").unwrap() && modifiers == keyboard::Modifiers::CTRL {
                        return (event::Status::Captured, Some(CanvasAction::Undo));
                    } else if char == SmolStr::from_str("S").unwrap()
                        && modifiers == keyboard::Modifiers::CTRL
                    {
                        return (event::Status::Captured, Some(CanvasAction::Save));
                    } else if char == SmolStr::from_str("Y").unwrap()
                        && modifiers == keyboard::Modifiers::CTRL
                    {
                        return (event::Status::Captured, Some(CanvasAction::Redo));
>>>>>>> Stashed changes
                    }
                }
                _ => {}
            }
        }

        let cursor_position =
            if let Some(position) = cursor.position_in(bounds) {
                position
            } else {
                return (event::Status::Ignored, None);
            };

        match state {
            None => {
                *state = Some((*self.current_tool).boxed_clone());
                (event::Status::Captured, None)
            },
            Some(pending_state) => {
                let new_tool = pending_state.id() != self.current_tool.id();
                if new_tool {
                    *state = Some((*self.current_tool).boxed_clone());
                    (event::Status::Ignored, None)
                } else {
                    pending_state.update(event, cursor_position, self.style.clone())
                }
            }
        }
    }

    fn draw(
        &self,
        state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        cursor: Cursor
    ) -> Vec<canvas::Geometry> {

        let base = {
            let mut frame = canvas::Frame::new(renderer, bounds.size());

            frame.fill_rectangle(
                Point::ORIGIN,
                frame.size(),
                canvas::Fill {
                    style: canvas::Style::Solid(Color::TRANSPARENT),
                    rule: Rule::NonZero }
            );

            frame.stroke(
                &canvas::Path::rectangle(Point::ORIGIN, frame.size()),
                canvas::Stroke::default().with_width(2.0)
            );

            frame.into_geometry()
        };

        let content = match self.state {
            None => {
                return vec![base];
            }
            Some(state) => {
                state.draw(
                    renderer,
                    bounds.size(),
                    |frame| {
                        for tool in self.tools {
                            tool.add_to_frame(frame);
                        }
                    }
                )
            }
        };

        if !self.active {
            return vec![base, content];
        }

        let pending = match state {
            None => {
                return vec![base, content];
            }
            Some(state) => {
                state.draw(renderer, bounds, cursor, self.style.clone())
            }
        };

        vec![base, content, pending]
    }

    fn mouse_interaction(
        &self,
        _state: &Self::State,
        bounds: Rectangle,
        cursor: Cursor,
    ) -> mouse::Interaction {
        if cursor.is_over(bounds) {
            mouse::Interaction::Crosshair
        } else {
            mouse::Interaction::default()
        }
    }
}

#[derive(Clone)]
pub enum CanvasAction {
    UseTool(Arc<dyn Tool>),
    ChangeTool(Box<dyn Pending>),
    UpdateStyle(StyleUpdate),
    AddLayer,
    ActivateLayer(usize),
    Save,
    Saved(Arc<InsertManyResult>),
    Loaded(usize, Vec<Document>),
    Undo,
    Redo,
}