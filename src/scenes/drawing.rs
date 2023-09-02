use std::any::Any;
use std::default::Default;

use iced::{Alignment, Element, event, Length, mouse, Point, Rectangle, Renderer, Theme};
use iced::mouse::Cursor;
use iced::widget::{button, text, column, row, canvas, Canvas};
use iced::widget::canvas::{Cache, Event, Frame, Geometry, Path, Stroke};

use crate::scene::{Scene, Action, Message};
use crate::tool::{Tool, Pending};
use crate::tools::{line::LinePending, rect::RectPending};
use crate::scenes::scenes::Scenes;

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
                        let strokes = Path::new(|p| {
                            for tool in self.tools {
                                tool.add_to_path(p);
                            }
                        });

                        frame.stroke(&strokes, Stroke::default().with_width(2.0));
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
                self.state.request_redraw();
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        column![
            text(format!("{}", self.get_title())).width(Length::Shrink).size(50),
            self.state.view(&self.tools, &self.current_tool).map(|tool| {Message::DoAction(Box::new(DrawingAction::UseTool(tool)).into())}),
            row![
                button("Back").padding(8).on_press(Message::ChangeScene(Scenes::Main)),
                button("Line").padding(8).on_press(Message::DoAction(Box::new(DrawingAction::ChangeTool(Box::new(LinePending::None))))),
                button("Rectangle").padding(8).on_press(Message::DoAction(Box::new(DrawingAction::ChangeTool(Box::new(RectPending::None))))),
                ]
        ]
            .padding(20)
            .spacing(20)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_items(Alignment::Center)
            .into()
    }
}