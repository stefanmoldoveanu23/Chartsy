use std::fmt::{self, Display, Formatter};
use std::ops::Sub;
use iced::advanced::{Layout, Widget, Renderer, Clipboard, Shell};
use iced::{Background, BorderRadius, Element, event, Event, Length, mouse, Rectangle, Size, Vector};
use iced::advanced::layout::{Limits, Node};
use iced::advanced::renderer::{Quad, Style};
use iced::advanced::widget::Tree;
use iced::mouse::{Button, Cursor};
use crate::theme::Theme;

pub struct ColorPicker<Message> {
    hovering: Option<iced::Color>,
    on_submit: fn(iced::Color) -> Message,
    width: f32,
    height: f32,
}

impl<Message> ColorPicker<Message> {
    pub fn new(on_submit: fn(iced::Color) -> Message) -> Self {
        let row_size = ((250.0 - 10.0) / 40.0) as usize;
        let col_size = (Color::size() as f32 / row_size as f32).ceil() as usize;
        let height = 40.0 * (col_size as f32) + 10.0;

        ColorPicker { hovering: None, on_submit, height, width: 250.0}
    }

    pub fn width(mut self, width: f32) -> Self {
        self.width = width;

        let row_size = ((self.width - 10.0) / 40.0) as usize;
        let col_size = (Color::size() as f32 / row_size as f32).ceil() as usize;
        self.height = 40.0 * (col_size as f32) + 10.0;

        self
    }
}

impl<Message> Widget<Message, iced::Renderer<Theme>> for ColorPicker<Message> {
    fn width(&self) -> Length {
        Length::Fixed(self.width)
    }

    fn height(&self) -> Length {
        Length::Fixed(self.height)
    }

    fn layout(&self, _renderer: &iced::Renderer<Theme>, _limits: &Limits) -> Node {
        Node::new(Size::new(self.width, self.height))
    }

    fn draw(
        &self,
        _state: &Tree,
        renderer: &mut iced::Renderer<Theme>,
        _theme: &Theme,
        _style: &Style,
        layout: Layout<'_>,
        _cursor: Cursor,
        _viewport: &Rectangle
    ) {
        let bounds = layout.bounds();
        let width = bounds.width;
        let row_size = ((width - 10.0) / 40.0) as usize;

        let mut i :usize= 0;
        let mut j :usize= 0;

        for color in Color::values().iter() {
            renderer.fill_quad(
                Quad {
                    bounds: Rectangle {
                        x: bounds.x + (10 * (j + 1) + 30 * j) as f32,
                        y: bounds.y + (10 * (i + 1) + 30 * i) as f32,
                        width: 30.0,
                        height: 30.0,
                    },
                    border_radius: BorderRadius::from(0.0),
                    border_width: 2.0,
                    border_color: iced::Color::from_rgb8(192, 192, 192),
                },
                Background::Color(color.to_color())
            );

            if j == row_size - 1 {
                j = 0;
                i += 1;
            } else {
                j += 1;
            }
        }
    }

    fn on_event(
        &mut self,
        _state: &mut Tree,
        event: Event,
        layout: Layout<'_>,
        _cursor: Cursor,
        _renderer: &iced::Renderer<Theme>,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) -> event::Status {
        match event {
            Event::Keyboard(_) => event::Status::Ignored,
            Event::Mouse(event) => {
                match event {
                    mouse::Event::CursorMoved { position } => {
                        if !layout.bounds().contains(position) {
                            self.hovering = None;
                            return event::Status::Ignored;
                        }

                        let relative = position.sub(Vector::new(layout.bounds().x, layout.bounds().y));
                        if (relative.x / 40.0).fract() > 0.25 && (relative.y / 40.0).fract() > 0.25 {
                            let row_size = ((layout.bounds().width - 10.0) / 40.0) as usize;
                            let x = (relative.x / 40.0).floor() as usize;
                            let y = (relative.y / 40.0).floor() as usize;

                            self.hovering = Some(Color::get(y * row_size + x));
                        } else {
                            self.hovering = None;
                        }

                        event::Status::Captured
                    }
                    mouse::Event::ButtonPressed(Button::Left) => {
                        if let Some(color) = self.hovering {
                            shell.publish((self.on_submit)(color));
                            event::Status::Captured
                        } else {
                            event::Status::Ignored
                        }
                    }
                    _ => event::Status::Ignored,
                }
            }
            Event::Window(_) => event::Status::Ignored,
            Event::Touch(_) => event::Status::Ignored,
        }
    }
}

impl<'a, Message: 'a> From<ColorPicker<Message>> for Element<'a, Message, iced::Renderer<Theme>> {
    fn from(value: ColorPicker<Message>) -> Self {
        Self::new(value)
    }
}

enum Color {
    BLACK,
    WHITE,
    RED,
    BLUE,
    GREEN,
    YELLOW,
    CYAN,
    PURPLE,
}

impl Color {
    fn to_color(&self) -> iced::Color {
        match self {
            Color::BLACK => iced::Color::BLACK,
            Color::WHITE => iced::Color::WHITE,
            Color::RED => iced::Color::from_rgb8(255, 0, 0),
            Color::BLUE => iced::Color::from_rgb8(0, 0, 255),
            Color::GREEN => iced::Color::from_rgb8(0, 255, 0),
            Color::YELLOW => iced::Color::from_rgb8(255, 255, 0),
            Color::CYAN => iced::Color::from_rgb8(0, 255, 255),
            Color::PURPLE => iced::Color::from_rgb8(128, 0, 128),
        }
    }

    fn get(idx: usize) -> iced::Color {
        match idx {
            0 => Color::BLACK.to_color(),
            1 => Color::WHITE.to_color(),
            2 => Color::RED.to_color(),
            3 => Color::BLUE.to_color(),
            4 => Color::GREEN.to_color(),
            5 => Color::YELLOW.to_color(),
            6 => Color::CYAN.to_color(),
            7 => Color::PURPLE.to_color(),
            _ => Color::BLACK.to_color(),
        }
    }

    fn values() -> Vec<Self> {
        vec![
            Color::BLACK,
            Color::WHITE,
            Color::RED,
            Color::BLUE,
            Color::GREEN,
            Color::YELLOW,
            Color::CYAN,
            Color::PURPLE
        ]
    }

    fn size() -> usize {
        8
    }
}

impl Display for Color {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(
            match self {
                Color::BLACK => "BLACK",
                Color::WHITE => "WHITE",
                Color::RED => "RED",
                Color::BLUE => "BLUE",
                Color::GREEN => "GREEN",
                Color::YELLOW => "YELLOW",
                Color::CYAN => "CYAN",
                Color::PURPLE => "PURPLE",
            }
        )
    }
}