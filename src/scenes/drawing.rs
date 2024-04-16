use directories::ProjectDirs;
use std::any::Any;
use std::fs;
use std::fs::{create_dir_all, File};
use std::io::Write;

use crate::canvas::canvas::Canvas;
use iced::alignment::Horizontal;
use iced::widget::{Container, Row, Column, Text, Button, TextInput, Image, Scrollable, Space};
use iced::{Alignment, Command, Element, Length, Padding, Renderer};
use iced::widget::image::Handle;
use iced::widget::scrollable::{Direction, Properties};
use iced_aw::Badge;
use json::object::Object;
use json::JsonValue;
use mongodb::bson::Uuid;
use svg2webp::svg2webp;

use crate::canvas::layer::CanvasAction;
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
use crate::database;
use crate::errors::debug::DebugError;
use crate::errors::error::Error;
use crate::scene::{Action, Globals, Message, Scene, SceneOptions};
use crate::scenes::scenes::Scenes;

use crate::theme::Theme;

use crate::widgets::combo_box::ComboBox;
use crate::widgets::modal_stack::ModalStack;
use crate::widgets::card::Card;
use crate::widgets::closeable::Closeable;
use crate::widgets::grid::Grid;

use crate::scenes::data::drawing::*;

use crate::icons::{Icon, ICON, ToolIcon};

/// The [Messages](Action) for the [Drawing] scene.
#[derive(Clone)]
pub(crate) enum DrawingAction {
    /// Triggered when the user has interacted with the canvas.
    CanvasAction(CanvasAction),

    /// Creates a new post given the canvas and the [PostData].
    PostDrawing,

    /// Updates the [PostData] given the modified field.
    UpdatePostData(UpdatePostData),

    /// Toggles a [Modal](ModalTypes).
    ToggleModal(ModalTypes),

    /// Handles errors.
    ErrorHandler(Error),
}

impl Action for DrawingAction {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn get_name(&self) -> String {
        match self {
            DrawingAction::CanvasAction(_) => String::from("Canvas action"),
            DrawingAction::PostDrawing => String::from("Post drawing"),
            DrawingAction::UpdatePostData(_) => String::from("Update post data"),
            DrawingAction::ToggleModal(_) => String::from("Toggle modal"),
            DrawingAction::ErrorHandler(_) => String::from("Handle error"),
        }
    }

    fn boxed_clone(&self) -> Box<dyn Action + 'static> {
        Box::new((*self).clone())
    }
}

