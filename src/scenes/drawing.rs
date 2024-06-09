use std::any::Any;

use crate::canvas::canvas::Canvas;
use crate::canvas::svg::SVG;
use crate::widgets::{ModalStack, WaitPanel};
use iced::widget::text_editor::Content;
use iced::widget::Container;
use iced::{Command, Element, Length, Renderer};
use json::object::Object;
use json::JsonValue;
use mongodb::bson::Uuid;

use crate::canvas::layer::CanvasMessage;
use crate::canvas::tools::line::LinePending;
use crate::scene::{Globals, Message, Scene, SceneMessage};
use crate::utils::errors::Error;
use crate::{database, scenes::services, utils};

use crate::utils::theme::Theme;

use crate::scenes::data::drawing::*;

use super::scenes::Scenes;

/// The [Messages](SceneMessage) for the [Drawing] scene.
#[derive(Clone)]
pub enum DrawingMessage {
    /// Triggered when the user has interacted with the canvas.
    CanvasMessage(CanvasMessage),

    /// Creates a new post given the canvas and the [PostData].
    PostDrawing,

    /// Saves the file with the format and location that the user provides.
    SaveAs,

    /// Updates the [PostData] given the modified field.
    UpdatePostData(UpdatePostData),

    /// Deletes the currently opened drawing.
    DeleteDrawing,

    /// Toggles a [Modal](ModalTypes).
    ToggleModal(ModalTypes),

    /// Handles errors.
    ErrorHandler(Error),
}

impl Into<Message> for DrawingMessage {
    fn into(self) -> Message {
        Message::DoAction(Box::new(self))
    }
}

impl SceneMessage for DrawingMessage {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn get_name(&self) -> String {
        match self {
            Self::CanvasMessage(_) => String::from("Canvas action"),
            Self::PostDrawing => String::from("Post drawing"),
            Self::SaveAs => String::from("Save as..."),
            Self::UpdatePostData(_) => String::from("Update post data"),
            Self::DeleteDrawing => String::from("Delete drawing"),
            Self::ToggleModal(_) => String::from("Toggle modal"),
            Self::ErrorHandler(_) => String::from("Handle error"),
        }
    }

    fn boxed_clone(&self) -> Box<dyn SceneMessage + 'static> {
        Box::new((*self).clone())
    }
}

impl Into<Box<dyn SceneMessage + 'static>> for Box<DrawingMessage> {
    fn into(self) -> Box<dyn SceneMessage + 'static> {
        self
    }
}

/// The drawing scene of the [Application](crate::Chartsy).
pub struct Drawing {
    /// The canvas where the user can draw.
    canvas: Canvas,

    /// The new post data.
    post_data: PostData,

    /// The save mode of the drawing.
    save_mode: SaveMode,

    /// The stack of modals displayed.
    modal_stack: ModalStack<ModalTypes>,
}

impl Drawing {
    /// Initialize the drawing scene from the database.
    /// If the uuid is 0, then insert a new drawing in the database.
    fn init_online(self: &mut Self, globals: &mut Globals) -> Command<Message> {
        let mut uuid = *self.canvas.get_id();
        if uuid != Uuid::from_bytes([0; 16]) {
            if let Some(db) = globals.get_db() {
                Command::perform(
                    async move { database::drawing::get_drawing(&db, uuid).await },
                    move |res| match res {
                        Ok((layers, tools)) => CanvasMessage::Loaded {
                            layers,
                            tools,
                            json_tools: None,
                        }
                        .into(),
                        Err(err) => Message::Error(err),
                    },
                )
            } else {
                Command::none()
            }
        } else {
            uuid = Uuid::new();
            self.canvas.set_id(uuid.clone());

            if let Some(db) = globals.get_db() {
                let user_id = globals.get_user().unwrap().get_id();

                Command::batch(vec![
                    Command::perform(
                        async move {
                            let document = SVG::new(&vec![Uuid::new()]).as_document();

                            let webp = utils::encoder::encode_svg(document, "webp").await?;

                            database::base::upload_file(format!("/{}/{}.webp", user_id, uuid), webp)
                                .await
                        },
                        |result| match result {
                            Ok(_) => Message::None,
                            Err(err) => Message::Error(err),
                        },
                    ),
                    Command::perform(
                        async move { database::drawing::create_drawing(&db, uuid, user_id).await },
                        move |result| match result {
                            Ok(layer) => CanvasMessage::Loaded {
                                layers: vec![layer],
                                tools: vec![],
                                json_tools: None,
                            }
                            .into(),
                            Err(err) => Message::Error(err),
                        },
                    ),
                ])
            } else {
                Command::none()
            }
        }
    }

