use iced::{Color, Element, event, keyboard, Length, Point, Rectangle};
use iced::advanced::mouse;
use iced::mouse::Cursor;
use iced::widget::canvas;
use iced_widget::canvas::fill::Rule;
use crate::theme::Theme;
use crate::tool::{Pending, Tool};

#[derive(Default)]
pub struct State {
    cache: canvas::Cache,
}

unsafe impl Send for State {}
unsafe impl Sync for State {}

impl State {
    pub fn view<'a>(&'a self, tools: &'a [Box<dyn Tool>], current_tool: &'a Box<dyn Pending>) -> Element<'a, CanvasAction, iced_widget::renderer::Renderer<Theme>> {
        canvas::Canvas::new(Canvas {
            state: Some(self),
            tools,
            current_tool,
        })
            .width(Length::Fixed(800.0))
            .height(Length::Fixed(600.0))
            .into()
    }

    pub fn request_redraw(&mut self) {
        self.cache.clear();
    }
}

struct Canvas<'a> {
    state: Option<&'a State>,
    tools: &'a [Box<dyn Tool>],
    current_tool: &'a Box<dyn Pending>,
}

impl<'a> canvas::Program<CanvasAction, iced_widget::renderer::Renderer<Theme>> for Canvas<'a>
{
    type State = Option<Box<dyn Pending>>;

    fn update(
        &self,
        state: &mut Self::State,
        event: canvas::Event,
        bounds: Rectangle,
        cursor: Cursor,
    ) -> (event::Status, Option<CanvasAction>) {
        if let canvas::Event::Keyboard(event) = event {
            match event {
                keyboard::Event::KeyPressed {key_code, modifiers} => {
                    if key_code == keyboard::KeyCode::Z && modifiers == keyboard::Modifiers::CTRL {
                        return (event::Status::Captured, Some(CanvasAction::Undo))
                    } else if key_code == keyboard::KeyCode::S && modifiers == keyboard::Modifiers::CTRL {
                        return (event::Status::Captured, Some(CanvasAction::Save))
                    } else if key_code == keyboard::KeyCode::Y && modifiers == keyboard::Modifiers::CTRL {
                        return (event::Status::Captured, Some(CanvasAction::Redo))
                    }
                }
                _ => {}
            }
        }

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
    ) -> Vec<canvas::Geometry> {
        let base = {
            let mut frame = canvas::Frame::new(renderer, bounds.size());

            frame.fill_rectangle(
                Point::ORIGIN,
                frame.size(),
                canvas::Fill {
                    style: canvas::Style::Solid(Color::WHITE),
                    rule: Rule::NonZero }
            );

            frame.stroke(
                &canvas::Path::rectangle(Point::ORIGIN, frame.size()),
                canvas::Stroke::default().with_width(2.0)
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

#[derive(Clone)]
pub enum CanvasAction {
    UseTool(Box<dyn Tool>),
    Save,
    Undo,
    Redo,
}