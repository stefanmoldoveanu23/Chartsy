use iced::advanced::layout::{Limits, Node};
use iced::advanced::renderer::{Quad, Style};
use iced::advanced::widget::Tree;
use iced::advanced::{Clipboard, Layout, Shell, Widget};
use iced::alignment::Horizontal;
use iced::event::Status;
use iced::gradient::Linear;
use iced::mouse::{Button, Cursor, Interaction};
use iced::widget::{Column, Row, Slider, Text, TextInput};
use iced::{
    mouse, Alignment, Background, Color, Element, Event, Gradient, Length, Padding, Point,
    Rectangle, Size,
};
use std::f32::consts::PI;

/// The default padding of the [ColorPicker].
const DEFAULT_PADDING: f32 = 8.0;

/// A widget where the user can select any color.
///
/// Comprised of four sections:
/// - The 2d gradient, which selects the colors red and green.
/// - The 1d gradient, which selects the color blue.
/// - The current color, on the right side.
/// - Text inputs for each color at the bottom.
pub struct ColorPicker<Message>
where
    Message: Clone,
{
    /// The R component of the [ColorPicker].
    red: f32,

    /// The G component of the [ColorPicker].
    green: f32,

    /// The B component of the [ColorPicker].
    blue: f32,

    /// The A component of the [ColorPicker].
    alpha: f32,

    /// Tells whether the 2d gradient is currently being updated.
    editing_gradient_2d: bool,

    /// Tells whether the 1d gradient is currently being updated.
    editing_gradient_1d: bool,

    /// The width of the [ColorPicker].
    width: Length,

    /// The height of the [ColorPicker].
    height: Length,

    /// The padding of the [ColorPicker].
    padding: Padding,

    /// The spacing of the [ColorPicker].
    spacing: f32,

    /// The update function of the [ColorPicker].
    on_update: fn(Color) -> Message,
}

