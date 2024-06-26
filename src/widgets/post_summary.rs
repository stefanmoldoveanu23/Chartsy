use iced::advanced::layout::{Limits, Node};
use iced::advanced::renderer::{Quad, Style};
use iced::advanced::widget::{Operation, Tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget};
use iced::event::Status;
use iced::mouse::{Cursor, Interaction};
use iced::{
    mouse, Alignment, Background, Border, Color, Element, Event, Length, Padding, Point, Rectangle,
    Size, Vector,
};

/// The default padding of the image in the [post summary](PostSummary).
const DEFAULT_PADDING: f32 = 8.0;

/// A widget which represents the summary of the post. Will present the image and basic data.
pub struct PostSummary<'a, Message, Theme, Renderer>
where
    Message: 'a + Clone,
    Renderer: 'a + iced::advanced::Renderer,
    Theme: 'a + StyleSheet,
{
    /// The padding of the image associated to the post.
    padding: Padding,

    /// The summary of the post.
    summary: Element<'a, Message, Theme, Renderer>,

    /// The image associated to the post.
    image: Element<'a, Message, Theme, Renderer>,

    /// Optional message triggered when pressing on the post.
    on_click_data: Option<Message>,

    /// Optional message triggered when pressing on the image.
    on_click_image: Option<Message>,

    /// The style of the [post summary](PostSummary).
    style: <Theme as StyleSheet>::Style,
}

impl<'a, Message, Theme, Renderer> PostSummary<'a, Message, Theme, Renderer>
where
    Message: 'a + Clone,
    Renderer: 'a + iced::advanced::Renderer,
    Theme: 'a + StyleSheet,
{
    /// Creates a new [post summary](PostSummary), given the posts image.
    pub fn new(
        summary: impl Into<Element<'a, Message, Theme, Renderer>>,
        image: impl Into<Element<'a, Message, Theme, Renderer>>,
    ) -> Self {
        PostSummary {
            padding: DEFAULT_PADDING.into(),
            summary: summary.into(),
            image: image.into(),
            on_click_data: None,
            on_click_image: None,
            style: <Theme as StyleSheet>::Style::default(),
        }
    }

    /// Sets the padding of the image.
    pub fn padding(mut self, padding: impl Into<Padding>) -> Self {
        self.padding = padding.into();

        self
    }

    /// Sets the message triggered when pressing on the [post summary](PostSummary).
    pub fn on_click_data(mut self, on_click_data: impl Into<Message>) -> Self {
        self.on_click_data = Some(on_click_data.into());

        self
    }

    /// Sets the message triggered when pressing on the image.
    pub fn on_click_image(mut self, on_click_image: impl Into<Message>) -> Self {
        self.on_click_image = Some(on_click_image.into());

        self
    }

    /// Sets the style of the [post summary](PostSummary).
    pub fn style(mut self, style: impl Into<<Theme as StyleSheet>::Style>) -> Self {
        self.style = style.into();

        self
    }
}

