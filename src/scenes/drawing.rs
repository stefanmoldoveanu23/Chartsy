use directories::ProjectDirs;
use std::any::Any;
use std::fs;
use std::fs::{create_dir_all, File};
use std::io::Write;

use crate::canvas::canvas::Canvas;
use crate::canvas::svg::SVG;
use crate::widgets::{Card, Close, Closeable, ComboBox, Grid, ModalStack, WaitPanel};
use iced::alignment::{Horizontal, Vertical};
use iced::widget::image::Handle;
use iced::widget::scrollable::{Direction, Properties};
use iced::widget::{Button, Column, Container, Image, Row, Scrollable, Space, Text, TextInput};
use iced::{Alignment, Command, Element, Length, Padding, Renderer};
use json::object::Object;
use json::JsonValue;
use mongodb::bson::Uuid;
use rfd::AsyncFileDialog;

use crate::canvas::layer::CanvasMessage;
use crate::canvas::tool;
use crate::canvas::tool::Pending;
use crate::canvas::tools::{
    brush::BrushPending,
    brushes::{airbrush::Airbrush, eraser::Eraser, pen::Pen, pencil::Pencil},
};
use crate::canvas::tools::{
    circle::CirclePending, ellipse::EllipsePending, line::LinePending, polygon::PolygonPending,
    rect::RectPending, triangle::TrianglePending,
};
use crate::errors::error::Error;
use crate::scene::{Globals, Message, Scene, SceneMessage};
use crate::{database, debug_message, services, utils};

use crate::utils::theme::{self, Theme};

use crate::scenes::data::drawing::*;
use crate::utils::encoder::encode_svg;