impl<Message> ColorPicker<Message>
where
    Message: Clone,
{
    /// Initializes a [ColorPicker] with colors and an update function.
    pub fn new(
        red: f32,
        green: f32,
        blue: f32,
        alpha: f32,
        on_update: fn(Color) -> Message,
    ) -> Self {
        ColorPicker {
            red,
            green,
            blue,
            alpha,
            editing_gradient_2d: false,
            editing_gradient_1d: false,
            width: Length::Shrink,
            height: Length::Shrink,
            padding: DEFAULT_PADDING.into(),
            spacing: DEFAULT_PADDING,
            on_update,
        }
    }

    /// Initializes a [ColorPicker] with u8 colors and an update function.
    pub fn new_rgb8(
        red: u8,
        green: u8,
        blue: u8,
        alpha: u8,
        on_update: fn(Color) -> Message,
    ) -> Self {
        Self::new(
            (red as f32) / 255.0,
            (green as f32) / 255.0,
            (blue as f32) / 255.0,
            (alpha as f32) / 255.0,
            on_update,
        )
    }

    /// Sets the width of the [ColorPicker].
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();

        self
    }

    /// Sets the height of the [ColorPicker].
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();

        self
    }

    /// Sets the padding of the [ColorPicker].
    pub fn padding(mut self, padding: impl Into<Padding>) -> Self {
        self.padding = padding.into();

        self
    }

    /// Sets the spacing of the [ColorPicker].
    pub fn spacing(mut self, spacing: impl Into<f32>) -> Self {
        self.spacing = spacing.into();

        self
    }
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer> for ColorPicker<Message>
where
    Message: Clone,
    Renderer: iced::advanced::Renderer,
{
    fn size(&self) -> Size<Length> {
        Size::new(self.width, self.height)
    }

    fn layout(&self, _tree: &mut Tree, _renderer: &Renderer, limits: &Limits) -> Node {
        let limits_max = limits.max().height.min(limits.max().width);
        let limits = limits
            .max_width(limits_max)
            .max_height(limits_max)
            .shrink(Size::new(
                self.padding.left + self.padding.right,
                self.padding.top + self.padding.bottom,
            ));

        let width_unit = (limits.max().width - self.spacing) / 4.0;
        let height_unit = (limits.max().height - self.spacing) / 4.0;

        let mut gradient_2d = Node::new(Size::new(3.0 * width_unit, 3.0 * height_unit));

        gradient_2d.move_to_mut(Point::new(self.padding.left, self.padding.top));

        let mut gradient_1d = Node::new(Size::new(3.0 * width_unit, height_unit));

        gradient_1d.move_to_mut(Point::new(
            self.padding.left,
            3.0 * height_unit + self.spacing + self.padding.top,
        ));

        let mut color = Node::new(Size::new(width_unit, 4.0 * height_unit + self.spacing));

        color.move_to_mut(Point::new(
            3.0 * width_unit + self.spacing + self.padding.left,
            self.padding.top,
        ));

        Node::with_children(
            limits.max().expand(Size::new(
                self.padding.left + self.padding.right,
                self.padding.top + self.padding.bottom,
            )),
            vec![gradient_2d, gradient_1d, color],
        )
    }

    fn draw(
        &self,
        _tree: &Tree,
        renderer: &mut Renderer,
        _theme: &Theme,
        _style: &Style,
        layout: Layout<'_>,
        _cursor: Cursor,
        _viewport: &Rectangle,
    ) {
        let mut children = layout.children();

        let layout_2d = children
            .next()
            .expect("ColorPicker needs to have gradient 2d.");
        let bounds_2d = layout_2d.bounds();
        let unit_height = bounds_2d.height / 256.0;
        let unit_width = bounds_2d.width / 256.0;

        for i in 0..256 {
            renderer.fill_quad(
                Quad {
                    bounds: Rectangle {
                        height: unit_height,
                        y: unit_height * (i as f32) + bounds_2d.y,
                        ..bounds_2d
                    },
                    ..Default::default()
                },
                Background::Gradient(Gradient::Linear(
                    Linear::new(PI / 2.0)
                        .add_stop(0.0, Color::from_rgb((i as f32) / 255.0, 0.0, self.blue))
                        .add_stop(1.0, Color::from_rgb((i as f32) / 255.0, 1.0, self.blue)),
                )),
            );
        }

        renderer.fill_quad(
            Quad {
                bounds: Rectangle {
                    x: bounds_2d.x + self.green * bounds_2d.width - unit_width,
                    width: 2.0 * unit_width,
                    ..bounds_2d
                },
                ..Default::default()
            },
            Background::Gradient(Gradient::Linear(
                Linear::new(PI)
                    .add_stop(0.0, Color::from_rgb(1.0, 1.0 - self.green, 1.0 - self.blue))
                    .add_stop(1.0, Color::from_rgb(0.0, 1.0 - self.green, 1.0 - self.blue)),
            )),
        );

        renderer.fill_quad(
            Quad {
                bounds: Rectangle {
                    y: bounds_2d.y + self.red * bounds_2d.height - unit_height,
                    height: 2.0 * unit_height,
                    ..bounds_2d
                },
                ..Default::default()
            },
            Background::Gradient(Gradient::Linear(
                Linear::new(PI / 2.0)
                    .add_stop(0.0, Color::from_rgb(1.0 - self.red, 1.0, 1.0 - self.blue))
                    .add_stop(1.0, Color::from_rgb(1.0 - self.red, 0.0, 1.0 - self.blue)),
            )),
        );

        let layout_1d = children.next().expect("ColorPicker needs gradient 1d.");
        let bounds_1d = layout_1d.bounds();

        renderer.fill_quad(
            Quad {
                bounds: bounds_1d,
                ..Default::default()
            },
            Background::Gradient(Gradient::Linear(
                Linear::new(PI / 2.0)
                    .add_stop(0.0, Color::from_rgb(self.red, self.green, 0.0))
                    .add_stop(1.0, Color::from_rgb(self.red, self.green, 1.0)),
            )),
        );

        renderer.fill_quad(
            Quad {
                bounds: Rectangle {
                    x: bounds_1d.x + self.blue * bounds_1d.width - unit_width,
                    width: 2.0 * unit_width,
                    ..bounds_1d
                },
                ..Default::default()
            },
            Background::Color(Color::from_rgb(
                1.0 - self.red,
                1.0 - self.green,
                1.0 - self.blue,
            )),
        );

        let color_layout = children.next().expect("ColorPicker needs color.");
        renderer.fill_quad(
            Quad {
                bounds: color_layout.bounds(),
                ..Default::default()
            },
            Background::Color(Color::from_rgb(self.red, self.green, self.blue)),
        );
    }

    fn on_event(
        &mut self,
        _state: &mut Tree,
        event: Event,
        layout: Layout<'_>,
        cursor: Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) -> Status {
        let mut children = layout.children();

        let layout_2d = children
            .next()
            .expect("ColorPicker needs to have gradient 2d.");
        let layout_1d = children
            .next()
            .expect("ColorPicker needs to have gradient 1d.");

        let bounds_2d = layout_2d.bounds();
        let bounds_1d = layout_1d.bounds();

        let over_gradient_2d = cursor.position_over(bounds_2d).is_some();
        let over_gradient_1d = cursor.position_over(bounds_1d).is_some();

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(Button::Left)) => {
                if over_gradient_2d {
                    self.editing_gradient_2d = true;
                    Status::Captured
                } else if over_gradient_1d {
                    self.editing_gradient_1d = true;
                    Status::Captured
                } else {
                    Status::Ignored
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(Button::Left)) => {
                if self.editing_gradient_2d || self.editing_gradient_1d {
                    shell.publish((self.on_update)(Color::from_rgb(
                        self.red, self.green, self.blue,
                    )));
                    self.editing_gradient_2d = false;
                    self.editing_gradient_1d = false;

                    Status::Captured
                } else {
                    Status::Ignored
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                let position = cursor.position();

                if position.is_none() {
                    return Status::Ignored;
                }

                let position = position.unwrap();

                if self.editing_gradient_2d {
                    let position = Point::new(
                        position
                            .x
                            .max(bounds_2d.x)
                            .min(bounds_2d.x + bounds_2d.width),
                        position
                            .y
                            .max(bounds_2d.y)
                            .min(bounds_2d.y + bounds_2d.height),
                    );

                    let position = Point::new(
                        (position.x - bounds_2d.x) / bounds_2d.width,
                        (position.y - bounds_2d.y) / bounds_2d.height,
                    );
                    self.red = position.y;
                    self.green = position.x;

                    Status::Captured
                } else if self.editing_gradient_1d {
                    let position_x = position
                        .x
                        .max(bounds_1d.x)
                        .min(bounds_1d.x + bounds_1d.width);
                    let position_x = (position_x - bounds_1d.x) / bounds_1d.width;

                    self.blue = position_x;

                    Status::Captured
                } else {
                    Status::Ignored
                }
            }
            _ => Status::Ignored,
        }
    }

    fn mouse_interaction(
        &self,
        _state: &Tree,
        layout: Layout<'_>,
        cursor: Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> Interaction {
        let mut children = layout.children();

        let layout_2d = children
            .next()
            .expect("ColorPicker should have gradient 2d.");
        let layout_1d = children
            .next()
            .expect("ColorPicker should have gradient 1d.");

        let bounds_2d = layout_2d.bounds();
        let bounds_1d = layout_1d.bounds();

        if self.editing_gradient_1d || self.editing_gradient_2d {
            Interaction::Grabbing
        } else if cursor.is_over(bounds_2d) || cursor.is_over(bounds_1d) {
            Interaction::Crosshair
        } else {
            Interaction::default()
        }
    }
}

impl<'a, Message, Theme, Renderer> From<ColorPicker<Message>>
    for Element<'a, Message, Theme, Renderer>
where
    Renderer: 'a + iced::advanced::Renderer + iced::advanced::text::Renderer,
    Theme: 'a
        + iced::widget::text::Catalog
        + iced::widget::text_input::Catalog
        + iced::widget::slider::Catalog,
    Message: 'a + Clone,
{
    fn from(value: ColorPicker<Message>) -> Self {
        let red_default = (value.red * 255.0) as u8;
        let green_default = (value.green * 255.0) as u8;
        let blue_default = (value.blue * 255.0) as u8;
        let alpha = (value.alpha * 255.0) as u8;

        let parse_input = move |mut input: String, index: usize| {
            if input == "" {
                input = String::from("0");
            }
            let parsed = input.parse::<u8>();

            if let Ok(number) = parsed {
                match index {
                    0 => (value.on_update)(Color::from_rgb8(number, green_default, blue_default)),
                    1 => (value.on_update)(Color::from_rgb8(red_default, number, blue_default)),
                    2 => (value.on_update)(Color::from_rgb8(red_default, green_default, number)),
                    _ => (value.on_update)(Color::from_rgb8(
                        red_default,
                        green_default,
                        blue_default,
                    )),
                }
            } else {
                (value.on_update)(Color::from_rgb8(red_default, green_default, blue_default))
            }
        };

        let send_alpha = move |alpha| {
            (value.on_update)(Color::from_rgba8(
                red_default,
                green_default,
                blue_default,
                (alpha as f32) / 255.0,
            ))
        };

        let red_text = if red_default == 0 {
            String::from("")
        } else {
            red_default.to_string()
        };
        let green_text = if green_default == 0 {
            String::from("")
        } else {
            green_default.to_string()
        };
        let blue_text = if blue_default == 0 {
            String::from("")
        } else {
            blue_default.to_string()
        };

        Column::with_children(vec![
            Element::new(value),
            Row::with_children(vec![
                Row::with_children(vec![
                    Text::new("R:").into(),
                    TextInput::new("0", &red_text)
                        .on_input(move |input| parse_input(input, 0))
                        .into(),
                ])
                .align_items(Alignment::Center)
                .spacing(2.0)
                .into(),
                Row::with_children(vec![
                    Text::new("G:").into(),
                    TextInput::new("0", &green_text)
                        .on_input(move |input| parse_input(input, 1))
                        .into(),
                ])
                .align_items(Alignment::Center)
                .spacing(2.0)
                .into(),
                Row::with_children(vec![
                    Text::new("B:").into(),
                    TextInput::new("0", &blue_text)
                        .on_input(move |input| parse_input(input, 2))
                        .into(),
                ])
                .align_items(Alignment::Center)
                .spacing(2.0)
                .into(),
            ])
            .align_items(Alignment::Center)
            .spacing(5.0)
            .padding(DEFAULT_PADDING)
            .into(),
            Row::with_children(vec![
                Text::new("A:").into(),
                Slider::new(0..=255, alpha, move |alpha| send_alpha(alpha)).into(),
                Text::new(alpha)
                    .width(Length::Fixed(25.0))
                    .horizontal_alignment(Horizontal::Right)
                    .into(),
            ])
            .align_items(Alignment::Center)
            .spacing(2.0)
            .padding(DEFAULT_PADDING)
            .into(),
        ])
        .into()
    }
}