impl Into<Box<dyn Action + 'static>> for Box<DrawingAction> {
    fn into(self) -> Box<dyn Action + 'static> {
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
    /// Initialize the drawing scene from the database database.
    /// If the uuid is 0, then insert a new drawing in the database.
    fn init_online(self: &mut Box<Self>, globals: &mut Globals) -> Command<Message> {
        let mut uuid = *self.canvas.get_id();
        if uuid != Uuid::from_bytes([0; 16]) {
            if let Some(db) = globals.get_db() {
                Command::perform(
                    async move {
                        database::drawing::get_drawing(
                            &db,
                            uuid
                        ).await
                    },
                    move |res| {
                        match res {
                            Ok((layers, tools)) => {
                                Message::DoAction(Box::new(DrawingAction::CanvasAction(CanvasAction::Loaded {
                                    layers,
                                    tools,
                                    json_tools: None,
                                })))
                            }
                            Err(err) => Message::Error(err)
                        }
                    }
                )
            } else {
                Command::none()
            }
        } else {
            uuid = Uuid::new();
            self.canvas.set_id(uuid.clone());

            if let Some(db) = globals.get_db() {
                let user_id = globals.get_user().unwrap().get_id();

                Command::perform(
                    async move {
                        database::drawing::create_drawing(
                            &db,
                            uuid,
                            user_id
                        ).await
                    },
                    move |result| {
                        match result {
                            Ok(layer) => {
                                Message::DoAction(Box::new(DrawingAction::CanvasAction(CanvasAction::Loaded {
                                    layers: vec![layer],
                                    tools: vec![],
                                    json_tools: None,
                                })))
                            }
                            Err(err) => {
                                Message::Error(err)
                            }
                        }
                    }
                )
            } else {
                Command::none()
            }
        }
    }

    /// Initialize the drawing scene from the user's computer.
    /// If the uuid is 0, then create a new directory.
    fn init_offline(self: &mut Box<Self>, globals: &mut Globals) -> Command<Message> {
        let default_id = Uuid::new();
        let mut default_layer = Object::new();
        default_layer.insert("id", JsonValue::String(default_id.to_string()));
        default_layer.insert("name", JsonValue::String("New layer".into()));

        let mut default_json = Object::new();
        default_json.insert("layers", JsonValue::Array(vec![JsonValue::Object(default_layer)]));
        default_json.insert("tools", JsonValue::Array(vec![]));

        let mut uuid = *self.canvas.get_id();
        if uuid != Uuid::from_bytes([0; 16]) {
            Command::perform(
                async move {
                    let proj_dirs = ProjectDirs::from("", "CharMe", "Chartsy").unwrap();
                    let file_path = proj_dirs
                        .data_local_dir()
                        .join(String::from("./") + &*uuid.to_string() + "/data.json");
                    let data = fs::read_to_string(file_path).unwrap();

                    let data = json::parse(&*data).unwrap();

                    if let JsonValue::Object(data) = data.clone() {
                        let mut layers = vec![];
                        let mut tools = vec![];
                        let mut json_tools = vec![];

                        if let Some(JsonValue::Array(layer_array)) = data.get("layers") {
                            layers = layer_array.iter().filter_map(
                                |json| {
                                    if let JsonValue::Object(object) = json {
                                        Some((
                                            Uuid::parse_str(object.get("id").unwrap().as_str().unwrap()).unwrap(),
                                            object.get("name").unwrap().as_str().unwrap().to_string()
                                        ))
                                    } else {
                                        None
                                    }
                                }
                            ).collect();
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

                        (layers, tools, json_tools)
                    } else {
                        (vec![], vec![], vec![])
                    }
                },
                |(layers, tools, json_tools)| {
                    Message::DoAction(Box::new(DrawingAction::CanvasAction(
                        CanvasAction::Loaded {
                            layers,
                            tools,
                            json_tools: Some(json_tools),
                        },
                    )))
                },
            )
        } else {
            uuid = Uuid::new();
            self.canvas.set_id(uuid.clone());

            let proj_dirs = ProjectDirs::from("", "CharMe", "Chartsy").unwrap();
            let dir_path = proj_dirs
                .data_local_dir()
                .join(String::from("./") + &*uuid.to_string());
            create_dir_all(dir_path.clone()).unwrap();

            let file_path = dir_path.join("./data.json");
            let mut file = File::create(file_path).unwrap();
            file.write(json::stringify(JsonValue::Object(default_json)).as_bytes())
                .unwrap();

            self.update(globals, Box::new(DrawingAction::CanvasAction(
                CanvasAction::Loaded {
                    layers: vec![(default_id, "New layer".to_string())],
                    tools: vec![],
                    json_tools: Some(vec![]),
                },
            )))
        }
    }
}

/// The options of the [Drawing] scene.
#[derive(Debug, Clone, Copy)]
pub struct DrawingOptions {
    /// The id of the drawing.
    uuid: Option<Uuid>,

    /// The save mode of the drawing.
    save_mode: Option<SaveMode>,
}

impl DrawingOptions {
    /// Returns a new instance with the given parameters.
    pub(crate) fn new(uuid: Option<Uuid>, save_mode: Option<SaveMode>) -> Self {
        DrawingOptions { uuid, save_mode }
    }
}

impl SceneOptions<Box<Drawing>> for DrawingOptions {
    fn apply_options(&self, scene: &mut Box<Drawing>) {
        if let Some(uuid) = self.uuid {
            scene.canvas.set_id(uuid);
        }

        if let Some(save_mode) = self.save_mode {
            scene.save_mode = save_mode;
        }
    }

