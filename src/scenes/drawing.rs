use std::any::Any;

use iced::{Alignment, Command, Element, Length, Renderer};
use iced::alignment::Horizontal;
use iced::widget::{button, text, column, row, Container, Row};
use iced_aw::tabs::Tabs;
use iced_aw::tab_bar::TabLabel;
use mongodb::bson::{Bson, doc, Uuid};
use crate::canvas::canvas::Canvas;

use crate::scene::{Scene, Action, Message, SceneOptions, Globals};
use crate::canvas::tools::{line::LinePending, rect::RectPending, triangle::TrianglePending, polygon::PolygonPending, circle::CirclePending, ellipse::EllipsePending};
use crate::canvas::tools::{brush::BrushPending, brushes::{pencil::Pencil, pen::Pen, airbrush::Airbrush, eraser::Eraser}};
use crate::scenes::scenes::Scenes;
use crate::canvas::layer::CanvasAction;

use crate::theme::Theme;

use crate::mongo::{MongoRequest, MongoRequestType, MongoResponse};

#[derive(Clone)]
pub(crate) enum DrawingAction {
    None,
    CanvasAction(CanvasAction),
    TabSelection(TabIds),
}

impl Action for DrawingAction {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn get_name(&self) -> String {
        match self {
            DrawingAction::None => String::from("None"),
            DrawingAction::CanvasAction(_) => String::from("Canvas action"),
            DrawingAction::TabSelection(_) => String::from("Tab selected"),
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
    canvas: Canvas,
    active_tab: TabIds,
    globals: Globals,
}

impl Drawing {
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
            scene.canvas.id = uuid;
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
                canvas: Canvas::new().width(Length::Fixed(800.0)).height(Length::Fixed(600.0)),
                active_tab: TabIds::Tools,
                globals,
            }
        );

