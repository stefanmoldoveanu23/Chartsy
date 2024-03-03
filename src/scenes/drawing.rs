use directories::ProjectDirs;
use dropbox_sdk::default_client::{NoauthDefaultClient, UserAuthDefaultClient};
use dropbox_sdk::files;
use dropbox_sdk::files::WriteMode;
use std::any::Any;
use std::fs;
use std::fs::{create_dir_all, File};
use std::io::Write;
use std::sync::Arc;

use crate::canvas::canvas::Canvas;
use iced::alignment::Horizontal;
use iced::widget::{button, column, row, text, Container, Row};
use iced::{Alignment, Command, Element, Length, Renderer};
use iced_aw::tab_bar::TabLabel;
use iced_aw::tabs::Tabs;
use json::object::Object;
use json::JsonValue;
use mongodb::bson::{doc, Bson, Uuid};

use crate::canvas::layer::CanvasAction;
use crate::canvas::tool;
use crate::canvas::tool::Tool;
use crate::canvas::tools::{
    brush::BrushPending,
    brushes::{airbrush::Airbrush, eraser::Eraser, pen::Pen, pencil::Pencil},
};
use crate::canvas::tools::{
    circle::CirclePending, ellipse::EllipsePending, line::LinePending, polygon::PolygonPending,
    rect::RectPending, triangle::TrianglePending,
};
use crate::config::{DROPBOX_ID, DROPBOX_REFRESH_TOKEN};
use crate::errors::debug::DebugError;
use crate::errors::error::Error;
use crate::scene::{Action, Globals, Message, Scene, SceneOptions};
use crate::scenes::scenes::Scenes;

use crate::theme::Theme;

use crate::mongo::{MongoRequest, MongoRequestType, MongoResponse};

/// The [Messages](Action) for the [Drawing] scene:
/// - [CanvasAction](DrawingAction::CanvasAction), for when the user interacts with the canvas;
/// to be sent to the [Canvas] instance for handling;
/// - [PostDrawing](DrawingAction::PostDrawing), to post a drawing for other users;
/// - [TabSelection](DrawingAction::TabSelection), which handles the options tab for drawing;
/// - [ErrorHandler(Error)](DrawingAction::ErrorHandler), which handles errors.
#[derive(Clone)]
pub(crate) enum DrawingAction {
    None,
    CanvasAction(CanvasAction),
    PostDrawing,
    TabSelection(TabIds),
    ErrorHandler(Error),
}