    fn boxed_clone(&self) -> Box<dyn SceneOptions<Box<Drawing>>> {
        Box::new((*self).clone())
    }
}

impl Scene for Box<Drawing> {
    fn new(
        options: Option<Box<dyn SceneOptions<Box<Drawing>>>>,
        globals: &mut Globals,
    ) -> (Self, Command<Message>)
    where
        Self: Sized,
    {
        let mut drawing = Box::new(Drawing {
            canvas: Canvas::new()
                .width(Length::Fixed(800.0))
                .height(Length::Fixed(600.0)),
            post_data: Default::default(),
            save_mode: SaveMode::Online,
            modal_stack: ModalStack::new(),
        });

        let set_tool = Command::perform(async {}, |_| {
            Message::DoAction(Box::new(DrawingAction::CanvasAction(
                CanvasAction::ChangeTool(Box::new(LinePending::None)),
            )))
        });

        if let Some(options) = options {
            options.apply_options(&mut drawing);
        }

        let init_data: Command<Message> = match drawing.save_mode {
            SaveMode::Online => drawing.init_online(globals),
            SaveMode::Offline => drawing.init_offline(globals),
        };

        return (drawing, Command::batch([set_tool, init_data]));
    }

    fn get_title(&self) -> String {
        String::from("Drawing")
    }

    fn update(&mut self, globals: &mut Globals, message: Box<dyn Action>) -> Command<Message> {
        let as_option: Option<&DrawingAction> = message
            .as_any()
            .downcast_ref::<DrawingAction>();
        let message = if let Some(message) = as_option {
            message
        } else {
            return Command::perform(async {}, move |()| Message::Error(
                Error::DebugError(DebugError::new(
                    format!("Message doesn't belong to drawing scene: {}.", message.get_name())
                ))
            ))
        };

        match message {
            DrawingAction::CanvasAction(action) => self.canvas.update(globals, action.clone()),
            DrawingAction::UpdatePostData(update) => {
                self.post_data.update(update.clone());
                Command::none()
            }
            DrawingAction::PostDrawing => {
                let document = self.canvas.get_svg().as_document();
                let db = globals.get_db().unwrap();
                let user_id = globals.get_user().unwrap().get_id();
                let description = self.post_data.get_description().clone();

                let tags :Vec<String>= self.post_data.get_post_tags().iter().map(
                    |tag| tag.get_name().clone()
                ).collect();

                self.post_data.set_post_tags(vec![]);
                self.post_data.set_description("");
                self.post_data.set_tag_input("");

                Command::perform(
                    async move {
                        let buffer = document.to_string();
                        let img = svg2webp(&*buffer, 80.0).unwrap();
                        let post_id = Uuid::new();

                        match database::base::upload_file(
                            format!("/{}/{}.webp", user_id, post_id),
                            img
                        ).await {
                            Ok(()) => { }
                            Err(err) => {
                                return Err(err);
                            }
                        }

                        database::drawing::create_post(
                            &db,
                            post_id,
                            user_id,
                            description,
                            tags
                        ).await
                    },
                    |res| {
                        match res {
                            Ok(_) => Message::DoAction(Box::new(
                                DrawingAction::ToggleModal(ModalTypes::PostPrompt)
                            )),
                            Err(err) => Message::Error(err)
                        }
                    },
                )
            }
            DrawingAction::ToggleModal(modal) => {
                self.modal_stack.toggle_modal(modal.clone());

                match modal {
                    ModalTypes::PostPrompt => {
                        if self.post_data.no_tags() {
                            if let (Some(_), Some(db)) = (globals.get_user(), globals.get_db()) {
                                Command::perform(
                                    async move {
                                        database::drawing::get_tags(&db).await
                                    },
                                    |res| {
                                        match res {
                                            Ok(tags) => {
                                                Message::DoAction(Box::new(DrawingAction::UpdatePostData(
                                                    UpdatePostData::AllTags(tags)
                                                )))
                                            }
                                            Err(err) => Message::Error(err)
                                        }
                                    }
                                )
                            } else {
                                Command::none()
                            }
                        } else {
                            Command::none()
                        }
                    }
                }
            }
            DrawingAction::ErrorHandler(_) => Command::none(),
        }
    }

