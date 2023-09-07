use std::any::Any;
use std::default::Default;

use iced::{Alignment, Element, event, Length, mouse, Point, Rectangle, Renderer, Theme};
use iced::mouse::Cursor;
use iced::widget::{button, text, column, row, canvas, Canvas};
use iced::widget::canvas::{Cache, Event, Frame, Geometry, Path, Stroke};

use crate::scene::{Scene, Action, Message};
use crate::tool::{Tool, Pending};
use crate::tools::{line::LinePending, rect::RectPending, triangle::TrianglePending, polygon::PolygonPending, circle::CirclePending, ellipse::EllipsePending};
use crate::tools::{brush::BrushPending, brushes::{pencil::Pencil, pen::Pen, airbrush::Airbrush, eraser::Eraser}};
use crate::scenes::scenes::Scenes;
use crate::menu::menu;

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
    UseTool(Box<dyn Tool>),
    ChangeTool(Box<dyn Pending>),
}

impl Action for DrawingAction {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn get_name(&self) -> String {
        match self {
            DrawingAction::UseTool(_tool) => String::from("Use tool"),
            DrawingAction::ChangeTool(_tool) => String::from("Change tool"),
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
    state: State,
    tools: Box<Vec<Box<dyn Tool>>>,
    current_tool: Box<dyn Pending>,
}

impl Drawing {
    pub fn new() -> Box<Self> {
        Box::new(
            Drawing {
                state: State::default(),
                tools: Box::new(vec![]),
                current_tool: Box::new(LinePending::None)
            }
        )
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
    fn get_title(&self) -> String {
        String::from("Drawing")
    }

    fn update(&mut self, message: Box<dyn Action>) {
        let message: &DrawingAction = message.as_any().downcast_ref::<DrawingAction>().expect("Panic downcasting to DrawingAction");

        match message {
            DrawingAction::UseTool(tool) => {
                self.tools.push(tool.clone());
                self.state.request_redraw();
            }
            DrawingAction::ChangeTool(tool) => {
                self.current_tool = (*tool).boxed_clone();
            }
        }
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
            ),
            column![
                text(format!("{}", self.get_title())).width(Length::Shrink).size(50),
                self.state.view(&self.tools, &self.current_tool).map(|tool| {Message::DoAction(Box::new(DrawingAction::UseTool(tool)).into())}),
                button("Back").padding(8).on_press(Message::ChangeScene(Scenes::Main)),
            ]
        ]
            .padding(0)
            .spacing(20)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_items(Alignment::Center)
            .into()
    }
}