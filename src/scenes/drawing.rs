use directories::ProjectDirs;
use dropbox_sdk::default_client::{NoauthDefaultClient, UserAuthDefaultClient};
use dropbox_sdk::files;
use dropbox_sdk::files::WriteMode;
use std::any::Any;
use std::fmt::{Display, Formatter};
use std::fs;
use std::fs::{create_dir_all, File};
use std::io::Write;
use std::sync::Arc;

use crate::canvas::canvas::Canvas;
use iced::alignment::Horizontal;
use iced::widget::{Container, Row, Column, Text, Button, TextInput, Image};
use iced::{Alignment, Command, Element, Length, Padding, Renderer};
use iced::widget::image::Handle;
use iced_aw::Badge;
use iced_aw::tab_bar::TabLabel;
use iced_aw::tabs::Tabs;
use json::object::Object;
use json::JsonValue;
use mongodb::bson::{doc, Bson, Uuid, Document};
use svg2webp::svg2webp;

use crate::canvas::layer::CanvasAction;
use crate::canvas::tool;
use crate::canvas::tool::{Pending, Tool};
use crate::canvas::tools::{
    brush::BrushPending,
    brushes::{airbrush::Airbrush, eraser::Eraser, pen::Pen, pencil::Pencil},
};
use crate::canvas::tools::{
    circle::CirclePending, ellipse::EllipsePending, line::LinePending, polygon::PolygonPending,
    rect::RectPending, triangle::TrianglePending,
};
use crate::config;
use crate::errors::debug::DebugError;
use crate::errors::error::Error;
use crate::scene::{Action, Globals, Message, Scene, SceneOptions};
use crate::scenes::scenes::Scenes;

use crate::theme::Theme;

use crate::mongo::{MongoRequest, MongoRequestType, MongoResponse};
use crate::serde::{Deserialize, Serialize};
use crate::widgets::combo_box::ComboBox;
use crate::widgets::modal_stack::ModalStack;
use crate::widgets::card::Card;
use crate::widgets::closeable::Closeable;
use crate::widgets::grid::Grid;

/// The types of the modals that can be opened.
#[derive(Clone, Eq, PartialEq)]
pub enum ModalTypes {
    /// A prompt where the user can write data for a post they are creating.
    PostPrompt
}

/// Data for a post tag.
#[derive(Default, Clone)]
pub struct Tag {
    /// The name of the tag.
    name: String,

    /// The number of posts the tag has been used in.
    uses: u32,
}

impl Tag {
    /// Reduces the name of a new tag to a base tag form.
    pub fn reduced(mut self) -> Self {
        self.name = self.name.to_ascii_lowercase().split_whitespace().collect::<Vec<&str>>().join(" ");

        self
    }
}

impl PartialEq for Tag {
    fn eq(&self, other: &Self) -> bool {
        self.name.clone() == other.name
    }
}

impl Display for Tag {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&*format!("{}({})", self.name, self.uses))
    }
}

impl Serialize<Document> for Tag {
    fn serialize(&self) -> Document {
        doc![
            "name": self.name.clone(),
            "uses": self.uses
        ]
    }
}

impl Deserialize<Document> for Tag {
    fn deserialize(document: Document) -> Self where Self: Sized {
        let mut tag = Tag { name: "".into(), uses: 0 };

        if let Some(Bson::String(name)) = document.get("name") {
           tag.name = name.clone();
        }
        if let Some(Bson::Int32(uses)) = document.get("uses") {
            tag.uses = *uses as u32;
        }

        tag
    }
}

/// The data of a post.
#[derive(Default, Clone)]
pub struct PostData {
    /// The description of the post.
    description: String,

    /// The list of new tags the user has added.
    new_tags: Vec<Tag>,

    /// The list of tags the user has chosen for the post.
    post_tags: Vec<Tag>,

    /// A list of all tags that have been applied to a post.
    all_tags: Vec<Tag>,

    /// The current input the user has written for a new tag.
    tag_input: String,
}

/// Possible updates to a new post data.
#[derive(Clone)]
pub enum UpdatePostData {
    Description(String),
    NewTag(String),
    SelectedTag(Tag),
    AllTags(Vec<Tag>),
    TagInput(String),
}