    fn view<'a>(&'a self, globals: &Globals) -> Element<'a, Message, Theme, Renderer> {
        let tool_button :fn(String, Box<dyn Pending>) -> Element<'a, Message, Theme, Renderer>=
            |name, pending| {
                Button::<Message, Theme, Renderer>::new(
                    Text::new(name).font(ICON).line_height(1.0).size(25.0)
                )
                    .on_press(Message::DoAction(Box::new(DrawingAction::CanvasAction(
                        CanvasAction::ChangeTool(pending)
                    ))))
                    .padding(10.0)
                    .into()
            };

        let geometry_section :Element<Message, Theme, Renderer>= Grid::new(vec![
            tool_button(ToolIcon::Line.to_string(), Box::new(LinePending::None)),
            tool_button(ToolIcon::Rectangle.to_string(), Box::new(RectPending::None)),
            tool_button(ToolIcon::Triangle.to_string(), Box::new(TrianglePending::None)),
            tool_button(ToolIcon::Polygon.to_string(), Box::new(PolygonPending::None)),
            tool_button(ToolIcon::Circle.to_string(), Box::new(CirclePending::None)),
            tool_button(ToolIcon::Ellipse.to_string(), Box::new(EllipsePending::None)),
        ])
            .spacing(25.0)
            .padding(18.0)
            .into();

        let brushes_section :Element<Message, Theme, Renderer>= Grid::new(vec![
            tool_button(ToolIcon::Pencil.to_string(), Box::new(BrushPending::<Pencil>::None)),
            tool_button(ToolIcon::FountainPen.to_string(), Box::new(BrushPending::<Pen>::None)),
            tool_button(ToolIcon::Airbrush.to_string(), Box::new(BrushPending::<Airbrush>::None))
        ])
            .spacing(25.0)
            .padding(18.0)
            .into();

        let eraser_section :Element<Message, Theme, Renderer>= Grid::new(vec![
            tool_button(ToolIcon::Eraser.to_string(), Box::new(BrushPending::<Eraser>::None))
        ])
            .spacing(25.0)
            .padding(18.0)
            .into();

        let tools_section = Container::new(Scrollable::new(
            Column::with_children(
                vec![
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
                ]
            )
                .padding(8.0)
                .spacing(15.0)
                .width(Length::Fill)
        ))
            .padding(2.0)
            .width(Length::Fill)
            .style(crate::theme::container::Container::Bordered)
            .height(Length::FillPortion(1));

        let style_section = Container::new(Scrollable::new(
            self.canvas
                .get_style()
                .view()
                .map(|update| Message::DoAction(Box::new(
                    DrawingAction::CanvasAction(CanvasAction::UpdateStyle(update))
                )))
        ))
            .padding(2.0)
            .width(Length::Fill)
            .style(crate::theme::container::Container::Bordered)
            .height(Length::FillPortion(1));

        let layers_section = Container::new(Scrollable::new(
            Column::with_children(vec![
                Row::with_children(vec![
                    Text::new("Layers")
                        .size(20.0)
                        .width(Length::Fill)
                        .into(),
                    Button::new(
                        Text::new(Icon::Add.to_string())
                            .size(20.0)
                            .font(ICON)
                    )
                        .padding(0.0)
                        .style(crate::theme::button::Button::Transparent)
                        .on_press(Message::DoAction(Box::new(DrawingAction::CanvasAction(
                            CanvasAction::AddLayer
                        ))))
                        .into()
                ])
                    .padding(8.0)
                    .width(Length::Fill)
                    .into(),
                Column::with_children(
                    self.canvas.get_layer_order().iter().map(
                        |id| {
                            let style = if *id == *self.canvas.get_current_layer() {
                                crate::theme::button::Button::SelectedLayer
                            } else {
                                crate::theme::button::Button::UnselectedLayer
                            };
                            let text_style = || if *id == *self.canvas.get_current_layer() {
                                crate::theme::text::Text::Dark
                            } else {
                                crate::theme::text::Text::Light
                            };

                            let layer = &self.canvas.get_layers().get(id).unwrap();
                            let layer_count = self.canvas.get_layers().len();

                            Button::new(
                                Row::with_children(vec![
                                    if let Some(new_name) = layer.get_new_name() {
                                        TextInput::new(
                                            "Write layer name...",
                                            &*new_name.clone()
                                        )
                                            .on_input(|input|
                                                Message::DoAction(Box::new(DrawingAction::CanvasAction(
                                                    CanvasAction::UpdateLayerName(*id, input)
                                                )))
                                            )
                                            .on_submit(Message::DoAction(Box::new(DrawingAction::CanvasAction(
                                                CanvasAction::ToggleEditLayerName(*id)
                                            ))))
                                            .into()
                                    } else {
                                        Row::with_children(vec![
                                            Text::new(layer.get_name().clone())
                                                .width(Length::Fill)
                                                .into(),
                                            Button::new(
                                                Text::new(Icon::Edit.to_string()).font(ICON)
                                                    .style(text_style())
                                            )
                                                .style(crate::theme::button::Button::Transparent)
                                                .on_press(Message::DoAction(Box::new(DrawingAction::CanvasAction(
                                                    CanvasAction::ToggleEditLayerName(*id)
                                                ))))
                                                .into()
                                        ])
                                            .align_items(Alignment::Center)
                                            .into()
                                    },
                                    Button::new(
                                        Text::new(
                                            if layer.is_visible() { Icon::Visible } else { Icon::Hidden }
                                                .to_string()
                                        )
                                            .style(text_style())
                                            .font(ICON)
                                    )
                                        .style(crate::theme::button::Button::Transparent)
                                        .on_press(Message::DoAction(Box::new(DrawingAction::CanvasAction(
                                            CanvasAction::ToggleLayer(*id)
                                        ))))
                                        .into(),
                                    if layer_count > 1 {
                                        Button::new(
                                            Text::new(Icon::X.to_string()).font(ICON)
                                                .style(text_style())
                                        )
                                            .style(crate::theme::button::Button::Transparent)
                                            .on_press(Message::DoAction(Box::new(DrawingAction::CanvasAction(
                                                CanvasAction::RemoveLayer(*id)
                                            ))))
                                            .into()
                                    } else {
                                        Space::with_width(Length::Shrink)
                                            .into()
                                    }
                                ])
                                    .align_items(Alignment::Center)
                            )
                                .width(Length::Fill)
                                .style(style)
                                .on_press(Message::DoAction(Box::new(
                                    DrawingAction::CanvasAction(CanvasAction::ActivateLayer(*id))
                                )))
                                .into()
                        }
                    ).collect::<Vec<Element<Message, Theme, Renderer>>>()
                )
                    .padding(8.0)
                    .spacing(5.0)
                    .into()
            ])
        ))
            .padding(2.0)
            .width(Length::Fill)
            .style(crate::theme::container::Container::Bordered)
            .height(Length::FillPortion(1));

        let menu_section = Container::new(
            Column::with_children(vec![
                Button::new(Text::new("Save"))
                    .on_press(Message::DoAction(Box::new(DrawingAction::CanvasAction(CanvasAction::Save))))
                    .into(),
                if globals.get_db().is_some() && globals.get_user().is_some() {
                    Button::new(Text::new("Post"))
                        .on_press(Message::DoAction(Box::new(DrawingAction::ToggleModal(ModalTypes::PostPrompt))))
                } else {
                    Button::new(Text::new("Post"))
                }
                    .into(),
                Button::new(Text::new("Back"))
                    .on_press(Message::ChangeScene(Scenes::Main(None)))
                    .into(),
            ])
                .spacing(8.0)
                .align_items(Alignment::Center)
        )
            .padding(10.0)
            .style(crate::theme::container::Container::Bordered)
            .align_x(Horizontal::Center)
            .width(Length::Fill)
            .height(Length::FillPortion(1));

        let underlay = Row::with_children(
            vec![
                Column::with_children(vec![
                    tools_section.into(),
                    style_section.into()
                ])
                    .width(Length::Fixed(250.0))
                    .height(Length::Fill)
                    .into(),
                Column::with_children(vec![
                    Text::new(format!("{}", self.get_title()))
                        .width(Length::Shrink)
                        .size(50)
                        .into(),
                    Container::new::<Scrollable<Message, Theme, Renderer>>(
                        Scrollable::new(&self.canvas)
                            .direction(Direction::Both {
                                vertical: Properties::default(),
                                horizontal: Properties::default()
                            })
                    )
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .center_x()
                        .center_y()
                        .into(),
                ])
                    .height(Length::Fill)
                    .into(),
                Column::with_children(vec![
                    layers_section.into(),
                    menu_section.into()
                ])
                    .align_items(Alignment::Center)
                    .width(Length::Fixed(250.0))
                    .height(Length::Fill)
                    .into()
            ]
        )
            .padding(0)
            .spacing(20)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_items(Alignment::Center)
            .into();

        let modal_transform = |modal_type: ModalTypes| -> Element<Message, Theme, Renderer> {
            match modal_type {
                ModalTypes::PostPrompt => {
                    Closeable::new(
                        Card::new(
                            Text::new("Create a new post"),
                            Column::with_children(
                                vec![
                                    Text::new("Description:").into(),
                                    TextInput::new(
                                        "Write description here...",
                                        &*self.post_data.get_description()
                                    )
                                        .on_input(|new_value| Message::DoAction(Box::new(DrawingAction::UpdatePostData(UpdatePostData::Description(new_value)))))
                                        .into(),
                                    Text::new("Tags:").into(),
                                    Grid::new(self.post_data.get_post_tags().iter().map(
                                        |tag| Badge::new(
                                            Text::new(tag.get_name().clone())
                                        )
                                            .padding(3)
                                    ).collect())
                                        .padding(Padding::from([5.0, 0.0, 5.0, 0.0]))
                                        .spacing(5.0)
                                        .into(),
                                    Row::with_children(
                                        vec![
                                            ComboBox::new(
                                                self.post_data.get_all_tags().clone(),
                                                "Add a new tag...",
                                                &*self.post_data.get_tag_input(),
                                                |tag| Message::DoAction(Box::new(
                                                    DrawingAction::UpdatePostData(UpdatePostData::SelectedTag(tag))
                                                ))
                                            )
                                                .width(Length::Fill)
                                                .on_input(|new_value| Message::DoAction(Box::new(
                                                    DrawingAction::UpdatePostData(UpdatePostData::TagInput(new_value))
                                                )))
                                                .into(),
                                            Button::new(
                                                Image::new(Handle::from_memory(
                                                    fs::read("src/images/add.png").unwrap()
                                                ))
                                                    .width(30.0)
                                                    .height(30.0)
                                            )
                                                .on_press(Message::DoAction(Box::new(DrawingAction::UpdatePostData(
                                                    UpdatePostData::NewTag(self.post_data.get_tag_input().clone())
                                                ))))
                                                .padding(0)
                                                .into()
                                        ]
                                    )
                                        .spacing(10)
                                        .into()
                                ]
                            )
                                .height(Length::Shrink)
                        )
                            .footer(
                                Button::new("Post")
                                    .on_press(Message::DoAction(Box::new(DrawingAction::PostDrawing)))
                            )
                            .width(Length::Fixed(300.0))
                    )
                        .style(crate::theme::closeable::Closeable::Transparent)
                        .on_close(
                            Message::DoAction(Box::new(DrawingAction::ToggleModal(ModalTypes::PostPrompt))),
                            32.0
                        )
                        .width(Length::Shrink)
                        .height(Length::Shrink)
                        .into()
                }
            }
        };

        self.modal_stack.get_modal(underlay, modal_transform)
    }

    fn get_error_handler(&self, error: Error) -> Box<dyn Action> {
        Box::new(DrawingAction::ErrorHandler(error))
    }

    fn clear(&self, _globals: &mut Globals) {}
}
