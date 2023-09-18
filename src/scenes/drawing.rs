use std::any::Any;
use std::default::Default;
use std::sync::Arc;

use iced::{Alignment, Color, Command, Element, event, Length, mouse, Point, Rectangle, Renderer};
use iced::alignment::Horizontal;
use iced::mouse::Cursor;
use iced::widget::{button, text, column, row, canvas, Canvas, Container};
use iced::widget::canvas::{Cache, Event, Frame, Geometry, Path, Stroke};
use iced_aw::card::Card;
use iced_widget::canvas::{Fill, Style};
use iced_widget::canvas::fill::Rule;

use mongodb::bson::{doc, Document, Uuid};
use mongodb::results::InsertManyResult;

use crate::scene::{Scene, Action, Message, SceneOptions, Globals};
use crate::tool::{self, Tool, Pending};
use crate::tools::{line::LinePending, rect::RectPending, triangle::TrianglePending, polygon::PolygonPending, circle::CirclePending, ellipse::EllipsePending};
use crate::tools::{brush::BrushPending, brushes::{pencil::Pencil, pen::Pen, airbrush::Airbrush, eraser::Eraser}};
use crate::scenes::scenes::Scenes;

use crate::theme::{container, Theme};

use crate::mongo::{MongoRequest, MongoResponse};

#[derive(Default)]
struct State {
    cache: Cache,
}

unsafe impl Send for State {}
unsafe impl Sync for State {}

impl State {
    pub fn view<'a>(&'a self, tools: &'a [Box<dyn Tool>], current_tool: &'a Box<dyn Pending>) -> Element<'a, Box<dyn Tool>, iced_widget::renderer::Renderer<Theme>> {
        Canvas::new(DrawingVessel {
            state: Some(self),
            tools,
            current_tool,
        })
            .width(Length::Fixed(50.0))
            .height(Length::Fixed(50.0))
            .into()
    }

    pub fn request_redraw(&mut self) {
        self.cache.clear();
    }
}

#[derive(Clone)]
enum DrawingAction {
    None,
    UseTool(Box<dyn Tool>),
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
            DrawingAction::UseTool(_tool) => String::from("Use tool"),
            DrawingAction::ChangeTool(_tool) => String::from("Change tool"),
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

struct DrawingVessel<'a> {
    state: Option<&'a State>,
    tools: &'a [Box<dyn Tool>],
    current_tool: &'a Box<dyn Pending>,
}

impl<'a> canvas::Program<Box<dyn Tool>, iced_widget::renderer::Renderer<Theme>> for DrawingVessel<'a>
{
    type State = Option<Box<dyn Pending>>;

    fn update(
        &self,
        state: &mut Self::State,
        event: Event,
        bounds: Rectangle,
        cursor: Cursor,
    ) -> (event::Status, Option<Box<dyn Tool>>) {
        let cursor_position =
            if let Some(position) = cursor.position_in(bounds) {
                position
            } else {
                return (event::Status::Ignored, None);
            };

        match state {
            None => {
                *state = Some((*self.current_tool).boxed_clone());
                (event::Status::Captured, None)
            },
            Some(pending_state) => {
                let new_tool = pending_state.id() != self.current_tool.id();
                if new_tool {
                    *state = Some((*self.current_tool).boxed_clone());
                    (event::Status::Ignored, None)
                } else {
                    pending_state.update(event, cursor_position)
                }
            }
        }
    }

    fn draw(
        &self,
        state: &Self::State,
        renderer: &iced_widget::renderer::Renderer<Theme>,
        _theme: &Theme,
        bounds: Rectangle,
        cursor: Cursor
    ) -> Vec<Geometry> {
        let base = {
            let mut frame = Frame::new(renderer, bounds.size());

            frame.fill_rectangle(Point::ORIGIN, frame.size(), Fill { style: Style::Solid(Color::WHITE), rule: Rule::NonZero });

            frame.stroke(
                &Path::rectangle(Point::ORIGIN, frame.size()),
                Stroke::default().with_width(2.0)
            );

            frame.into_geometry()
        };

        let content = match self.state {
            None => {
                return vec![base];
            }
            Some(state) => {
                state.cache.draw(
                    renderer,
                    bounds.size(),
                    |frame| {
                        for tool in self.tools {
                            tool.add_to_frame(frame);
                        }
                    }
                )
            }
        };

        let pending = match state {
            None => {
                return vec![base, content];
            }
            Some(state) => {
                state.draw(renderer, bounds, cursor)
            }
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

impl Scene for Box<Drawing> {
    fn new(options: Option<Box<dyn SceneOptions<Box<Drawing>>>>, globals: Globals) -> (Self, Command<Message>) where Self: Sized {
        let mut drawing = Box::new(
            Drawing {
                canvas_id: Uuid::new(),
                state: State::default(),
                tools: Box::new(vec![]),
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
                        Message::SendMongoRequest((
                            "tools".into(),
                            MongoRequest::Get(doc! {"canvas_id": uuid}),
                            move |res| {
                                if let MongoResponse::Get(cursor) = res {
                                    Box::new(DrawingAction::Loaded(cursor))
                                } else {
                                    Box::new(DrawingAction::None)
                                }
                            }
                        ))
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
                            Message::SendMongoRequest((
                                "canvases".into(),
                                MongoRequest::Insert(vec![doc!{"id": uuid}]),
                                |_| Box::new(DrawingAction::None),
                                ))
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
            DrawingAction::UseTool(tool) => {
                self.tools.push(tool.clone());
                self.state.request_redraw();
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
                    self.state.view(&self.tools, &self.current_tool).map(|tool| {Message::DoAction(Box::new(DrawingAction::UseTool(tool)).into())})
                )
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .center_x()
                    .center_y()
                    .style(container::Container::Canvas),
                row![
                    button("Back").padding(8).on_press(Message::ChangeScene(Scenes::Main(None))),
                    button("Save").padding(8).on_press(Message::SendMongoRequest(
                        (
                            "tools".into(),
                            MongoRequest::Insert(self.get_tools_serialized()),
                            |response| {
                                match response {
                                    MongoResponse::Insert(result) => Box::new(DrawingAction::Saved(Arc::new(result))),
                                    _ => Box::new(DrawingAction::None),
                                }
                            }
                        )
                    )),
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