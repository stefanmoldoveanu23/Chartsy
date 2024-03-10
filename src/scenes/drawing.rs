use std::any::Any;

use iced::{Alignment, Command, Element, Length, Renderer};
use json::object::Object;
use json::JsonValue;
use mongodb::bson::{doc, Bson, Uuid};

use crate::scene::{Scene, Action, Message, SceneOptions, Globals};
use crate::canvas::tools::{line::LinePending, rect::RectPending, triangle::TrianglePending, polygon::PolygonPending, circle::CirclePending, ellipse::EllipsePending};
use crate::canvas::tools::{brush::BrushPending, brushes::{pencil::Pencil, pen::Pen, airbrush::Airbrush, eraser::Eraser}};
use crate::scenes::scenes::Scenes;
use crate::canvas::layer::CanvasAction;
use crate::errors::error::Error;

use crate::theme::Theme;

use crate::mongo::{MongoRequest, MongoRequestType, MongoResponse};
use crate::widgets::tabs::{Tab, Tabs};

/// The [Messages](Action) for the [Drawing] scene:
/// - [None](DrawingAction::None), for when no action is required;
/// - [CanvasAction](DrawingAction::CanvasAction), for when the user interacts with the canvas;
/// to be sent to the [Canvas] instance for handling;
/// - [TabSelection](DrawingAction::TabSelection), which handles the options tab for drawing;
/// - [ErrorHandler(Error)](DrawingAction::ErrorHandler), which handles errors.
#[derive(Clone)]
pub(crate) enum DrawingAction {
    None,
    CanvasAction(CanvasAction),
    TabSelection(TabIds),
    ErrorHandler(Error),
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
/// Split into a section where the user can choose [Tools](crate::canvas::tool::Tool) and
/// modify the drawing [Style](crate::canvas::style::Style), and the [Canvas].
pub struct Drawing {
    canvas: Canvas,
    active_tab: TabIds,
    globals: Globals,
}

impl Drawing {
}

/// Contains the [uuid](Uuid) of the current [Drawing].
#[derive(Debug, Clone, Copy)]
pub struct DrawingOptions {
    uuid: Option<Uuid>,
}

impl DrawingOptions {
    /// Returns a new instance with the given [uuid](Uuid).
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

        let set_tool = Command::perform(
            async {},
            |_| {
                Message::DoAction(Box::new(DrawingAction::CanvasAction(CanvasAction::ChangeTool(Box::new(LinePending::None)))))
            }
        );
        let init_data:Command<Message>=
            if let Some(options) = options {
                options.apply_options(&mut drawing);

                let uuid = drawing.canvas.id.clone();

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
            } else {
                let uuid = drawing.canvas.id.clone();

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
            };

        return (
            drawing,
            Command::batch(
                [
                    set_tool,
                    init_data,
                ]
            )
        )
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
            DrawingAction::ErrorHandler(_) => { Command::none() }
            DrawingAction::None => { Command::none() }
        }
    }

<<<<<<< Updated upstream
    fn view(&self) -> Element<'_, Message, Theme, Renderer> {
        if self.globals.get_window_height() == 0.0 {
            return Element::new(text(""));
        } else {
            return Element::new(text(""));
        }

        /*row![
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
=======
    fn view(&self, globals: &Globals) -> Element<'_, Message, Theme, Renderer> {
        if globals.get_window_height() == 0.0 {
            return Element::new(Text::new(""));
        }

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
                    .on_press(Message::DoAction(Box::new(DrawingAction::PostDrawing)))
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

        Row::with_children(
            vec![
                Tabs::new(
                    vec![
                        Tab::new(
                            TabIds::Tools,
                            Text::new("Tools"),
                            Column::<Message, Theme, Renderer>::with_children(
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
                        ),
                        Tab::new(
                            TabIds::Style,
                            Text::new("Style"),
                            self.canvas
                                .style
                                .view()
                                .map(|update| Message::DoAction(Box::new(
                                    DrawingAction::CanvasAction(CanvasAction::UpdateStyle(update))
                                )))
                        )
                    ],
                    |tab_id| Message::DoAction(Box::new(DrawingAction::TabSelection(*tab_id)))
                )
                    .width(Length::Fixed(250.0))
                    .height(Length::Fill)
                    .active_tab(self.active_tab)
                    .into(),
                Column::with_children(vec![
                    Text::new(format!("{}", self.get_title()))
                        .width(Length::Shrink)
                        .size(50)
                        .into(),
                    Container::new::<&Canvas>(&self.canvas)
                        .width(Length::Fill)
>>>>>>> Stashed changes
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
            .into()*/
    }

    fn get_error_handler(&self, error: Error) -> Box<dyn Action> { Box::new(DrawingAction::ErrorHandler(error)) }

    fn update_globals(&mut self, globals: Globals) {
        self.globals = globals;
    }

    fn clear(&self) { }
}

/// The tabs in the selection section:
/// - [Tools](TabIds::Tools), for selecting the used [Tool](crate::canvas::tool::Tool);
/// - [Style](TabIds::Style), for modifying the tools [Style](crate::canvas::style::Style).
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
pub enum TabIds {
    #[default]
    Tools,
    Style,
}