use crate::canvas::style::{Style, StyleUpdate};
use crate::canvas::tool::{Pending, Tool};
use crate::theme::Theme;
use iced::advanced::mouse;
use iced::mouse::Cursor;
use iced::widget::canvas::{self, fill::Rule};
use iced::{event, keyboard, Color, Point, Rectangle, Renderer};
use json::JsonValue;
use std::sync::Arc;
use iced::keyboard::Key;

/// A layer in the [canvas](crate::canvas::canvas::Canvas).
pub struct Layer<'a> {
    /// The cache of the [Layer].
    pub(crate) state: Option<&'a canvas::Cache>,

    /// The [tools](Tool) stored on the [Layer].
    pub(crate) tools: &'a [Arc<dyn Tool>],

    /// The currently selected [Tool].
    pub(crate) current_tool: &'a Box<dyn Pending>,

    /// The currently selected [Style].
    pub(crate) style: &'a Style,

    /// Tells whether this layer is currently being drawn on.
    pub active: bool,
}

impl<'a> canvas::Program<CanvasAction, Theme, Renderer> for Layer<'a> {
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

        if let canvas::Event::Keyboard(event) = event.clone() {
            match event {
                keyboard::Event::KeyPressed {
                    key: Key::Character(key),
                    modifiers,
                    ..
                } => {
                    let value :&str= key.as_str();

                    if value == "Z" && modifiers == keyboard::Modifiers::CTRL {
                        return (event::Status::Captured, Some(CanvasAction::Undo));
                    } else if value == "S" && modifiers == keyboard::Modifiers::CTRL
                    {
                        return (event::Status::Captured, Some(CanvasAction::Save));
                    } else if value == "Y" && modifiers == keyboard::Modifiers::CTRL
                    {
                        return (event::Status::Captured, Some(CanvasAction::Redo));
                    } else {
                        return (event::Status::Ignored, None);
                    }
                }
                _ => {}
            }
        }

        let cursor_position = if let Some(position) = cursor.position_in(bounds) {
            position
        } else {
            return (event::Status::Ignored, None);
        };

        match state {
            None => {
                *state = Some((*self.current_tool).boxed_clone());
                (event::Status::Captured, None)
            }
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
        cursor: Cursor,
    ) -> Vec<canvas::Geometry> {
        let base = {
            let mut frame = canvas::Frame::new(renderer, bounds.size());

            frame.fill_rectangle(
                Point::ORIGIN,
                frame.size(),
                canvas::Fill {
                    style: canvas::Style::Solid(Color::TRANSPARENT),
                    rule: Rule::NonZero,
                },
            );

            frame.stroke(
                &canvas::Path::rectangle(Point::ORIGIN, frame.size()),
                canvas::Stroke::default().with_width(2.0),
            );

            frame.into_geometry()
        };

        let content = match self.state {
            None => {
                return vec![base];
            }
            Some(state) => state.draw(renderer, bounds.size(), |frame| {
                for tool in self.tools {
                    tool.add_to_frame(frame);
                }
            }),
        };

        if !self.active {
            return vec![base, content];
        }

        let pending = match state {
            None => {
                return vec![base, content];
            }
            Some(state) => state.draw(renderer, bounds, cursor, self.style.clone()),
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

/// Scene messages that relate to the [canvas](crate::canvas::canvas::Canvas):
/// - [UseTool](CanvasAction::UseTool), to add a tool to the drawing;
/// - [ChangeTool](CanvasAction::ChangeTool), to select a new drawing tool;
/// - [UpdateStyle](CanvasAction::UpdateStyle), to modify the drawing style parameters;
/// - [AddLayer](CanvasAction::AddLayer), to add a new layer to the drawing;
/// - [ActivateLayer](CanvasAction::ActivateLayer), to select the layer on which the tools will be added;
/// - [Save](CanvasAction::Save), to save the progress since the last save;
/// - [Saved](CanvasAction::Saved), which triggers when a save is complete;
/// - [Loaded](CanvasAction::Loaded), which triggers when the drawing data is received;
/// - [Undo](CanvasAction::Undo), to undo the last tool addition;
/// - [Redo](CanvasAction::Redo), to redo the last undo.
#[derive(Clone)]
pub enum CanvasAction {
    /// Adds a [Tool] to the active [Layer].
    UseTool(Arc<dyn Tool>),

    /// Changed the [Tool] used for drawing.
    ChangeTool(Box<dyn Pending>),

    /// Updates the [Style].
    UpdateStyle(StyleUpdate),

    /// Appends a new [Layer].
    AddLayer,

    /// Sets the currently active [Layer].
    ActivateLayer(usize),

    /// Saves the state of the drawing.
    Save,

    /// Triggered when the drawing is successfully saved.
    Saved,

    /// Triggered when the drawing data is successfully loaded.
    Loaded {
        layers: usize,
        tools: Vec<(Arc<dyn Tool>, usize)>,
        json_tools: Option<Vec<JsonValue>>,
    },

    /// Removes the last added [Tool].
    Undo,

    /// Adds the last removed [Tool].
    Redo,
}
