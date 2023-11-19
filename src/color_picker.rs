use std::fmt::{self, Display, Formatter};
use std::ops::Sub;
use iced::advanced::{Layout, Widget, Renderer, Clipboard, Shell};
use iced::{Background, BorderRadius, Element, event, Event, Length, mouse, Rectangle, Size, Vector};
use iced::advanced::layout::{Limits, Node};
use iced::advanced::renderer::{Quad, Style};
use iced::advanced::widget::{Tree, tree};
use iced::mouse::{Button, Cursor, Interaction};
use iced::widget::Slider;
use crate::theme::Theme;

/// A basic color picker widget.
///
/// Features a section of [colors](Color) to pick from, and a [slider](Slider) for transparency.
pub struct ColorPicker<'a, Message> {
    hovering: Option<iced::Color>,
    on_submit: fn(iced::Color) -> Message,
    slider: Slider<'a, f32, Message, iced::Renderer<Theme>>,
    alpha: f32,
    width: f32,
    height: f32,
}

impl<'a, Message> ColorPicker<'a, Message>
where Message: Clone + 'a {
    /// Computes the grid dimensions for the [ColorPicker], and initializes a new instance
    /// given the submit function.
    pub fn new(on_submit: fn(iced::Color) -> Message) -> Self {
        let row_size = ((250.0 - 10.0) / 40.0) as usize;
        let col_size = (Color::size() as f32 / row_size as f32).ceil() as usize;
        let height = 40.0 * (col_size as f32) + 10.0;

        ColorPicker {
            hovering: None,
            on_submit,
            slider: Slider::new(
                0.0..=255.0,
                255.0,
                move |val| {(on_submit)(iced::Color::new(0.0, 0.0, 0.0, val / 255.0))}),
            alpha: 1.0,
            height,
            width: 250.0
        }
    }

    /// Changes the width of the [ColorPicker].
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;

        let row_size = ((self.width - 10.0) / 40.0) as usize;
        let col_size = (Color::size() as f32 / row_size as f32).ceil() as usize;
        self.height = 40.0 * (col_size as f32) + 10.0;

        self
    }

    /// Changes the [color](iced::Color) of the [ColorPicker].
    pub fn color(mut self, color: iced::Color) -> Self {
        self.alpha = color.a;
        self.slider = Slider::new(
            0.0..=255.0,
            color.a * 255.0,
            move |val| {(self.on_submit)(iced::Color::new(color.r, color.g, color.b, val / 255.0))},
        );

        self
    }
}

impl<'a, Message> Widget<Message, iced::Renderer<Theme>> for ColorPicker<'a, Message>
where Message: Clone+'a {
    fn width(&self) -> Length {
        Length::Fixed(self.width)
    }

    fn height(&self) -> Length {
        Length::Fixed(self.height + 20.0)
    }

    fn layout(&self, _renderer: &iced::Renderer<Theme>, _limits: &Limits) -> Node {
        Node::new(Size::new(self.width, self.height + 20.0))
    }

    fn draw(
        &self,
        state: &Tree,
        renderer: &mut iced::Renderer<Theme>,
        theme: &Theme,
        style: &Style,
        layout: Layout<'_>,
        cursor: Cursor,
        viewport: &Rectangle
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

        self.slider.draw(
            state,
            renderer,
            theme,
            style,
            Layout::with_offset(
                Vector::new(0.0, self.height + bounds.y),
                &Node::new(Size::new(self.width, 20.0))
            ),
            cursor,
            viewport
        );
    }

    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<iced::widget::slider::State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(iced::widget::slider::State::new())
    }

    fn on_event(
        &mut self,
        state: &mut Tree,
        event: Event,
        layout: Layout<'_>,
        cursor: Cursor,
        renderer: &iced::Renderer<Theme>,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) -> event::Status {

        if self.slider.on_event(
            state,
            event.clone(),
            Layout::with_offset(
                Vector::new(0.0, self.height + layout.bounds().y),
                &Node::new(Size::new(self.width, 20.0))
            ),
            cursor,
            renderer,
            clipboard,
            shell,
            viewport
        ) == event::Status::Captured {
            return event::Status::Captured;
        }

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

                            if y * row_size + x < Color::size() {
                                self.hovering = Some(Color::get(y * row_size + x));
                            } else {
                                self.hovering = None;
                            }
                        } else {
                            self.hovering = None;
                        }

                        event::Status::Captured
                    }
                    mouse::Event::ButtonPressed(Button::Left) => {
                        if let Some(color) = self.hovering {
                            shell.publish((self.on_submit)(iced::Color::new(color.r, color.g, color.b, self.alpha)));
                            let on_submit = self.on_submit.clone();

                            self.slider = Slider::new(
                                0.0..=255.0,
                                self.alpha * 255.0,
                                move |val| {(on_submit)(iced::Color::new(color.r, color.g, color.b, val / 255.0))}
                            );
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

    fn mouse_interaction(
        &self,
        state: &Tree,
        layout: Layout<'_>,
        cursor: Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer<Theme>
    ) -> Interaction {
        if self.hovering.is_some() {
            Interaction::Pointer
        } else {
            self.slider.mouse_interaction(
                state,
                Layout::with_offset(
                    Vector::new(0.0, self.height + layout.bounds().y),
                    &Node::new(Size::new(self.width, 20.0))
                ),
                cursor,
                viewport,
                renderer
            )
        }
    }
}

impl<'a, Message: 'a> From<ColorPicker<'a, Message>> for Element<'a, Message, iced::Renderer<Theme>>
where Message: Clone {
    fn from(value: ColorPicker<'a, Message>) -> Self {
        Self::new(value)
    }
}

/// A fixed list of colors to choose from.
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
    /// Turns a [Color] to an [iced::Color].
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

    /// Returns the [iced::Color] corresponding to the grid position.
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

    /// Returns a list of all of the [Color] options.
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

    /// Returns the number of [Color] options.
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