impl PostData {
    /// Updates the new post data.
    fn update(&mut self, update: UpdatePostData) {
        match update {
            UpdatePostData::Description(description) => self.description = description,
            UpdatePostData::NewTag(name) => {
                let tag = Tag { name, uses: 0 }.reduced();

                if self.post_tags.iter().find(|pos_tag| **pos_tag == tag).is_none() {
                    self.post_tags.push(tag.clone());
                    self.new_tags.push(tag);
                }
                self.tag_input = "".into();
            }
            UpdatePostData::SelectedTag(tag) => {
                if self.post_tags.iter().find(|pos_tag| **pos_tag == tag).is_none() {
                    self.post_tags.push(tag);
                }
                self.tag_input = "".into();
            }
            UpdatePostData::AllTags(tags) => self.all_tags = tags,
            UpdatePostData::TagInput(tag_input) => self.tag_input = tag_input,
        }
    }
}

/// The [Messages](Action) for the [Drawing] scene.
#[derive(Clone)]
pub(crate) enum DrawingAction {
    None,

    /// Triggered when the user has interacted with the canvas.
    CanvasAction(CanvasAction),

    /// Creates a new post given the canvas and the [PostData].
    PostDrawing,

    /// Updates the [PostData] given the modified field.
    UpdatePostData(UpdatePostData),

    /// Toggles a [Modal](ModalTypes).
    ToggleModal(ModalTypes),

    /// Opens the given tab.
    TabSelection(TabIds),

    /// Handles errors.
    ErrorHandler(Error),
}

/// The mode in which the progress will be saved.
#[derive(Debug, Clone, Copy)]
pub(crate) enum SaveMode {
    /// Saves the canvas locally.
    Offline,

    /// Saves the canvas in a database.
    Online,
}

