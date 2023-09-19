use std::any::Any;
use std::sync::Arc;

use iced::{Alignment, Command, Element, Length, Renderer};
use iced::alignment::Horizontal;
use iced::widget::{button, text, column, row, Container};
use iced_aw::card::Card;
use mongodb::bson::{doc, Document, Uuid};
use mongodb::results::InsertManyResult;

use crate::scene::{Scene, Action, Message, SceneOptions, Globals};
use crate::tool::{self, Tool, Pending};
use crate::tools::{line::LinePending, rect::RectPending, triangle::TrianglePending, polygon::PolygonPending, circle::CirclePending, ellipse::EllipsePending};
use crate::tools::{brush::BrushPending, brushes::{pencil::Pencil, pen::Pen, airbrush::Airbrush, eraser::Eraser}};
use crate::scenes::scenes::Scenes;
use crate::canvas::canvas::{CanvasAction, State};

use crate::theme::{container, Theme};

use crate::mongo::{MongoRequest, MongoRequestType, MongoResponse};

#[derive(Clone)]
enum DrawingAction {
    None,
    CanvasAction(CanvasAction),
    ChangeTool(Box<dyn Pending>),
    Saved(Arc<InsertManyResult>),
    Loaded(Vec<Document>),
}

impl Action for DrawingAction {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn get_name(&self) -> String {
        match self {
            DrawingAction::None => String::from("None"),
            DrawingAction::CanvasAction(_) => String::from("Canvas action"),
            DrawingAction::ChangeTool(_) => String::from("Change tool"),
            DrawingAction::Saved(_) => String::from("Finished saving"),
            DrawingAction::Loaded(_) => String::from(format!("Finished loading from database")),
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

pub struct Drawing {
    canvas_id: Uuid,
    state: State,
    tools: Box<Vec<Box<dyn Tool>>>,
    undo_stack: Box<Vec<Box<dyn Tool>>>,
    count_saved: usize,
    current_tool: Box<dyn Pending>,
    globals: Globals,
}

impl Drawing {
    fn get_tools_serialized(&self) -> Vec<Document> {
        let mut vec = vec![];

        for pos in self.count_saved..self.tools.len() {
            let val = self.tools.get(pos);

            if let Some(tool) = val {
                let mut document = tool.serialize();
                document.insert("order", pos.clone() as u32);
                document.insert("canvas_id", self.canvas_id);
                document.insert("name", tool.id());

                vec.push(document);
            }
        }

        vec
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DrawingOptions {
    uuid: Option<Uuid>,
}

impl DrawingOptions {
    pub(crate) fn new(uuid: Option<Uuid>) -> Self {
        DrawingOptions { uuid }
    }
}

impl SceneOptions<Box<Drawing>> for DrawingOptions {
    fn apply_options(&self, scene: &mut Box<Drawing>) {
        if let Some(uuid) = self.uuid {
            scene.canvas_id = uuid;
        }
    }

    fn boxed_clone(&self) -> Box<dyn SceneOptions<Box<Drawing>>> {
        Box::new((*self).clone())
    }
}

impl Scene for Box<Drawing> {
    fn new(options: Option<Box<dyn SceneOptions<Box<Drawing>>>>, globals: Globals) -> (Self, Command<Message>) where Self: Sized {
        let mut drawing = Box::new(
            Drawing {
                canvas_id: Uuid::new(),
                state: State::default(),
                tools: Box::new(vec![]),
                undo_stack: Box::new(vec![]),
                count_saved: 0,
                current_tool: Box::new(LinePending::None),
                globals,
            }
        );

        if let Some(options) = options {
            options.apply_options(&mut drawing);

            let uuid = drawing.canvas_id.clone();

            (
                drawing,
                Command::perform(
                    async {},
                    move |_| {
                        Message::SendMongoRequests(
                            vec![
                                MongoRequest::new(
                                    "tools".into(),
                                    MongoRequestType::Get(doc! {"canvas_id": uuid}),
                                )
                            ],
                            move |res| {
                                if let Some(MongoResponse::Get(cursor)) = res.get(0) {
                                    Box::new(DrawingAction::Loaded(cursor.clone()))
                                } else {
                                    Box::new(DrawingAction::None)
                                }
                            }
                        )
                    }
                )
            )
        } else {
            let uuid = drawing.canvas_id.clone();

            (
                drawing,
                Command::perform(
                    async {},
                        move |_| {
                            Message::SendMongoRequests(
                                vec![
                                    MongoRequest::new(
                                        "canvases".into(),
                                        MongoRequestType::Insert(vec![doc!{"id": uuid}]),
                                    )
                                ],
                                |_| Box::new(DrawingAction::None),
                            )
                        }
                )
            )
        }
    }

    fn get_title(&self) -> String {
        String::from("Drawing")
    }

    fn update(&mut self, message: Box<dyn Action>) -> Command<Message> {
        let message: &DrawingAction = message.as_any().downcast_ref::<DrawingAction>().expect("Panic downcasting to DrawingAction");

        match message {
            DrawingAction::CanvasAction(action) => {
                match action {
                    CanvasAction::UseTool(tool) => {
                        self.tools.push(tool.clone());
                        self.undo_stack = Box::new(vec![]);
                        self.state.request_redraw();
                    }
                    CanvasAction::Save => {
                        let tools = self.get_tools_serialized();
                        let delete_lower_bound = self.count_saved;
                        let delete_upper_bound = self.tools.len();

                        return
                            Command::perform(
                                async {},
                                move |_| {
                                    Message::SendMongoRequests(
                                        vec![
                                            MongoRequest::new(
                                                "tools".into(),
                                                MongoRequestType::Delete(
                                                    doc!{"order": {
                                                        "$gte": delete_lower_bound as u32,
                                                        "$lte": delete_upper_bound as u32,
                                                    }}
                                                )
                                            ),
                                            MongoRequest::new(
                                                "tools".into(),
                                                MongoRequestType::Insert(tools),
                                            )
                                        ],
                                        move |responses| {
                                            for response in responses {
                                                if let MongoResponse::Insert(result) = response {
                                                    return Box::new(DrawingAction::Saved(Arc::new(result)));
                                                }
                                                break
                                            }

                                            Box::new(DrawingAction::None)
                                        }
                                    )
                                }
                            );
                    }
                    CanvasAction::Undo => {
                        if self.tools.len() > 0 {
                            let opt = self.tools.pop();
                            if let Some(tool) = opt {
                                self.undo_stack.push(tool);
                            }
                            self.state.request_redraw();

                            if self.count_saved > self.tools.len() {
                                self.count_saved -= 1;
                            }
                        }
                    }
                    CanvasAction::Redo => {
                        let opt = self.undo_stack.pop();

                        if let Some(tool) = opt {
                            self.tools.push(tool);
                            self.state.request_redraw();
                        }
                    }
                }
            }
            DrawingAction::ChangeTool(tool) => {
                self.current_tool = (*tool).boxed_clone();
            }
            DrawingAction::Saved(insert_result) => {
                self.count_saved += insert_result.inserted_ids.len();

                for (key, value) in insert_result.inserted_ids.iter() {
                    println!("Inserted key {} with value {}.", key, value);
                }
            }
            DrawingAction::Loaded(cursor) => {
                self.tools = Box::new(vec![]);
                for tool in cursor {
                    let tool = tool::get_deserialized(tool.clone());

                    match tool {
                        Some(tool) => {
                            self.tools.push(tool);
                        }
                        None => {
                            eprintln!("Failed to get correct type of tool.");
                        }
                    }
                }

                self.count_saved = self.tools.len();
                self.state.request_redraw();
            }
            _ => {}
        }

        Command::none()
    }

    fn view(&self) -> Element<'_, Message, Renderer<Theme>> {
        if self.globals.get_window_height() == 0.0 {
            return Element::new(text(""));
        }

        row![
            Card::new(
                text("Tools").horizontal_alignment(Horizontal::Center).size(25.0).height(Length::Fixed(50.0)),
                column![
                    text("Geometry").horizontal_alignment(Horizontal::Center).size(20.0),
                    column![
                        button("Line").on_press(Message::DoAction(Box::new(DrawingAction::ChangeTool(Box::new(LinePending::None))))),
                        button("Rectangle").on_press(Message::DoAction(Box::new(DrawingAction::ChangeTool(Box::new(RectPending::None))))),
                        button("Triangle").on_press(Message::DoAction(Box::new(DrawingAction::ChangeTool(Box::new(TrianglePending::None))))),
                        button("Polygon").on_press(Message::DoAction(Box::new(DrawingAction::ChangeTool(Box::new(PolygonPending::None))))),
                        button("Circle").on_press(Message::DoAction(Box::new(DrawingAction::ChangeTool(Box::new(CirclePending::None))))),
                        button("Ellipse").on_press(Message::DoAction(Box::new(DrawingAction::ChangeTool(Box::new(EllipsePending::None))))),
                    ].spacing(5.0).padding(10.0),
                    text("Brushes").horizontal_alignment(Horizontal::Center).size(20.0),
                    column![
                        button("Pencil").on_press(Message::DoAction(Box::new(DrawingAction::ChangeTool(Box::new(BrushPending::<Pencil>::None))))),
                        button("Fountain pen").on_press(Message::DoAction(Box::new(DrawingAction::ChangeTool(Box::new(BrushPending::<Pen>::None))))),
                        button("Airbrush").on_press(Message::DoAction(Box::new(DrawingAction::ChangeTool(Box::new(BrushPending::<Airbrush>::None))))),
                    ].spacing(5.0).padding(10.0),
                    text("Eraser").horizontal_alignment(Horizontal::Center).size(20.0),
                    column![
                        button("Eraser").on_press(Message::DoAction(Box::new(DrawingAction::ChangeTool(Box::new(BrushPending::<Eraser>::None)))))
                    ].spacing(5.0).padding(10.0),
                ]
                .spacing(15.0)
                .height(Length::Fill)
                .width(Length::Fixed(250.0)),
            )
            .height(Length::Fill)
            .width(Length::Fixed(250.0))
            .max_height(self.globals.get_window_height() - 70.0),
            column![
                text(format!("{}", self.get_title())).width(Length::Shrink).size(50),
                Container::new(
                    self.state.view(
                        &self.tools,
                        &self.current_tool
                    ).map(
                        |action|
                        {
                            Message::DoAction(Box::new(DrawingAction::CanvasAction(action)).into())
                        }
                    )
                )
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .center_x()
                    .center_y()
                    .style(container::Container::Canvas),
                row![
                    button("Back").padding(8).on_press(Message::ChangeScene(Scenes::Main(None))),
                    button("Save").padding(8).on_press(Message::DoAction(Box::new(DrawingAction::CanvasAction(CanvasAction::Save)))),
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

    fn update_globals(&mut self, globals: Globals) {
        self.globals = globals;
    }

    fn clear(&self) { }
}