    /// Initialize the drawing scene from the user's computer.
    /// If the uuid is 0, then create a new directory.
    fn init_offline(self: &mut Self, globals: &mut Globals) -> Command<Message> {
        let default_id = Uuid::new();
        let mut default_layer = Object::new();
        default_layer.insert("id", JsonValue::String(default_id.to_string()));
        default_layer.insert("name", JsonValue::String("New layer".into()));

        let mut default_json = Object::new();
        default_json.insert(
            "layers",
            JsonValue::Array(vec![JsonValue::Object(default_layer)]),
        );
        default_json.insert("tools", JsonValue::Array(vec![]));
        default_json.insert("name", JsonValue::String(String::from("New drawing")));

        let mut uuid = *self.canvas.get_id();
        if uuid != Uuid::from_bytes([0; 16]) {
            Command::perform(
                async move { services::drawing::get_drawing_offline(uuid).await },
                |result| match result {
                    Ok((layers, tools, json_tools)) => CanvasMessage::Loaded {
                        layers,
                        tools,
                        json_tools: Some(json_tools),
                    }
                    .into(),
                    Err(err) => Message::Error(err),
                },
            )
        } else {
            uuid = Uuid::new();
            self.canvas.set_id(uuid.clone());

            Command::batch(vec![
                Command::perform(
                    async move { services::drawing::create_drawing_offline(uuid, default_json).await },
                    |result| match result {
                        Ok(_) => Message::None,
                        Err(err) => Message::Error(err),
                    },
                ),
                self.update(
                    globals,
                    &CanvasMessage::Loaded {
                        layers: vec![(default_id, "New layer".to_string())],
                        tools: vec![],
                        json_tools: Some(vec![]),
                    }
                    .into(),
                ),
            ])
        }
    }

    fn handle_canvas_message(
        &mut self,
        message: &CanvasMessage,
        globals: &mut Globals,
    ) -> Command<Message> {
        let mut commands = vec![];

        match message {
            CanvasMessage::Save | CanvasMessage::Saved => commands.push(self.update(
                globals,
                &DrawingMessage::ToggleModal(ModalTypes::WaitScreen(String::from("Saving..."))),
            )),
            _ => {}
        }

        commands.push(self.canvas.update(globals, message.clone()));

        Command::batch(commands)
    }

    fn post_drawing(&mut self, globals: &mut Globals) -> Command<Message> {
        let document = self.canvas.get_svg().as_document();
        let db = globals.get_db().unwrap();
        let user_id = globals.get_user().unwrap().get_id();
        let description = self.post_data.get_description().text();

        let tags: Vec<String> = self
            .post_data
            .get_post_tags()
            .iter()
            .map(|tag| tag.get_name().clone())
            .collect();

        self.post_data.set_post_tags(vec![]);
        self.post_data.set_description(Content::new());
        self.post_data.set_tag_input("");

        let close_modal_command = self.update(
            globals,
            &DrawingMessage::ToggleModal(ModalTypes::PostPrompt),
        );
        let wait_modal_command = self.update(
            globals,
            &DrawingMessage::ToggleModal(ModalTypes::WaitScreen(String::from(
                "Posting drawing...",
            ))),
        );

        Command::batch(vec![
            close_modal_command,
            wait_modal_command,
            Command::perform(
                async move {
                    services::drawing::create_post(user_id, &document, description, tags, &db).await
                },
                |res| match res {
                    Ok(_) => {
                        DrawingMessage::ToggleModal(ModalTypes::WaitScreen(String::from(""))).into()
                    }
                    Err(err) => Message::Error(err),
                },
            ),
        ])
    }

    fn save_as(&mut self, globals: &mut Globals) -> Command<Message> {
        let document = self.canvas.get_svg().as_document();

        let download = Command::perform(
            async move { services::drawing::download_drawing(&document).await },
            |result| match result {
                Ok(_) => Message::None,
                Err(err) => Message::Error(err),
            },
        );

        Command::batch(vec![
            self.update(globals, &CanvasMessage::Save.into()),
            download,
        ])
    }

    fn delete_drawing(&mut self, globals: &mut Globals) -> Command<Message> {
        let modal_command = self.update(
            globals,
            &DrawingMessage::ToggleModal(ModalTypes::WaitScreen(String::from(
                "Deleting drawing...",
            ))),
        );

        let is_offline = self.canvas.is_offline();
        let id = *self.canvas.get_id();
        let globals = globals.clone();

        Command::batch(vec![
            modal_command,
            Command::perform(
                async move {
                    if is_offline {
                        services::drawing::delete_drawing_offline(id).await
                    } else {
                        services::drawing::delete_drawing_online(id, &globals).await
                    }
                },
                |result| match result {
                    Ok(_) => Message::ChangeScene(Scenes::Main(None)),
                    Err(err) => Message::Error(err),
                },
            ),
        ])
    }

