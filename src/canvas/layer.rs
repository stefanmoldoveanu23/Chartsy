use std::sync::Arc;
use iced::{Color, event, keyboard, Point, Rectangle, Renderer};
use iced::advanced::mouse;
use iced::mouse::Cursor;
use iced::widget::canvas::{self, fill::Rule};
use mongodb::bson::Document;
use mongodb::results::InsertManyResult;
use crate::canvas::style::{Style, StyleUpdate};
use crate::theme::Theme;
use crate::canvas::tool::{Pending, Tool};

#[derive(Default)]
pub struct State {
    cache: canvas::Cache,
}

unsafe impl Send for State {}
unsafe impl Sync for State {}

pub struct Layer<'a> {
    pub(crate) state: Option<&'a canvas::Cache>,
    pub(crate) tools: &'a [Arc<dyn Tool>],
    pub(crate) current_tool: &'a Box<dyn Pending>,
    pub(crate) style: &'a Style,
    pub active: bool,
}

impl<'a> canvas::Program<CanvasAction, Renderer<Theme>> for Layer<'a>
{
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

        if let canvas::Event::Keyboard(event) = event {
            match event {
                keyboard::Event::KeyPressed {key_code, modifiers} => {
                    if key_code == keyboard::KeyCode::Z && modifiers == keyboard::Modifiers::CTRL {
                        return (event::Status::Captured, Some(CanvasAction::Undo))
                    } else if key_code == keyboard::KeyCode::S && modifiers == keyboard::Modifiers::CTRL {
                        return (event::Status::Captured, Some(CanvasAction::Save))
                    } else if key_code == keyboard::KeyCode::Y && modifiers == keyboard::Modifiers::CTRL {
                        return (event::Status::Captured, Some(CanvasAction::Redo))
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
        renderer: &Renderer<Theme>,
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