impl Action for DrawingAction {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn get_name(&self) -> String {
        match self {
            DrawingAction::None => String::from("None"),
            DrawingAction::CanvasAction(_) => String::from("Canvas action"),
            DrawingAction::PostDrawing => String::from("Post drawing"),
            DrawingAction::UpdatePostData(_) => String::from("Update post data"),
            DrawingAction::ToggleModal(_) => String::from("Toggle modal"),
            DrawingAction::TabSelection(_) => String::from("Tab selected"),
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

    ///The currently active tab that displays the possible tools or styles.
    active_tab: TabIds,

    /// The new post data.
    post_data: PostData,

    /// The save mode of the drawing.
    save_mode: SaveMode,

    /// The stack of modals displayed.
    modal_stack: ModalStack<ModalTypes>,
}

impl Drawing {
    /// Initialize the drawing scene from the mongo database.
    /// If the uuid is 0, then insert a new drawing in the database.
    fn init_online(self: &mut Box<Self>, globals: &mut Globals) -> Command<Message> {
        let mut uuid = self.canvas.id.clone();
        if uuid != Uuid::from_bytes([0; 16]) {
            if let Some(db) = globals.get_db() {
                Command::perform(
                    async move {
                        MongoRequest::send_requests(
                            db,
                            vec![
                                MongoRequest::new(
                                    "canvases".into(),
                                    MongoRequestType::Get{
                                        filter: doc! {"id": uuid},
                                        options: None
                                    },
                                ),
                                MongoRequest::new(
                                    "tools".into(),
                                    MongoRequestType::Get{
                                        filter: doc! {"canvas_id": uuid},
                                        options: None
                                    },
                                ),
                            ]
                        ).await
                    },
                    move |res| {
                        match res {
                            Ok(res) => {
                                if let (Some(MongoResponse::Get(canvas)), Some(MongoResponse::Get(tools))) =
                                    (res.get(0), res.get(1))
                                {
                                    let layer_count = canvas.get(0);
                                    let layer_count = if let Some(document) = layer_count {
                                        if let Some(Bson::Int32(layer_count)) = document.get("layers") {
                                            *layer_count as usize
                                        } else {
                                            1
                                        }
                                    } else {
                                        1
                                    };

                                    let mut tools_vec: Vec<(Arc<dyn Tool>, usize)> = vec![];
                                    for tool in tools {
                                        tools_vec.push(tool::get_deserialized(tool.clone()).unwrap());
                                    }

                                    Message::DoAction(Box::new(DrawingAction::CanvasAction(CanvasAction::Loaded {
                                        layers: layer_count,
                                        tools: tools_vec,
                                        json_tools: None,
                                    })))
                                } else {
                                    Message::DoAction(Box::new(DrawingAction::None))
                                }
                            }
                            Err(message) => message
                        }
                    }
                )
            } else {
                Command::none()
            }
        } else {
            uuid = Uuid::new();
            self.canvas.id = Uuid::from(uuid.clone());

            if let Some(db) = globals.get_db() {
                let user_id = globals.get_user().unwrap().get_id();

                Command::perform(
                    async move {
                        MongoRequest::send_requests(
                            db,
                            vec![MongoRequest::new(
                                "canvases".into(),
                                MongoRequestType::Insert{
                                    documents: vec![doc! {"id": uuid, "user_id": user_id, "layers": 1}],
                                    options: None
                                },
                            )]
                        ).await
                    },
                    move |_| {
                        Message::DoAction(Box::new(DrawingAction::CanvasAction(CanvasAction::Loaded {
                            layers: 1,
                            tools: vec![],
                            json_tools: None,
                        })))
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
        let mut default_json = Object::new();
        default_json.insert("layers", JsonValue::Number(1.into()));
        default_json.insert("tools", JsonValue::Array(vec![]));

        let mut uuid = self.canvas.id.clone();
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
                        let mut layers = 1;
                        let mut tools = vec![];
                        let mut json_tools = vec![];

                        if let Some(JsonValue::Number(cnt_layers)) = data.get("layers") {
                            layers = f32::from(*cnt_layers) as usize;
                        }
                        if let Some(JsonValue::Array(tool_list)) = data.get("tools") {
                            json_tools = tool_list.clone();

                            for tool in tool_list {
                                if let JsonValue::Object(tool) = tool {
                                    if let Some(tool) = tool::get_json(tool.clone()) {
                                        tools.push(tool);
                                    }
                                }
                            }
                        }

                        (layers, tools, json_tools)
                    } else {
                        (1, vec![], vec![])
                    }
                },
                |(layer_count, tools, json_tools)| {
                    Message::DoAction(Box::new(DrawingAction::CanvasAction(
                        CanvasAction::Loaded {
                            layers: layer_count,
                            tools,
                            json_tools: Some(json_tools),
                        },
                    )))
                },
            )
        } else {
            uuid = Uuid::new();
            self.canvas.id = Uuid::from(uuid.clone());

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
                    layers: 1,
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
            scene.canvas.id = uuid;
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
            active_tab: TabIds::Tools,
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
        let message: &DrawingAction = message
            .as_any()
            .downcast_ref::<DrawingAction>()
            .expect("Panic downcasting to DrawingAction");

        match message {
            DrawingAction::CanvasAction(action) => self.canvas.update(globals, action.clone()),
            DrawingAction::UpdatePostData(update) => {
                self.post_data.update(update.clone());
                Command::none()
            }
            DrawingAction::PostDrawing => {
                let document = self.canvas.svg.as_document();
                let db = globals.get_db().unwrap();
                let user_id = globals.get_user().unwrap().get_id();
                let description = self.post_data.description.clone();

                let tags :Vec<String>= self.post_data.post_tags.iter().map(|tag| tag.name.clone()).collect();
                let new_tags: Vec<Tag> = self.post_data.new_tags.clone();

                self.post_data.post_tags = vec![];
                self.post_data.new_tags = vec![];
                self.post_data.description = "".into();
                self.post_data.tag_input = "".into();

                Command::perform(
                    async move {
                        let buffer = document.to_string();
                        let img = svg2webp(&*buffer, 80.0).unwrap();
                        let mut auth = dropbox_sdk::oauth2::Authorization::from_refresh_token(
                            config::dropbox_id().into(),
                            config::dropbox_refresh_token().into(),
                        );

                        let post_id = Uuid::new();

                        let _token = auth
                            .obtain_access_token(NoauthDefaultClient::default())
                            .unwrap();
                        let client = UserAuthDefaultClient::new(auth);

                        match files::upload(
                            &client,
                            &files::UploadArg::new(format!("/{}/{}.webp", user_id, post_id))
                                .with_mute(false)
                                .with_mode(WriteMode::Overwrite),
                            &img,
                        ) {
                            Ok(Ok(_metadata)) => {
                                println!("File successfully sent!");
                            }
                            Ok(Err(err)) => {
                                return Err(Error::DebugError(DebugError::new(format!("Error sending file: {}", err))));
                            }
                            Err(err) => {
                                return Err(Error::DebugError(DebugError::new(format!("Error with upload request: {}", err))));
                            }
                        }

                        Ok(MongoRequest::send_requests(
                            db,
                            vec![
                                MongoRequest::new(
                                    "posts".into(),
                                    MongoRequestType::Insert {
                                        documents: vec![
                                            doc!{
                                                "id": post_id,
                                                "user_id": user_id,
                                                "description": description,
                                                "tags": tags.clone()
                                            }
                                        ],
                                        options: None
                                    }
                                ),
                                MongoRequest::new(
                                    "tags".into(),
                                    MongoRequestType::Insert {
                                        documents: new_tags.iter().map(Serialize::serialize).collect(),
                                        options: None
                                    }
                                ),
                                MongoRequest::new(
                                    "tags".into(),
                                    MongoRequestType::Update {
                                        filter: doc! {
                                            "name": { "$in": tags }
                                        },
                                        update: doc! {
                                            "$inc": { "uses": 1 }
                                        },
                                        options: None
                                    }
                                )
                            ]
                        ).await)
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
                        if self.post_data.all_tags.is_empty() {
                            if let (Some(_), Some(db)) = (globals.get_user(), globals.get_db()) {
                                Command::perform(
                                    async move {
                                        MongoRequest::send_requests(
                                            db,
                                            vec![
                                                MongoRequest::new(
                                                    "tags".into(),
                                                    MongoRequestType::Get {
                                                        filter: doc![],
                                                        options: None
                                                    }
                                                )
                                            ]
                                        ).await
                                    },
                                    |res| {
                                        match res {
                                            Ok(responses) => {
                                                if let Some(MongoResponse::Get(documents)) = responses.get(0) {
                                                    Message::DoAction(Box::new(DrawingAction::UpdatePostData(UpdatePostData::AllTags(
                                                        documents.iter().map(
                                                            |document| {
                                                                Tag::deserialize(document.clone())
                                                            }
                                                        ).collect()
                                                    ))))
                                                } else {
                                                    Message::Error(Error::DebugError(
                                                        DebugError::new("Request answered wrong type".into())
                                                    ))
                                                }
                                            }
                                            Err(message) => message
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
            DrawingAction::TabSelection(tab_id) => {
                self.active_tab = *tab_id;
                Command::none()
            }
            DrawingAction::ErrorHandler(_) => Command::none(),
            DrawingAction::None => Command::none(),
        }
    }

    fn view(&self, globals: &Globals) -> Element<'_, Message, Theme, Renderer> {
        let tool_button = |name: String, pending: Box<dyn Pending>| {
            Button::new(Text::new(name))
                .on_press(Message::DoAction(Box::new(DrawingAction::CanvasAction(
                    CanvasAction::ChangeTool(pending)
                ))))
                .into()
        };

        let geometry_section :Element<Message, Theme, Renderer>= Column::with_children(vec![
            tool_button("Line".into(), Box::new(LinePending::None)),
            tool_button("Rectangle".into(), Box::new(RectPending::None)),
            tool_button("Triangle".into(), Box::new(TrianglePending::None)),
            tool_button("Polygon".into(), Box::new(PolygonPending::None)),
            tool_button("Circle".into(), Box::new(CirclePending::None)),
            tool_button("Ellipse".into(), Box::new(EllipsePending::None)),
        ])
            .spacing(5.0)
            .padding(10.0)
            .into();

        let brushes_section :Element<Message, Theme, Renderer>= Column::with_children(vec![
            tool_button("Pencil".into(), Box::new(BrushPending::<Pencil>::None)),
            tool_button("Fountain Pen".into(), Box::new(BrushPending::<Pen>::None)),
            tool_button("Airbrush".into(), Box::new(BrushPending::<Airbrush>::None))
        ])
            .spacing(5.0)
            .padding(10.0)
            .into();

        let eraser_section :Element<Message, Theme, Renderer>= Column::with_children(vec![
            tool_button("Eraser".into(), Box::new(BrushPending::<Eraser>::None))
        ])
            .spacing(5.0)
            .padding(10.0)
            .into();

        let layers_section :Element<Message, Theme, Renderer>= Row::with_children((|layers: usize| {
            let mut buttons = vec![];
            for layer in 0..layers.clone() {
                buttons.push(
                    Button::new(Text::new(format!("Layer {}", layer + 1)))
                        .on_press(Message::DoAction(Box::new(
                            DrawingAction::CanvasAction(CanvasAction::ActivateLayer(
                                layer,
                            )),
                        )))
                        .into(),
                );
            }

            buttons
        })(self.canvas.get_layer_count()))
            .into();

        let buttons_section :Element<Message, Theme, Renderer>= Row::with_children(vec![
            Button::new(Text::new("Back"))
                .on_press(Message::ChangeScene(Scenes::Main(None)))
                .into(),
            if globals.get_db().is_some() && globals.get_user().is_some() {
                Button::new(Text::new("Post"))
                    .on_press(Message::DoAction(Box::new(DrawingAction::ToggleModal(ModalTypes::PostPrompt))))
            } else {
                Button::new(Text::new("Post"))
            }
                .into(),
            Button::new(Text::new("Save"))
                .on_press(Message::DoAction(Box::new(DrawingAction::CanvasAction(CanvasAction::Save))))
                .into(),
            Button::new(Text::new("Add layer"))
                .on_press(Message::DoAction(Box::new(DrawingAction::CanvasAction(CanvasAction::AddLayer))))
                .into(),
            layers_section,
        ])
            .spacing(8.0)
            .into();

        let underlay = Row::with_children(
            vec![
                Tabs::new_with_tabs(
                    vec![
                        (
                            TabIds::Tools,
                            TabLabel::Text("Tools".into()),
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
                                .spacing(15.0)
                                .height(Length::Fill)
                                .width(Length::Fixed(250.0))
                                .into()
                        ),
                        (
                            TabIds::Style,
                            TabLabel::Text("Style".into()),
                            self.canvas
                                .style
                                .view()
                                .map(|update| Message::DoAction(Box::new(
                                    DrawingAction::CanvasAction(CanvasAction::UpdateStyle(update))
                                ))),
                        )
                    ],
                    |tab_id| Message::DoAction(Box::new(DrawingAction::TabSelection(tab_id))),
                )
                    .tab_bar_height(Length::Fixed(35.0))
                    .width(Length::Fixed(250.0))
                    .height(Length::Fixed(800.0))
                    .set_active_tab(&self.active_tab)
                    .into(),
                Column::with_children(vec![
                    Text::new(format!("{}", self.get_title()))
                        .width(Length::Shrink)
                        .size(50)
                        .into(),
                    Container::new::<&Canvas>(&self.canvas)
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .center_x()
                        .center_y()
                        //.style(container::Container::Canvas),
                        .into(),
                    buttons_section
                ])
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
                                        &*self.post_data.description
                                    )
                                        .on_input(|new_value| Message::DoAction(Box::new(DrawingAction::UpdatePostData(UpdatePostData::Description(new_value)))))
                                        .into(),
                                    Text::new("Tags:").into(),
                                    Grid::new(self.post_data.post_tags.iter().map(
                                        |tag| Badge::new(
                                            Text::new(tag.name.clone())
                                        )
                                            .padding(3)
                                    ).collect())
                                        .padding(Padding::from([5.0, 0.0, 5.0, 0.0]))
                                        .spacing(5.0)
                                        .into(),
                                    Row::with_children(
                                        vec![
                                            ComboBox::new(
                                                self.post_data.all_tags.clone(),
                                                "Add a new tag...",
                                                &*self.post_data.tag_input,
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
                                                    UpdatePostData::NewTag(self.post_data.tag_input.clone())
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

    fn clear(&self) {}
}

/// The tabs in the selection section.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum TabIds {
    Tools,
    Style,
}
