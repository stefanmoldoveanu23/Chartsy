use std::any::Any;
use std::default::Default;
use std::sync::Arc;

use iced::{Alignment, Command, Element, event, Length, mouse, Point, Rectangle, Renderer, Theme};
use iced::mouse::Cursor;
use iced::widget::{button, text, column, row, canvas, Canvas};
use iced::widget::canvas::{Cache, Event, Frame, Geometry, Path, Stroke};

use mongodb::bson::{doc, Document, Uuid};
use mongodb::results::InsertManyResult;

use crate::scene::{Scene, Action, Message, SceneOptions};
use crate::tool::{self, Tool, Pending};
use crate::tools::{line::LinePending, rect::RectPending, triangle::TrianglePending, polygon::PolygonPending, circle::CirclePending, ellipse::EllipsePending};
use crate::tools::{brush::BrushPending, brushes::{pencil::Pencil, pen::Pen, airbrush::Airbrush, eraser::Eraser}};
use crate::scenes::scenes::Scenes;
use crate::menu::menu;

use crate::mongo::{MongoRequest, MongoResponse};

#[derive(Default)]
struct State {
    cache: Cache,
}

unsafe impl Send for State {}
unsafe impl Sync for State {}

impl State {
    pub fn view<'a>(&'a self, tools: &'a [Box<dyn Tool>], current_tool: &'a Box<dyn Pending>) -> Element<'a, Box<dyn Tool>> {
        Canvas::new(DrawingVessel {
            state: Some(self),
            tools,
            current_tool,
        })
            .width(Length::Fill)
            .height(Length::Fill)
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

impl<'a> canvas::Program<Box<dyn Tool>> for DrawingVessel<'a> {
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
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        cursor: Cursor
    ) -> Vec<Geometry> {
        let content = match self.state {
            None => {
                let mut frame = Frame::new(renderer, bounds.size());

                frame.stroke(
                    &Path::rectangle(Point::ORIGIN, frame.size()),
                    Stroke::default().with_width(2.0)
                );

                return vec![frame.into_geometry()];
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
                return vec![content];
            }
            Some(state) => {
                state.draw(renderer, bounds, cursor)
            }
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

impl Scene for Box<Drawing> {
    fn new(options: Option<Box<dyn SceneOptions<Box<Drawing>>>>) -> (Self, Command<Message>) where Self: Sized {
        let mut drawing = Box::new(
            Drawing {
                canvas_id: Uuid::new(),
                state: State::default(),
                tools: Box::new(vec![]),
                count_saved: 0,
                current_tool: Box::new(LinePending::None)
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

    fn view(&self) -> Element<'_, Message> {
        row![
            menu(250, Box::<[(String, Box<[(String, Box<dyn Pending>)]>); 3]>::new(
                    [
                        (String::from("Geometry"), Box::new(
                            [
                                ("Line".into(), Box::new(LinePending::None)),
                                ("Rectangle".into(), Box::new(RectPending::None)),
                                ("Triangle".into(), Box::new(TrianglePending::None)),
                                ("Polygon".into(), Box::new(PolygonPending::None)),
                                ("Circle".into(), Box::new(CirclePending::None)),
                                ("Ellipse".into(), Box::new(EllipsePending::None)),
                            ])),
                        (String::from("Brushes"), Box::new(
                            [
                                ("Pencil".into(), Box::new(BrushPending::<Pencil>::None)),
                                ("Fountain pen".into(), Box::new(BrushPending::<Pen>::None)),
                                ("Airbrush".into(), Box::new(BrushPending::<Airbrush>::None)),
                            ])),
                        (String::from("Eraser"), Box::new(
                            [
                                ("Eraser".into(), Box::new(BrushPending::<Eraser>::None)),
                            ])),
                    ]
                ),
                Box::new(|tool| {Message::DoAction(Box::new(DrawingAction::ChangeTool(tool)))}),
                Message::DoAction(Box::new(DrawingAction::None))
            ),
            column![
                text(format!("{}", self.get_title())).width(Length::Shrink).size(50),
                self.state.view(&self.tools, &self.current_tool).map(|tool| {Message::DoAction(Box::new(DrawingAction::UseTool(tool)).into())}),
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
        ]
            .padding(0)
            .spacing(20)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_items(Alignment::Center)
            .into()
    }

    fn clear(&self) { }
}