use crate::utils::icons::{Icon, ToolIcon, ICON};

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
                async move {
                    let proj_dirs = ProjectDirs::from("", "CharMe", "Chartsy")
                        .ok_or(debug_message!("Unable to find project directory.").into())?;
                    let file_path = proj_dirs
                        .data_local_dir()
                        .join(String::from("./") + &*uuid.to_string() + "/data.json");
                    let data = fs::read_to_string(file_path)
                        .map_err(|err| debug_message!("{}", err).into())?;

                    let data =
                        json::parse(&*data).map_err(|err| debug_message!("{}", err).into())?;

                    if let JsonValue::Object(data) = data.clone() {
                        let mut layers = vec![];
                        let mut tools = vec![];
                        let mut json_tools = vec![];

                        if let Some(JsonValue::Array(layer_array)) = data.get("layers") {
                            layers = layer_array
                                .iter()
                                .filter_map(|json| {
                                    if let JsonValue::Object(object) = json {
                                        Some((
                                            Uuid::parse_str(
                                                object.get("id").unwrap().as_str().unwrap(),
                                            )
                                            .unwrap(),
                                            object
                                                .get("name")
                                                .unwrap()
                                                .as_str()
                                                .unwrap()
                                                .to_string(),
                                        ))
                                    } else {
                                        None
                                    }
                                })
                                .collect();
                        }
                        if let Some(JsonValue::Array(tool_list)) = data.get("tools") {
                            json_tools = tool_list.clone();

                            for tool in tool_list {
                                if let JsonValue::Object(tool) = tool {
                                    if let Some(tool) = tool::get_json(tool) {
                                        tools.push(tool);
                                    }
                                }
                            }
                        }

                        Ok((layers, tools, json_tools))
                    } else {
                        Ok((vec![], vec![], vec![]))
                    }
                },
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
                    async move {
                        let proj_dirs = ProjectDirs::from("", "CharMe", "Chartsy")
                            .ok_or(debug_message!("Unable to find project directory.").into())?;

                        let drawings_path = proj_dirs.data_local_dir().join("drawings.json");
                        let drawings = json::parse(
                            &*fs::read_to_string(drawings_path.clone())
                                .map_err(|err| debug_message!("{}", err).into())?,
                        )
                        .map_err(|err| debug_message!("{}", err).into())?;

                        if let JsonValue::Array(mut drawings) = drawings {
                            let mut drawing = Object::new();
                            drawing.insert("id", JsonValue::String(uuid.to_string()));
                            drawing.insert("name", JsonValue::String(String::from("New drawing")));

                            drawings.push(JsonValue::Object(drawing));

                            fs::write(drawings_path, json::stringify(JsonValue::Array(drawings)))
                                .map_err(|err| debug_message!("{}", err).into())?;
                        }

                        let dir_path = proj_dirs
                            .data_local_dir()
                            .join(String::from("./") + &*uuid.to_string());
                        create_dir_all(dir_path.clone())
                            .map_err(|err| debug_message!("{}", err).into())?;

                        let drawing_path = dir_path.join("data.webp");

                        let file_path = dir_path.join("data.json");
                        let mut file = File::create(file_path)
                            .map_err(|err| debug_message!("{}", err).into())?;
                        file.write(json::stringify(JsonValue::Object(default_json)).as_bytes())
                            .map_err(|err| debug_message!("{}", err).into())?;

                        let svg = SVG::new(&vec![Uuid::new()]).as_document();
                        let webp = utils::encoder::encode_svg(svg, "webp").await?;

                        tokio::fs::write(drawing_path, webp)
                            .await
                            .map_err(|err| debug_message!("{}", err).into())
                    },
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
        }

        if let Some(save_mode) = options.save_mode {
            self.save_mode = save_mode;
        }
    }

    fn update(&mut self, globals: &mut Globals, message: &Self::Message) -> Command<Message> {
        match message {
            DrawingMessage::CanvasMessage(action) => {
                let mut commands = vec![];

                match action {
                    CanvasMessage::Save | CanvasMessage::Saved => commands.push(self.update(
                        globals,
                        &DrawingMessage::ToggleModal(ModalTypes::WaitScreen(String::from(
                            "Saving...",
                        ))),
                    )),
                    _ => {}
                }

                commands.push(self.canvas.update(globals, action.clone()));

                Command::batch(commands)
            }
            DrawingMessage::UpdatePostData(update) => {
                self.post_data.update(update.clone());
                Command::none()
            }
            DrawingMessage::PostDrawing => {
                let document = self.canvas.get_svg().as_document();
                let db = globals.get_db().unwrap();
                let user_id = globals.get_user().unwrap().get_id();
                let description = self.post_data.get_description().clone();

                let tags: Vec<String> = self
                    .post_data
                    .get_post_tags()
                    .iter()
                    .map(|tag| tag.get_name().clone())
                    .collect();

                self.post_data.set_post_tags(vec![]);
                self.post_data.set_description("");
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
                            let img = encode_svg(document, "webp").await?;
                            let post_id = Uuid::new();

                            match database::base::upload_file(
                                format!("/{}/{}.webp", user_id, post_id),
                                img,
                            )
                            .await
                            {
                                Ok(()) => {}
                                Err(err) => {
                                    return Err(err);
                                }
                            }

                            database::drawing::create_post(&db, post_id, user_id, description, tags)
                                .await
                        },
                        |res| match res {
                            Ok(_) => DrawingMessage::ToggleModal(ModalTypes::WaitScreen(
                                String::from(""),
                            ))
                            .into(),
                            Err(err) => Message::Error(err),
                        },
                    ),
                ])
            }
            DrawingMessage::SaveAs => {
                let document = self.canvas.get_svg().as_document();

                let download = Command::perform(
                    async move {
                        let file = AsyncFileDialog::new()
                            .set_title("Save As...")
                            .set_directory("~")
                            .add_filter(
                                "image",
                                &["png", "jpg", "jpeg", "webp", "svg", "tiff", "bmp"],
                            )
                            .save_file()
                            .await;

                        match file {
                            Some(handle) => {
                                let name = handle.file_name().to_string();
                                let format = name.split(".").last().ok_or(
                                    debug_message!("File needs to have a readable format.").into(),
                                )?;
                                let img = encode_svg(document, &*format).await?;

                                handle
                                    .write(img.as_slice())
                                    .await
                                    .map_err(|err| err.to_string().into())
                            }
                            None => Err(debug_message!("Error getting file.").into()),
                        }
                    },
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
            DrawingMessage::DeleteDrawing => {
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
                                services::drawings::delete_drawing_offline(id).await
                            } else {
                                services::drawings::delete_drawing_online(id, &globals).await
                            }
                        },
                        |result| match result {
                            Ok(_) => Message::ChangeScene(Scenes::Main(None)),
                            Err(err) => Message::Error(err),
                        },
                    ),
                ])
            }
            DrawingMessage::ToggleModal(modal) => {
                self.modal_stack.toggle_modal(modal.clone());

                match modal {
                    ModalTypes::PostPrompt => {
                        if self.post_data.no_tags() {
                            if let (Some(_), Some(db)) = (globals.get_user(), globals.get_db()) {
                                Command::perform(
                                    async move { database::drawing::get_tags(&db).await },
                                    |res| match res {
                                        Ok(tags) => DrawingMessage::UpdatePostData(
                                            UpdatePostData::AllTags(tags),
                                        )
                                        .into(),
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
            DrawingMessage::ErrorHandler(_) => Command::none(),
        }
    }

    fn view<'a>(&'a self, globals: &Globals) -> Element<'a, Message, Theme, Renderer> {
        let current_tool = self.canvas.get_current_tool().id();

        let tool_button =
            |name, pending: Box<dyn Pending>| -> Element<'a, Message, Theme, Renderer> {
                let style = if current_tool == pending.id() {
                    iced::widget::button::primary
                } else {
                    iced::widget::button::secondary
                };

                Button::<Message, Theme, Renderer>::new(
                    Text::new(name).font(ICON).line_height(1.0).size(25.0), //.style(text_style),
                )
                .style(style)
                .on_press(CanvasMessage::ChangeTool(pending).into())
                .padding(10.0)
                .into()
            };

        let geometry_section: Element<Message, Theme, Renderer> = Grid::new(vec![
            tool_button(ToolIcon::Line.to_string(), Box::new(LinePending::None)),
            tool_button(ToolIcon::Rectangle.to_string(), Box::new(RectPending::None)),
            tool_button(
                ToolIcon::Triangle.to_string(),
                Box::new(TrianglePending::None),
            ),
            tool_button(
                ToolIcon::Polygon.to_string(),
                Box::new(PolygonPending::None),
            ),
            tool_button(ToolIcon::Circle.to_string(), Box::new(CirclePending::None)),
            tool_button(
                ToolIcon::Ellipse.to_string(),
                Box::new(EllipsePending::None),
            ),
        ])
        .spacing(25.0)
        .padding(18.0)
        .into();

        let brushes_section: Element<Message, Theme, Renderer> = Grid::new(vec![
            tool_button(
                ToolIcon::Pencil.to_string(),
                Box::new(BrushPending::<Pencil>::None),
            ),
            tool_button(
                ToolIcon::FountainPen.to_string(),
                Box::new(BrushPending::<Pen>::None),
            ),
            tool_button(
                ToolIcon::Airbrush.to_string(),
                Box::new(BrushPending::<Airbrush>::None),
            ),
        ])
        .spacing(25.0)
        .padding(18.0)
        .into();

        let eraser_section: Element<Message, Theme, Renderer> = Grid::new(vec![tool_button(
            ToolIcon::Eraser.to_string(),
            Box::new(BrushPending::<Eraser>::None),
        )])
        .spacing(25.0)
        .padding(18.0)
        .into();

        let tools_section = Container::new(Scrollable::new(
            Column::with_children(vec![
                Text::new("Geometry")
                    .horizontal_alignment(Horizontal::Center)
                    .size(20.0)
                    .into(),
                geometry_section,
                Text::new("Brushes")
                    .horizontal_alignment(Horizontal::Center)
                    .size(20.0)
                    .into(),
                brushes_section,
                Text::new("Eraser")
                    .horizontal_alignment(Horizontal::Center)
                    .size(20.0)
                    .into(),
                eraser_section,
            ])
            .padding(8.0)
            .spacing(15.0)
            .width(Length::Fill),
        ))
        .padding(2.0)
        .width(Length::Fill)
        .style(iced::widget::container::bordered_box)
        .height(Length::FillPortion(1));

        let style_section = Container::new(Scrollable::new(
            self.canvas
                .get_style()
                .view()
                .map(|update| CanvasMessage::UpdateStyle(update).into()),
        ))
        .padding(2.0)
        .width(Length::Fill)
        .style(iced::widget::container::bordered_box)
        .height(Length::FillPortion(1));

        let layers_section = Container::new(Scrollable::new(Column::with_children(vec![
            Row::with_children(vec![
                Text::new("Layers").size(20.0).width(Length::Fill).into(),
                Button::new(Text::new(Icon::Add.to_string()).size(20.0).font(ICON))
                    .padding(0.0)
                    .style(iced::widget::button::text)
                    .on_press(CanvasMessage::AddLayer.into())
                    .into(),
            ])
            .padding(8.0)
            .width(Length::Fill)
            .into(),
            Column::with_children(
                self.canvas
                    .get_layer_order()
                    .iter()
                    .map(|id| {
                        let style = if *id == *self.canvas.get_current_layer() {
                            iced::widget::button::primary
                        } else {
                            iced::widget::button::secondary
                        };

                        let layer = &self.canvas.get_layers().get(id).unwrap();
                        let layer_count = self.canvas.get_layers().len();

                        Button::new(
                            Row::with_children(vec![
                                if let Some(new_name) = layer.get_new_name() {
                                    TextInput::new("Write layer name...", &*new_name.clone())
                                        .on_input(|input| {
                                            CanvasMessage::UpdateLayerName(*id, input).into()
                                        })
                                        .on_submit(CanvasMessage::ToggleEditLayerName(*id).into())
                                        .into()
                                } else {
                                    Row::with_children(vec![
                                        Text::new(layer.get_name().clone())
                                            .width(Length::Fill)
                                            .into(),
                                        Button::new(
                                            Text::new(Icon::Edit.to_string()).font(ICON), //.style(text_style()),
                                        )
                                        .style(iced::widget::button::text)
                                        .on_press(CanvasMessage::ToggleEditLayerName(*id).into())
                                        .into(),
                                    ])
                                    .align_items(Alignment::Center)
                                    .into()
                                },
                                Button::new(
                                    Text::new(
                                        if layer.is_visible() {
                                            Icon::Visible
                                        } else {
                                            Icon::Hidden
                                        }
                                        .to_string(),
                                    )
                                    //.style(text_style())
                                    .font(ICON),
                                )
                                .style(iced::widget::button::text)
                                .on_press(CanvasMessage::ToggleLayer(*id).into())
                                .into(),
                                if layer_count > 1 {
                                    Button::new(
                                        Text::new(Icon::X.to_string()).font(ICON), //.style(text_style()),
                                    )
                                    .style(iced::widget::button::text)
                                    .on_press(CanvasMessage::RemoveLayer(*id).into())
                                    .into()
                                } else {
                                    Space::with_width(Length::Shrink).into()
                                },
                            ])
                            .align_items(Alignment::Center),
                        )
                        .width(Length::Fill)
                        .style(style)
                        .on_press(CanvasMessage::ActivateLayer(*id).into())
                        .into()
                    })
                    .collect::<Vec<Element<Message, Theme, Renderer>>>(),
            )
            .padding(8.0)
            .spacing(5.0)
            .into(),
        ])))
        .padding(2.0)
        .width(Length::Fill)
        .style(iced::widget::container::bordered_box)
        .height(Length::FillPortion(1));

        let menu_section = Container::new(
            Column::with_children(vec![
                Space::with_height(Length::Fill).into(),
                Button::new(
                    Text::new("Save")
                        .horizontal_alignment(Horizontal::Center)
                        .width(Length::Fill)
                        .size(20.0),
                )
                .on_press(CanvasMessage::Save.into())
                .width(Length::Fill)
                .padding(5.0)
                .into(),
                Space::with_height(Length::Fill).into(),
                if globals.get_db().is_some() && globals.get_user().is_some() {
                    Button::new(
                        Text::new("Post")
                            .horizontal_alignment(Horizontal::Center)
                            .width(Length::Fill)
                            .size(20.0),
                    )
                    .on_press(DrawingMessage::ToggleModal(ModalTypes::PostPrompt).into())
                } else {
                    Button::new(
                        Text::new("Post")
                            .horizontal_alignment(Horizontal::Center)
                            .width(Length::Fill)
                            .size(20.0),
                    )
                }
                .padding(5.0)
                .width(Length::Fill)
                .into(),
                Space::with_height(Length::Fill).into(),
                Button::new(
                    Text::new("Save as...")
                        .horizontal_alignment(Horizontal::Center)
                        .width(Length::Fill)
                        .size(20.0),
                )
                .on_press(DrawingMessage::SaveAs.into())
                .padding(5.0)
                .width(Length::Fill)
                .into(),
                Space::with_height(Length::Fill).into(),
                Button::new(
                    Text::new("Delete")
                        .horizontal_alignment(Horizontal::Center)
                        .width(Length::Fill)
                        .style(theme::text::danger)
                        .size(20.0),
                )
                .on_press(DrawingMessage::DeleteDrawing.into())
                .padding(5.0)
                .width(Length::Fill)
                .into(),
                Space::with_height(Length::Fill).into(),
            ])
            .spacing(10.0)
            .align_items(Alignment::Center),
        )
        .padding(10.0)
        .style(iced::widget::container::bordered_box)
        .align_x(Horizontal::Center)
        .align_y(Vertical::Center)
        .width(Length::Fill)
        .height(Length::FillPortion(1));

        let underlay = Column::with_children(vec![
            Row::with_children(vec![
                Button::new(Text::new(Icon::Leave.to_string()).font(ICON).size(30.0))
                    .padding(0.0)
                    .style(iced::widget::button::text)
                    .on_press(Message::ChangeScene(Scenes::Main(None)))
                    .into(),
                if let Some(new_name) = self.canvas.get_new_name() {
                    TextInput::new("Add name", new_name)
                        .on_input(|value| CanvasMessage::SetNewName(value).into())
                        .on_submit(CanvasMessage::ToggleEditName.into())
                        .size(30.0)
                        .into()
                } else {
                    Text::new(self.canvas.get_name()).size(30.0).into()
                },
                if self.canvas.get_new_name().is_some() {
                    Space::with_width(Length::Shrink).into()
                } else {
                    Button::new(Text::new(Icon::Edit.to_string()).font(ICON).size(30.0))
                        .padding(0.0)
                        .style(iced::widget::button::text)
                        .on_press(CanvasMessage::ToggleEditName.into())
                        .into()
                },
            ])
            .spacing(10.0)
            .padding(10.0)
            .into(),
            Row::with_children(vec![
                Column::with_children(vec![tools_section.into(), style_section.into()])
                    .width(Length::Fixed(250.0))
                    .height(Length::Fill)
                    .into(),
                Container::new(Scrollable::with_direction(
                    &self.canvas,
                    Direction::Both {
                        vertical: Properties::default(),
                        horizontal: Properties::default(),
                    },
                ))
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .into(),
                Column::with_children(vec![layers_section.into(), menu_section.into()])
                    .align_items(Alignment::Center)
                    .width(Length::Fixed(250.0))
                    .height(Length::Fill)
                    .into(),
            ])
            .padding(0)
            .spacing(20)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_items(Alignment::Center)
            .into(),
        ]);

        let modal_transform = |modal_type: ModalTypes| -> Element<Message, Theme, Renderer> {
            match modal_type {
                ModalTypes::PostPrompt => Closeable::new(
                    Card::new(
                        Text::new("Create a new post"),
                        Column::with_children(vec![
                            Text::new("Description:").into(),
                            TextInput::new(
                                "Write description here...",
                                &*self.post_data.get_description(),
                            )
                            .on_input(|new_value| {
                                DrawingMessage::UpdatePostData(UpdatePostData::Description(
                                    new_value,
                                ))
                                .into()
                            })
                            .into(),
                            Text::new("Tags:").into(),
                            Grid::new(self.post_data.get_post_tags().iter().enumerate().map(
                                |(index, tag)| {
                                    Container::new(
                                        Row::with_children(vec![
                                            Text::new(tag.get_name().clone()).into(),
                                            Close::new(Into::<Message>::into(
                                                DrawingMessage::UpdatePostData(
                                                    UpdatePostData::RemoveTag(index),
                                                ),
                                            ))
                                            .size(15.0)
                                            .into(),
                                        ])
                                        .spacing(5.0)
                                        .align_items(Alignment::Center),
                                    )
                                    .style(theme::container::badge)
                                    .padding(10.0)
                                },
                            ))
                            .padding(Padding::from([5.0, 0.0, 5.0, 0.0]))
                            .spacing(5.0)
                            .into(),
                            Row::with_children(vec![
                                ComboBox::new(
                                    self.post_data.get_all_tags().clone(),
                                    "Add a new tag...",
                                    &*self.post_data.get_tag_input(),
                                    |tag| {
                                        DrawingMessage::UpdatePostData(UpdatePostData::SelectedTag(
                                            tag,
                                        ))
                                        .into()
                                    },
                                )
                                .width(Length::Fill)
                                .on_input(|new_value| {
                                    DrawingMessage::UpdatePostData(UpdatePostData::TagInput(
                                        new_value,
                                    ))
                                    .into()
                                })
                                .into(),
                                Button::new(
                                    Image::new(Handle::from_path("src/images/add.png"))
                                        .width(30.0)
                                        .height(30.0),
                                )
                                .on_press(
                                    DrawingMessage::UpdatePostData(UpdatePostData::NewTag(
                                        self.post_data.get_tag_input().clone(),
                                    ))
                                    .into(),
                                )
                                .padding(0)
                                .into(),
                            ])
                            .spacing(10)
                            .into(),
                        ])
                        .height(Length::Shrink),
                    )
                    .footer(Button::new("Post").on_press(DrawingMessage::PostDrawing.into()))
                    .width(Length::Fixed(300.0)),
                )
                .style(theme::closeable::Closeable::Transparent)
                .on_close(
                    Into::<Message>::into(DrawingMessage::ToggleModal(ModalTypes::PostPrompt)),
                    25.0,
                )
                .close_padding(7.0)
                .width(Length::Shrink)
                .height(Length::Shrink)
                .into(),
                ModalTypes::WaitScreen(message) => WaitPanel::new(message).into(),
            }
        };

        self.modal_stack.get_modal(underlay, modal_transform)
    }

    fn handle_error(&mut self, globals: &mut Globals, error: &Error) -> Command<Message> {
        self.update(globals, &DrawingMessage::ErrorHandler(error.clone()))
    }

    fn clear(&self, _globals: &mut Globals) {}
}
