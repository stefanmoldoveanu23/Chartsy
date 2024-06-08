use iced::advanced::layout::{Limits, Node};
use iced::advanced::renderer::Style;
use iced::advanced::widget::Tree;
use iced::advanced::{Clipboard, Layout, Shell, Text, Widget};
use iced::alignment::{Horizontal, Vertical};
use iced::event::Status;
use iced::mouse::{Cursor, Interaction};
use iced::widget::text::{LineHeight, Shaping};
use iced::{mouse, Color, Element, Event, Length, Padding, Point, Rectangle, Size};

use crate::utils::icons::{Icon, ICON};
use iced::advanced::text::Renderer;

/// The default size of a star.
const DEFAULT_SIZE: f32 = 25.0;

/// The default spacing between stars.
const DEFAULT_SPACING: f32 = 5.0;

/// The default padding.
const DEFAULT_PADDING: f32 = 5.0;

pub struct Rating<F, Message>
where
    F: Fn(usize) -> Message,
    Message: Clone,
{
    /// The size of a star.
    size: f32,

    /// The spacing between stars.
    spacing: f32,

    /// The padding.
    padding: Padding,

    /// Action to be triggered when a user has given a rating.
    on_rate: Option<F>,

    /// Action to be triggered when a user has retracted their rating.
    on_unrate: Option<Message>,

    /// The current rating(0 if no rating).
    value: usize,

    /// The rating on which the user is hovering.
    hovered_value: Option<usize>,
}

impl<F, Message> Rating<F, Message>
where
    Message: Clone,
    F: Fn(usize) -> Message,
{
    /// Initializes the empty and full star images and returns a default [Rating].
    pub fn new() -> Self {
        Rating {
            size: DEFAULT_SIZE,
            spacing: DEFAULT_SPACING,
            padding: DEFAULT_PADDING.into(),
            on_rate: None,
            on_unrate: None,
            value: 0,
            hovered_value: None,
        }
    }

    /// Sets the size of a star in the [Rating].
    pub fn size(mut self, size: impl Into<f32>) -> Self {
        self.size = size.into();

        self
    }

    /// Sets the spacing between stars in the [Rating].
    pub fn spacing(mut self, spacing: impl Into<f32>) -> Self {
        self.spacing = spacing.into();

        self
    }

    /// Sets the padding of the [Rating].
    pub fn padding(mut self, padding: impl Into<Padding>) -> Self {
        self.padding = padding.into();

        self
    }

    /// Sets the rating trigger for the [Rating].
    pub fn on_rate(mut self, on_rate: F) -> Self {
        self.on_rate = Some(on_rate);

        self
    }

    /// Sets the rating retraction trigger for the [Rating].
    pub fn on_unrate(mut self, on_unrate: impl Into<Message>) -> Self {
        self.on_unrate = Some(on_unrate.into());

        self
    }

    /// Sets the current rating.
    pub fn value(mut self, value: impl Into<usize>) -> Self {
        self.value = value.into();

        self
    }
}

impl<F, Message, Theme> Widget<Message, Theme, iced::Renderer> for Rating<F, Message>
where
    Message: Clone,
    F: Fn(usize) -> Message,
{
    fn size(&self) -> Size<Length> {
        Size::new(
            Length::Fixed(
                self.size * 5.0 + self.spacing * 4.0 + self.padding.left + self.padding.right,
            ),
            Length::Fixed(self.size + self.padding.top + self.padding.bottom),
        )
    }

    fn layout(&self, _tree: &mut Tree, _renderer: &iced::Renderer, _limits: &Limits) -> Node {
        let size = Size::new(
            self.size * 5.0 + self.spacing * 4.0 + self.padding.left + self.padding.right,
            self.size + self.padding.top + self.padding.bottom,
        );

        let mut nodes: Vec<Node> = vec![];

        for i in 0..5 {
            let mut node = Node::new(Size::new(self.size, self.size));

            node.move_to_mut(Point::new(
                self.padding.left + (i as f32) * (self.size + self.spacing),
                self.padding.top,
            ));

            nodes.push(node);
        }

        Node::with_children(size, nodes)
    }

    fn draw(
        &self,
        _tree: &Tree,
        renderer: &mut iced::Renderer,
        _theme: &Theme,
        _style: &Style,
        layout: Layout<'_>,
        _cursor: Cursor,
        _viewport: &Rectangle,
    ) {
        let mut children = layout.children();

        for i in 0..5 {
            let layout = children
                .next()
                .expect(&*format!("Rating needs to have more than {} children.", i));

            let mut content: String = Icon::StarEmpty.to_string();

            if let Some(hovered_value) = self.hovered_value {
                if hovered_value != self.value {
                    if i < hovered_value {
                        content = Icon::StarFull.to_string();
                    }
                }
            } else {
                if i < self.value {
                    content = Icon::StarFull.to_string();
                }
            }

            renderer.fill_text(
                Text {
                    content,
                    bounds: layout.bounds().size(),
                    size: self.size.into(),
                    font: ICON,
                    line_height: LineHeight::default(),
                    horizontal_alignment: Horizontal::Center,
                    vertical_alignment: Vertical::Center,
                    shaping: Shaping::Basic,
                },
                Point::new(layout.bounds().center_x(), layout.bounds().center_y()),
                Color::from_rgb8(255, 215, 0),
                layout.bounds(),
            );
        }
    }

    fn on_event(
        &mut self,
        _state: &mut Tree,
        event: Event,
        layout: Layout<'_>,
        cursor: Cursor,
        _renderer: &iced::Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) -> Status {
        match event {
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                let bounds = layout.bounds();
                let mut children = layout.children();

                if cursor.is_over(bounds) {
                    for i in 1..=5 {
                        let layout = children.next().expect("Rating needs to have 5 stars.");
                        let bounds = layout.bounds();

                        if cursor.is_over(bounds) {
                            match self.hovered_value {
                                None => {
                                    self.hovered_value = Some(i);
                                }
                                Some(value) => {
                                    if value != i {
                                        self.hovered_value = Some(i);
                                    }
                                }
                            }
                        }
                    }
                } else {
                    if self.hovered_value.is_some() {
                        self.hovered_value = None;
                    }
                }

                Status::Ignored
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if let Some(hovered_value) = self.hovered_value {
                    if self.value == hovered_value {
                        if let Some(on_unrate) = &self.on_unrate {
                            self.value = 0;

                            shell.publish(on_unrate.clone());
                            return Status::Captured;
                        }
                    } else {
                        if let Some(on_rate) = &self.on_rate {
                            self.value = hovered_value;

                            shell.publish(on_rate(self.value).clone());
                            return Status::Captured;
                        }
                    }
                }

                Status::Ignored
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
        _renderer: &iced::Renderer,
    ) -> Interaction {
        if cursor.is_over(layout.bounds()) {
            Interaction::Pointer
        } else {
            Interaction::default()
        }
    }
}

impl<'a, F, Message, Theme> From<Rating<F, Message>> for Element<'a, Message, Theme, iced::Renderer>
where
    Message: 'a + Clone,
    F: 'a + Fn(usize) -> Message,
{
    fn from(value: Rating<F, Message>) -> Self {
        Element::new(value)
    }
}