impl<'a, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for PostSummary<'a, Message, Theme, Renderer>
where
    Message: 'a + Clone,
    Renderer: 'a + iced::advanced::Renderer,
    Theme: 'a + StyleSheet,
{
    fn size(&self) -> Size<Length> {
        Size::new(Length::Shrink, Length::Shrink)
    }

    fn layout(&self, tree: &mut Tree, renderer: &Renderer, limits: &Limits) -> Node {
        let padding = self.padding;

        let limits_image = limits
            .loose()
            .width(self.image.as_widget().size().width)
            .height(self.image.as_widget().size().height)
            .shrink(padding);

        let mut image =
            self.image
                .as_widget()
                .layout(&mut tree.children[1], renderer, &limits_image);
        let image_size = image.size();

        let limits_summary = limits
            .loose()
            .width(image_size.width + padding.left + padding.right)
            .height(self.summary.as_widget().size().height)
            .shrink(padding);

        let mut summary =
            self.summary
                .as_widget()
                .layout(&mut tree.children[0], renderer, &limits_summary);
        let summary_size = summary.size();

        summary.move_to_mut(Point::new(padding.left, padding.top));
        summary.align_mut(Alignment::Start, Alignment::Start, summary.size());

        image.move_to_mut(Point::new(
            padding.left,
            2.0 * padding.top + summary_size.height,
        ));
        image.align_mut(Alignment::Center, Alignment::Center, image.size());

        Node::with_children(
            Size::new(
                image_size.width.max(summary_size.width),
                image_size.height + summary_size.height + padding.top,
            )
            .expand(padding),
            vec![summary, image],
        )
    }

    fn draw(
        &self,
        state: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &Style,
        layout: Layout<'_>,
        cursor: Cursor,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();

        let appearance = if cursor.is_over(bounds) {
            theme.hovered(&self.style)
        } else {
            theme.active(&self.style)
        };

        renderer.fill_quad(
            Quad {
                bounds,
                border: Border {
                    color: appearance.border_color,
                    width: 2.0,
                    radius: 10.0.into(),
                },
                shadow: Default::default(),
            },
            appearance.background_color,
        );

        let mut children = layout.children();
        let summary_layout = children.next().expect("Post needs to have summary.");
        self.summary.as_widget().draw(
            &state.children[0],
            renderer,
            theme,
            style,
            summary_layout,
            cursor,
            viewport,
        );

        let image_layout = children.next().expect("Post needs to have image.");
        self.image.as_widget().draw(
            &state.children[1],
            renderer,
            theme,
            style,
            image_layout,
            cursor,
            viewport,
        );
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.summary), Tree::new(&self.image)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&[&self.summary, &self.image]);
    }

    fn operate(
        &self,
        state: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation<Message>,
    ) {
        let mut children = layout.children();

        let summary_layout = children.next().expect("Post needs to have summary.");
        self.summary.as_widget().operate(
            &mut state.children[0],
            summary_layout,
            renderer,
            operation,
        );

        let image_layout = children.next().expect("Post needs to have image.");
        self.image
            .as_widget()
            .operate(&mut state.children[1], image_layout, renderer, operation);
    }

    fn on_event(
        &mut self,
        state: &mut Tree,
        event: Event,
        layout: Layout<'_>,
        cursor: Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) -> Status {
        let bounds = layout.bounds();

        let mut children = layout.children();
        let summary_layout = children.next().expect("Post needs to have summary");
        let image_layout = children.next().expect("Post needs to have image.");
        let image_bounds = image_layout.bounds();

        let result = self.summary.as_widget_mut().on_event(
            &mut state.children[0],
            event.clone(),
            summary_layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );

        if result == Status::Captured {
            return Status::Captured;
        }

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if cursor.is_over(image_bounds) {
                    if let Some(message) = &self.on_click_image {
                        shell.publish(message.clone());
                        return Status::Captured;
                    }
                }

                if cursor.is_over(bounds) {
                    if let Some(message) = &self.on_click_data {
                        shell.publish(message.clone());
                        return Status::Captured;
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
        _renderer: &Renderer,
    ) -> Interaction {
        let bounds = layout.bounds();

        if cursor.is_over(bounds) {
            Interaction::Pointer
        } else {
            Interaction::default()
        }
    }

    fn overlay<'b>(
        &'b mut self,
        state: &'b mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        translation: Vector,
    ) -> Option<iced::advanced::overlay::Element<'b, Message, Theme, Renderer>> {
        let summary_layout = layout
            .children()
            .next()
            .expect("Post needs to have summary");

        self.summary.as_widget_mut().overlay(
            &mut state.children[0],
            summary_layout,
            renderer,
            translation,
        )
    }
}

impl<'a, Message, Theme, Renderer> From<PostSummary<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a + Clone,
    Renderer: 'a + iced::advanced::Renderer,
    Theme: 'a + StyleSheet,
{
    fn from(value: PostSummary<'a, Message, Theme, Renderer>) -> Self {
        Element::new(value)
    }
}

/// The appearance of a [post summary](PostSummary).
#[derive(Debug, Clone, Copy)]
pub struct Appearance {
    /// The background color of the [post summary](PostSummary).
    pub background_color: Background,
    /// The border color of the [post summary](PostSummary).
    pub border_color: Color,
}

impl Default for Appearance {
    fn default() -> Self {
        Appearance {
            background_color: Background::Color(Color::TRANSPARENT),
            border_color: Color::TRANSPARENT,
        }
    }
}

/// The stylesheet of a [post summary](PostSummary).
pub trait StyleSheet {
    /// The style of the [StyleSheet].
    type Style: Default;

    /// Returns the [Appearance] of the [post summary](PostSummary) when it is active.
    fn active(&self, style: &Self::Style) -> Appearance;

    /// Returns the [Appearance] of the [post summary](PostSummary) when it is hovered over.
    fn hovered(&self, style: &Self::Style) -> Appearance {
        Appearance {
            background_color: Background::Color(Color::from_rgba(0.5, 0.5, 0.5, 0.5)),
            ..self.active(style)
        }
    }
}
