use iced::{Element, Event, Length, mouse, Point, Rectangle, Size};
use iced::advanced::layout::{Limits, Node};
use iced::advanced::renderer::Style;
use iced::advanced::{Clipboard, Layout, Shell, Widget};
use iced::advanced::widget::Tree;
use iced::event::Status;
use iced::mouse::{Cursor, Interaction};
use iced::widget::Image;
use iced::widget::image::Handle;

/// The default size of a star.
const DEFAULT_SIZE :f32= 25.0;

/// The default spacing between starts.
const DEFAULT_SPACING :f32= 5.0;

pub struct Rating<Message, F>
where
    F: Fn(usize) -> Message,
    Message: Clone
{
    /// The size of a star.
    size: Length,

    /// The spacing between stars.
    spacing: f32,

    /// The image to be used for an empty star.
    star_empty: Image<Handle>,

    /// The image to be used for a full star.
    star_full: Image<Handle>,

    /// Action to be triggered when a user has given a rating.
    on_rate: Option<F>,

    /// Action to be triggered when a user has retracted their rating.
    on_unrate: Option<Message>,

    /// The current rating(0 if no rating).
    value: usize,

    /// The rating on which the user is hovering.
    hovered_value: Option<usize>,
}

impl<Message, F> Rating<Message, F>
where
    Message: Clone,
    F: Fn(usize) -> Message
{
    /// Initializes the empty and full star images and returns a default [Rating].
    pub fn new() -> Self
    {
        let star_empty_memory = Handle::from_path("./src/images/star_empty.png");
        let star_full_memory = Handle::from_path("./src/images/star_full.png");

        Rating {
            size: DEFAULT_SIZE.into(),
            spacing: DEFAULT_SPACING,
            star_empty: Image::new(star_empty_memory.clone()).width(DEFAULT_SIZE).height(DEFAULT_SIZE),
            star_full: Image::new(star_full_memory.clone()).width(DEFAULT_SIZE).height(DEFAULT_SIZE),
            on_rate: None,
            on_unrate: None,
            value: 0,
            hovered_value: None
        }
    }

    /// Sets the size of the stars in the [Rating].
    pub fn size(mut self, size: impl Into<Length>) -> Self
    {
        let size = size.into();

        self.size = size;
        self.star_empty = self.star_empty.width(size.clone()).height(size.clone());
        self.star_full = self.star_full.width(size.clone()).height(size.clone());

        self
    }

    /// Sets the spacing between stars in the [Rating].
    pub fn spacing(mut self, spacing: impl Into<f32>) -> Self
    {
        self.spacing = spacing.into();

        self
    }

    /// Sets the rating trigger for the [Rating].
    pub fn on_rate(mut self, on_rate: F) -> Self
    {
        self.on_rate = Some(on_rate);

        self
    }

    /// Sets the rating retraction trigger for the [Rating].
    pub fn on_unrate(mut self, on_unrate: impl Into<Message>) -> Self
    {
        self.on_unrate = Some(on_unrate.into());

        self
    }

    /// Sets the current rating.
    pub fn value(mut self, value: impl Into<usize>) -> Self
    {
        self.value = value.into();

        self
    }
}

impl<'a, Message, Theme, Renderer, F> Widget<Message, Theme, Renderer> for Rating<Message, F>
where
    Message: 'a + Clone,
    Renderer: 'a + iced::advanced::Renderer + iced::advanced::image::Renderer<Handle=Handle>,
    F: 'a + Fn(usize) -> Message
{
    fn size(&self) -> Size<Length> {
        Size::new(
            self.size,
            self.size
        )
    }

    fn layout(&self, tree: &mut Tree, renderer: &Renderer, limits: &Limits) -> Node {
        let limits = limits
            .loose()
            .width((&self.star_empty as &dyn Widget<Message, Theme, Renderer>).size().width)
            .height((&self.star_empty as &dyn Widget<Message, Theme, Renderer>).size().height);

        let mut nodes :Vec<Node>= vec![];
        let mut width = 0.0;
        let mut height :f32= 0.0;

        for i in 1..=5 {
            let mut node = if i <= self.value {
                (&self.star_full as &dyn Widget<Message, Theme, Renderer>)
                    .layout(&mut tree.children[1], renderer, &limits)
            } else {
                (&self.star_empty as &dyn Widget<Message, Theme, Renderer>)
                    .layout(&mut tree.children[0], renderer, &limits)
            };

            node.move_to_mut(Point::new(width, 0.0));

            width += node.size().width + self.spacing;
            height = height.max(node.size().height);

            nodes.push(node);
        }

        Node::with_children(
            Size::new(
                width - self.spacing,
                height
            ),
            nodes
        )
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &Style,
        layout: Layout<'_>,
        cursor: Cursor,
        viewport: &Rectangle
    ) {
        let mut children = layout.children();
        let value = if let Some(value) = self.hovered_value {
            if value == self.value {
                0
            } else {
                value
            }
        } else {
            self.value
        };

        for i in 1..=5 {
            let layout = children.next().expect("Rating needs to have 5 stars.");

            if i <= value {
                (&self.star_full as &dyn Widget<Message, Theme, Renderer>).draw(
                    &tree.children[1],
                    renderer,
                    theme,
                    style,
                    layout,
                    cursor,
                    viewport
                );
            } else {
                (&self.star_empty as &dyn Widget<Message, Theme, Renderer>).draw(
                    &tree.children[0],
                    renderer,
                    theme,
                    style,
                    layout,
                    cursor,
                    viewport
                );
            }
        }
    }

    fn children(&self) -> Vec<Tree> {
        vec![
            Tree::new(&self.star_empty as &dyn Widget<Message, Theme, Renderer>),
            Tree::new(&self.star_full as &dyn Widget<Message, Theme, Renderer>)
        ]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&[
            &self.star_empty as &dyn Widget<Message, Theme, Renderer>,
            &self.star_full as &dyn Widget<Message, Theme, Renderer>
        ])
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
        _viewport: &Rectangle
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
            _ => Status::Ignored
        }
    }

    fn mouse_interaction(
        &self,
        _state: &Tree,
        layout: Layout<'_>,
        cursor: Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer
    ) -> Interaction {
        if cursor.is_over(layout.bounds()) {
            Interaction::Pointer
        } else {
            Interaction::default()
        }
    }
}

impl<'a, Message, Theme, Renderer, F> From<Rating<Message, F>> for Element<'a, Message, Theme, Renderer>
where
    Message: 'a + Clone,
    Renderer: 'a + iced::advanced::Renderer + iced::advanced::image::Renderer<Handle=Handle>,
    F: 'a + Fn(usize) -> Message
{
    fn from(value: Rating<Message, F>) -> Self {
        Element::new(value)
    }
}