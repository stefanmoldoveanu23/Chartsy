use crate::canvas::style::{Style, StyleUpdate};
use crate::canvas::tool::{Pending, Tool};
use crate::utils::theme::Theme;
use iced::advanced::mouse;
use iced::mouse::Cursor;
use iced::widget::canvas::{self};
use iced::{event, keyboard, Rectangle, Renderer};
use json::JsonValue;
use std::sync::Arc;
use iced::keyboard::Key;
use mongodb::bson::Uuid;
use crate::scene::Message;
use crate::scenes::drawing::DrawingMessage;

/// A layer in the [canvas](crate::canvas::canvas::Canvas).
pub struct Layer {
    /// The cache memory of the [Layer].
    cache: canvas::Cache,

    /// The tools drawn on the [Layer].
    tools: Vec<Arc<dyn Tool>>,

    /// The name of the [Layer].
    name: String,

    /// The new name for the [Layer]. Is None if it is not being edited.
    new_name: Option<String>,

    /// Tells whether the [Layer] is visible.
    visible: bool,
}

impl Layer {
    pub fn new(name: String) -> Self {
        Layer {
            name,
            ..Default::default()
        }
    }

    pub fn clear_cache(&self) {
        self.cache.clear()
    }

    pub fn get_cache(&self) -> &canvas::Cache {
        &self.cache
    }

    pub fn get_tools(&self) -> &[Arc<dyn Tool>] {
        self.tools.as_slice()
    }

    pub fn get_mut_tools(&mut self) -> &mut Vec<Arc<dyn Tool>> {
        &mut self.tools
    }

    pub fn get_name(&self) -> &String {
        &self.name
    }

    pub fn get_new_name(&self) -> &Option<String> {
        &self.new_name
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn toggle_visibility(&mut self) {
        self.visible = !self.visible;
    }

    pub fn toggle_name(&mut self) -> Option<String> {
        match self.new_name.clone() {
            Some(new_name) => {
                self.new_name = None;
                self.name = new_name.clone();

                Some(new_name)
            }
            None => {
                self.new_name = Some(self.name.clone());

                None
            }
        }
    }

    pub fn set_new_name(&mut self, new_name: impl Into<Option<String>>) {
        self.new_name = new_name.into();
    }
}

unsafe impl Send for Layer { }
unsafe impl Sync for Layer { }

impl Default for Layer {
    fn default() -> Self {
        Layer {
            cache: Default::default(),
            tools: vec![],
            name: "New layer".to_string(),
            new_name: None,
            visible: true,

        }
    }
}

/// A structure used to render a layer.
pub struct LayerVessel<'a> {
    /// The cache of the [LayerVessel].
    state: &'a canvas::Cache,

    /// The [tools](Tool) stored on the [LayerVessel].
    tools: &'a [Arc<dyn Tool>],

    /// The currently selected [Tool].
    current_tool: &'a Box<dyn Pending>,

    /// The currently selected [Style].
    style: &'a Style,

    /// Tells whether this layer is currently being drawn on.
    active: bool,
}

impl<'a> LayerVessel<'a>
{
    pub fn new(
        state: &'a canvas::Cache,
        tools: &'a [Arc<dyn Tool>],
        current_tool: &'a Box<dyn Pending>,
        style: &'a Style,
        active: bool
    ) -> Self {
        LayerVessel {
            state,
            tools,
            current_tool,
            style,
            active,
        }
    }
}

impl<'a> canvas::Program<CanvasMessage, Theme, Renderer> for LayerVessel<'a> {
    type State = Option<Box<dyn Pending>>;

    fn update(
        &self,
        state: &mut Self::State,
        event: canvas::Event,
        bounds: Rectangle,
        cursor: Cursor,
    ) -> (event::Status, Option<CanvasMessage>) {
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

                    if (value == "Z" || value == "z") && modifiers == keyboard::Modifiers::CTRL {
                        return (event::Status::Captured, Some(CanvasMessage::Undo));
                    } else if (value == "S" || value == "s") && modifiers == keyboard::Modifiers::CTRL
                    {
                        return (event::Status::Captured, Some(CanvasMessage::Save));
                    } else if (value == "Y" || value == "y") && modifiers == keyboard::Modifiers::CTRL
                    {
                        return (event::Status::Captured, Some(CanvasMessage::Redo));
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
        let content = self.state.draw(renderer, bounds.size(), |frame| {
                for tool in self.tools {
                    tool.add_to_frame(frame);
                }
        });

        if !self.active {
            return vec![content];
        }

        let pending = match state {
            None => {
                return vec![content];
            }
            Some(state) => state.draw(renderer, bounds, cursor, self.style.clone()),
        };

        vec![content, pending]
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

/// Scene messages that relate to the [canvas](crate::canvas::canvas::Canvas).
#[derive(Clone)]
pub enum CanvasMessage {
    /// Adds a [Tool] to the active [Layer].
    UseTool(Arc<dyn Tool>),

    /// Changed the [Tool] used for drawing.
    ChangeTool(Box<dyn Pending>),

    /// Updates the [Style].
    UpdateStyle(StyleUpdate),

    /// Appends a new [Layer].
    AddLayer,

    /// Sets the currently active [Layer].
    ActivateLayer(Uuid),

    /// Toggles the visibility of a [Layer].
    ToggleLayer(Uuid),

    /// Toggles the editing of the [Layer] name.
    ToggleEditLayerName(Uuid),

    /// Updates the [Layer] name when user inputs.
    UpdateLayerName(Uuid, String),

    /// Deletes a [Layer].
    RemoveLayer(Uuid),

    /// Saves the state of the drawing.
    Save,

    /// Triggered when the drawing is successfully saved.
    Saved,

    /// Triggered when the drawing data is successfully loaded.
    Loaded {
        layers: Vec<(Uuid, String)>,
        tools: Vec<(Arc<dyn Tool>, Uuid)>,
        json_tools: Option<Vec<JsonValue>>,
    },

    /// Removes the last added [Tool].
    Undo,

    /// Adds the last removed [Tool].
    Redo,
}

impl Into<Message> for CanvasMessage {
    fn into(self) -> Message {
        DrawingMessage::CanvasMessage(self).into()
    }
}

impl Into<DrawingMessage> for CanvasMessage {
    fn into(self) -> DrawingMessage {
        DrawingMessage::CanvasMessage(self)
    }
}