/// The mode in which the progress will be saved:
/// - [Offline](SaveMode::Offline) for local saving;
/// - [Online](SaveMode::Online) for remote saving in a database.
#[derive(Debug, Clone, Copy)]
pub(crate) enum SaveMode {
    Offline,
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
///
/// Split into a section where the user can choose [Tools](Tool) and
/// modify the drawing [Style](crate::canvas::style::Style), and the [Canvas].
pub struct Drawing {
    canvas: Canvas,
    active_tab: TabIds,
    save_mode: SaveMode,
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
                                    MongoRequestType::Get(doc! {"id": uuid}),
                                ),
                                MongoRequest::new(
                                    "tools".into(),
                                    MongoRequestType::Get(doc! {"canvas_id": uuid}),
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
                                MongoRequestType::Insert(vec![doc! {"id": uuid, "user_id": user_id, "layers": 1}]),
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

            let file_path = dir_path.join(String::from("./data.json"));
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

/// Contains the [uuid](Uuid) and the [save mode](SaveMode) of the current [Drawing].
#[derive(Debug, Clone, Copy)]
pub struct DrawingOptions {
    uuid: Option<Uuid>,
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
            save_mode: SaveMode::Online,
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
            DrawingAction::PostDrawing => {
                let document: svg::Document = self.canvas.svg.as_document();
                let db = globals.get_db().unwrap();
                let user_id = globals.get_user().unwrap().get_id();
                let drawing_id = self.canvas.id;

                Command::perform(
                    async move {
                        let buffer = document.to_string();
                        let img = buffer.as_bytes();
                        let mut auth = dropbox_sdk::oauth2::Authorization::from_refresh_token(
                            DROPBOX_ID.into(),
                            DROPBOX_REFRESH_TOKEN.into(),
                        );

                        let _token = auth
                            .obtain_access_token(NoauthDefaultClient::default())
                            .unwrap();
                        let client = UserAuthDefaultClient::new(auth);

                        match files::upload(
                            &client,
                            &files::UploadArg::new(format!("/{}/{}.svg", user_id, drawing_id))
                                .with_mute(false)
                                .with_mode(WriteMode::Overwrite),
                            img,
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
                                    MongoRequestType::Insert(vec![
                                        doc!{
                                            "drawing_id": drawing_id,
                                            "user_id": user_id,
                                        }
                                    ])
                                )
                            ]
                        ).await)
                    },
                    |res| {
                        match res {
                            Ok(_) => Message::DoAction(Box::new(DrawingAction::None)),
                            Err(err) => Message::Error(err)
                        }
                    },
                )
            }
            DrawingAction::TabSelection(tab_id) => {
                self.active_tab = *tab_id;
                Command::none()
            }
            DrawingAction::ErrorHandler(_) => Command::none(),
            DrawingAction::None => Command::none(),
        }
    }

    fn view(&self, globals: &Globals) -> Element<'_, Message, Renderer<Theme>> {
        if globals.get_window_height() == 0.0 {
            return Element::new(text(""));
        }

        row![
            Tabs::with_tabs(
                vec![
                    (
                        TabIds::Tools,
                        TabLabel::Text("Tools".into()),
                        column![
                            text("Geometry")
                                .horizontal_alignment(Horizontal::Center)
                                .size(20.0),
                            column![
                                button("Line").on_press(Message::DoAction(Box::new(
                                    DrawingAction::CanvasAction(CanvasAction::ChangeTool(
                                        Box::new(LinePending::None)
                                    ))
                                ))),
                                button("Rectangle").on_press(Message::DoAction(Box::new(
                                    DrawingAction::CanvasAction(CanvasAction::ChangeTool(
                                        Box::new(RectPending::None)
                                    ))
                                ))),
                                button("Triangle").on_press(Message::DoAction(Box::new(
                                    DrawingAction::CanvasAction(CanvasAction::ChangeTool(
                                        Box::new(TrianglePending::None)
                                    ))
                                ))),
                                button("Polygon").on_press(Message::DoAction(Box::new(
                                    DrawingAction::CanvasAction(CanvasAction::ChangeTool(
                                        Box::new(PolygonPending::None)
                                    ))
                                ))),
                                button("Circle").on_press(Message::DoAction(Box::new(
                                    DrawingAction::CanvasAction(CanvasAction::ChangeTool(
                                        Box::new(CirclePending::None)
                                    ))
                                ))),
                                button("Ellipse").on_press(Message::DoAction(Box::new(
                                    DrawingAction::CanvasAction(CanvasAction::ChangeTool(
                                        Box::new(EllipsePending::None)
                                    ))
                                ))),
                            ]
                            .spacing(5.0)
                            .padding(10.0),
                            text("Brushes")
                                .horizontal_alignment(Horizontal::Center)
                                .size(20.0),
                            column![
                                button("Pencil").on_press(Message::DoAction(Box::new(
                                    DrawingAction::CanvasAction(CanvasAction::ChangeTool(
                                        Box::new(BrushPending::<Pencil>::None)
                                    ))
                                ))),
                                button("Fountain pen").on_press(Message::DoAction(Box::new(
                                    DrawingAction::CanvasAction(CanvasAction::ChangeTool(
                                        Box::new(BrushPending::<Pen>::None)
                                    ))
                                ))),
                                button("Airbrush").on_press(Message::DoAction(Box::new(
                                    DrawingAction::CanvasAction(CanvasAction::ChangeTool(
                                        Box::new(BrushPending::<Airbrush>::None)
                                    ))
                                ))),
                            ]
                            .spacing(5.0)
                            .padding(10.0),
                            text("Eraser")
                                .horizontal_alignment(Horizontal::Center)
                                .size(20.0),
                            column![button("Eraser").on_press(Message::DoAction(Box::new(
                                DrawingAction::CanvasAction(CanvasAction::ChangeTool(Box::new(
                                    BrushPending::<Eraser>::None
                                )))
                            )))]
                            .spacing(5.0)
                            .padding(10.0),
                        ]
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
            .height(Length::Fixed(globals.get_window_height() - 35.0))
            .set_active_tab(&self.active_tab),
            column![
                text(format!("{}", self.get_title()))
                    .width(Length::Shrink)
                    .size(50),
                Container::new::<&Canvas>(&self.canvas)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .center_x()
                    .center_y(),
                //.style(container::Container::Canvas),
                row![
                    button("Back")
                        .padding(8)
                        .on_press(Message::ChangeScene(Scenes::Main(None))),
                    if globals.get_db().is_some() && globals.get_user().is_some() {
                        button("Post")
                            .padding(8)
                            .on_press(Message::DoAction(Box::new(DrawingAction::PostDrawing)))
                    } else {
                        button("Post")
                            .padding(8)
                    },
                    button("Save")
                        .padding(8)
                        .on_press(Message::DoAction(Box::new(DrawingAction::CanvasAction(
                            CanvasAction::Save
                        )))),
                    button("Add layer")
                        .padding(8)
                        .on_press(Message::DoAction(Box::new(DrawingAction::CanvasAction(
                            CanvasAction::AddLayer
                        )))),
                    Row::with_children((|layers: usize| {
                        let mut buttons = vec![];
                        for layer in 0..layers.clone() {
                            buttons.push(
                                button(text(format!("Layer {}", layer + 1)))
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
                ]
            ]
            .height(Length::Fill)
        ]
        .padding(0)
        .spacing(20)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_items(Alignment::Center)
        .into()
    }

    fn get_error_handler(&self, error: Error) -> Box<dyn Action> {
        Box::new(DrawingAction::ErrorHandler(error))
    }

    fn clear(&self) {}
}

/// The tabs in the selection section:
/// - [Tools](TabIds::Tools), for selecting the used [Tool](Tool);
/// - [Style](TabIds::Style), for modifying the tools [Style](crate::canvas::style::Style).
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum TabIds {
    Tools,
    Style,
}