    fn toggle_modal(&mut self, modal: &ModalTypes, globals: &mut Globals) -> Command<Message> {
        self.modal_stack.toggle_modal(modal.clone());

        match modal {
            ModalTypes::PostPrompt => {
                if self.post_data.no_tags() {
                    if let (Some(_), Some(db)) = (globals.get_user(), globals.get_db()) {
                        Command::perform(
                            async move { database::drawing::get_tags(&db).await },
                            |res| match res {
                                Ok(tags) => {
                                    DrawingMessage::UpdatePostData(UpdatePostData::AllTags(tags))
                                        .into()
                                }
                                Err(err) => Message::Error(err),
                            },
                        )
                    } else {
                        Command::none()
                    }
                } else {
                    Command::none()
                }
            }
            _ => Command::none(),
        }
    }
}

/// The options of the [Drawing] scene.
#[derive(Debug, Clone)]
pub struct DrawingOptions {
    /// The id of the drawing.
    uuid: Option<Uuid>,

    /// The name of the drawing.
    name: Option<String>,

    /// The save mode of the drawing.
    save_mode: Option<SaveMode>,
}

impl DrawingOptions {
    /// Returns a new instance with the given parameters.
    pub fn new(uuid: Option<Uuid>, name: Option<String>, save_mode: Option<SaveMode>) -> Self {
        DrawingOptions {
            uuid,
            name,
            save_mode,
        }
    }
}

impl Scene for Drawing {
    type Message = DrawingMessage;
    type Options = DrawingOptions;

    fn new(options: Option<Self::Options>, globals: &mut Globals) -> (Self, Command<Message>)
    where
        Self: Sized,
    {
        let mut drawing = Drawing {
            canvas: Canvas::new()
                .width(Length::Fixed(800.0))
                .height(Length::Fixed(600.0)),
            post_data: Default::default(),
            save_mode: SaveMode::Online,
            modal_stack: ModalStack::new(),
        };

        let set_tool = Command::perform(async {}, |_| {
            CanvasMessage::ChangeTool(Box::new(LinePending::None)).into()
        });

        if let Some(options) = options {
            drawing.apply_options(options);
        }

        let init_data: Command<Message> = match drawing.save_mode {
            SaveMode::Online => drawing.init_online(globals),
            SaveMode::Offline => drawing.init_offline(globals),
        };

        return (drawing, Command::batch([set_tool, init_data]));
    }

    fn get_title(&self) -> String {
        self.canvas.get_name().clone()
    }

    fn apply_options(&mut self, options: Self::Options) {
        if let Some(uuid) = options.uuid {
            self.canvas.set_id(uuid);
        }

        if let Some(name) = options.name {
            self.canvas.set_name(name);
        } else {
            self.canvas.set_name("New drawing");
        }

        if let Some(save_mode) = options.save_mode {
            self.save_mode = save_mode;
        }
    }

    fn update(&mut self, globals: &mut Globals, message: &Self::Message) -> Command<Message> {
        match message {
            DrawingMessage::CanvasMessage(action) => self.handle_canvas_message(action, globals),
            DrawingMessage::UpdatePostData(update) => {
                self.post_data.update(update.clone());
                Command::none()
            }
            DrawingMessage::PostDrawing => self.post_drawing(globals),
            DrawingMessage::SaveAs => self.save_as(globals),
            DrawingMessage::DeleteDrawing => self.delete_drawing(globals),
            DrawingMessage::ToggleModal(modal) => self.toggle_modal(modal, globals),
            DrawingMessage::ErrorHandler(_) => Command::none(),
        }
    }

    fn view<'a>(&'a self, globals: &Globals) -> Element<'a, Message, Theme, Renderer> {
        let current_tool = self.canvas.get_current_tool().id();

        let tools_section = services::drawing::tools_section(current_tool);
        let style_section = services::drawing::style_section(&self.canvas);
        let layers_section = services::drawing::layers_section(&self.canvas);
        let menu_section = services::drawing::menu_section(globals);

        let underlay = services::drawing::underlay(
            &self.canvas,
            tools_section,
            style_section,
            layers_section,
            menu_section,
        );

        let modal_transform = |modal_type: ModalTypes| -> Element<Message, Theme, Renderer> {
            match modal_type {
                ModalTypes::PostPrompt => services::drawing::post_prompt(&self.post_data),
                ModalTypes::WaitScreen(message) => Container::new(WaitPanel::new(message))
                    .style(iced::widget::container::bordered_box)
                    .into(),
            }
        };

        self.modal_stack.get_modal(underlay, modal_transform)
    }

    fn handle_error(&mut self, globals: &mut Globals, error: &Error) -> Command<Message> {
        self.update(globals, &DrawingMessage::ErrorHandler(error.clone()))
    }

    fn clear(&self, _globals: &mut Globals) {}
}