        if let Some(options) = options {
            options.apply_options(&mut drawing);

            let uuid = drawing.canvas.id.clone();

            (
                drawing,
                Command::perform(
                    async {},
                    move |_| {
                        Message::SendMongoRequests(
                            vec![
                                MongoRequest::new(
                                    "canvases".into(),
                                    MongoRequestType::Get(doc! {"id": uuid}),
                                ),
                                MongoRequest::new(
                                    "tools".into(),
                                    MongoRequestType::Get(doc! {"canvas_id": uuid}),
                                )
                            ],
                            move |res| {
                                if let (Some(MongoResponse::Get(canvas)),
                                    Some(MongoResponse::Get(tools))) = (res.get(0), res.get(1)) {
                                    let layer_count = canvas.get(0);
                                    let layer_count =
                                        if let Some(document) = layer_count {
                                            if let Some(Bson::Int32(layer_count)) = document.get("layers") {
                                                *layer_count as usize
                                            } else {
                                                1
                                            }
                                        } else {
                                            1
                                        };

                                    Box::new(DrawingAction::CanvasAction(CanvasAction::Loaded(layer_count, tools.clone())))
                                } else {
                                    Box::new(DrawingAction::None)
                                }
                            }
                        )
                    }
                )
            )
        } else {
            let uuid = drawing.canvas.id.clone();

            (
                drawing,
                Command::perform(
                    async {},
                        move |_| {
                            Message::SendMongoRequests(
                                vec![
                                    MongoRequest::new(
                                        "canvases".into(),
                                        MongoRequestType::Insert(vec![doc!{"id": uuid, "layers": 1}]),
                                    )
                                ],
                                |_| Box::new(DrawingAction::CanvasAction(
                                    CanvasAction::Loaded(1, vec![])
                                )),
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
                self.canvas.update(action.clone())
            }
            DrawingAction::TabSelection(tab_id) => {
                self.active_tab = *tab_id;
                Command::none()
            },
            _ => {Command::none()}
        }
    }

    fn view(&self) -> Element<'_, Message, Renderer<Theme>> {
        if self.globals.get_window_height() == 0.0 {
            return Element::new(text(""));
        }

        row![
            Tabs::with_tabs(
                vec![
                    (
                        TabIds::Tools,
                        TabLabel::Text("Tools".into()),
                        column![
                            text("Geometry").horizontal_alignment(Horizontal::Center).size(20.0),
                            column![
                                button("Line").on_press(Message::DoAction(Box::new(DrawingAction::CanvasAction(CanvasAction::ChangeTool(Box::new(LinePending::None)))))),
                                button("Rectangle").on_press(Message::DoAction(Box::new(DrawingAction::CanvasAction(CanvasAction::ChangeTool(Box::new(RectPending::None)))))),
                                button("Triangle").on_press(Message::DoAction(Box::new(DrawingAction::CanvasAction(CanvasAction::ChangeTool(Box::new(TrianglePending::None)))))),
                                button("Polygon").on_press(Message::DoAction(Box::new(DrawingAction::CanvasAction(CanvasAction::ChangeTool(Box::new(PolygonPending::None)))))),
                                button("Circle").on_press(Message::DoAction(Box::new(DrawingAction::CanvasAction(CanvasAction::ChangeTool(Box::new(CirclePending::None)))))),
                                button("Ellipse").on_press(Message::DoAction(Box::new(DrawingAction::CanvasAction(CanvasAction::ChangeTool(Box::new(EllipsePending::None)))))),
                            ].spacing(5.0).padding(10.0),
                            text("Brushes").horizontal_alignment(Horizontal::Center).size(20.0),
                            column![
                                button("Pencil").on_press(Message::DoAction(Box::new(DrawingAction::CanvasAction(CanvasAction::ChangeTool(Box::new(BrushPending::<Pencil>::None)))))),
                                button("Fountain pen").on_press(Message::DoAction(Box::new(DrawingAction::CanvasAction(CanvasAction::ChangeTool(Box::new(BrushPending::<Pen>::None)))))),
                                button("Airbrush").on_press(Message::DoAction(Box::new(DrawingAction::CanvasAction(CanvasAction::ChangeTool(Box::new(BrushPending::<Airbrush>::None)))))),
                            ].spacing(5.0).padding(10.0),
                            text("Eraser").horizontal_alignment(Horizontal::Center).size(20.0),
                            column![
                                button("Eraser").on_press(Message::DoAction(Box::new(DrawingAction::CanvasAction(CanvasAction::ChangeTool(Box::new(BrushPending::<Eraser>::None))))))
                            ].spacing(5.0).padding(10.0),
                        ]
                        .spacing(15.0)
                        .height(Length::Fill)
                        .width(Length::Fixed(250.0))
                        .into()
                    ),
                    (
                        TabIds::Style,
                        TabLabel::Text("Style".into()),
                        self.canvas.style.view().map(|update| Message::DoAction(Box::new(DrawingAction::CanvasAction(CanvasAction::UpdateStyle(update))))),
                    )
                ],
                |tab_id| Message::DoAction(Box::new(DrawingAction::TabSelection(tab_id))),
            )
            .tab_bar_height(Length::Fixed(35.0))
            .width(Length::Fixed(250.0))
            .height(Length::Fixed(self.globals.get_window_height() - 35.0))
            .set_active_tab(&self.active_tab),
            column![
                text(format!("{}", self.get_title())).width(Length::Shrink).size(50),
                Container::new::<&Canvas>(
                    &self.canvas
                )
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .center_x()
                    .center_y(),
                    //.style(container::Container::Canvas),
                row![
                    button("Back").padding(8).on_press(Message::ChangeScene(Scenes::Main(None))),
                    button("Save").padding(8).on_press(Message::DoAction(Box::new(DrawingAction::CanvasAction(CanvasAction::Save)))),
                    button("Add layer").padding(8).on_press(Message::DoAction(Box::new(DrawingAction::CanvasAction(CanvasAction::AddLayer)))),
                    Row::with_children(
                        (|layers: usize| {
                            let mut buttons = vec![];
                            for layer in 0..layers.clone() {
                                buttons.push(button(text(format!("Layer {}", layer + 1))).on_press(
                                    Message::DoAction(Box::new(DrawingAction::CanvasAction(CanvasAction::ActivateLayer(layer))))
                                ).into());
                            }

                            buttons
                        }) (self.canvas.get_layer_count())
                    )
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

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum TabIds {
    Tools,
    Style,
}