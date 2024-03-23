use iced::{Alignment, Background, Color, Element, Event, Length, mouse, Padding, Point, Rectangle, Size, Vector};
use iced::advanced::layout::{Limits, Node};
use iced::advanced::renderer::{Quad, Style};
use iced::advanced::{Clipboard, Layout, Shell, Widget};
use iced::advanced::widget::{Operation, Tree};
use iced::event::Status;
use iced::mouse::{Cursor, Interaction};
use iced::widget::image::Handle;
use crate::widgets::close::Close;

/// The default padding for the content.
const DEFAULT_PADDING :f32= 0.0;
/// The default padding for the [close button](Close).
const DEFAULT_CLOSE_PADDING :f32= 10.0;

/// A [Widget] for a container which can be closed.
pub struct Closeable<'a, Message, Theme, Renderer>
where
    Message: 'a + Clone,
    Renderer: 'a + iced::advanced::Renderer+iced::advanced::image::Renderer<Handle=Handle>,
    Theme: 'a + StyleSheet
{
    /// The width of the [Closeable].
    width: Length,

    /// The height of the [Closeable].
    height: Length,

    /// The horizontal alignment of the [Closeable].
    horizontal_alignment: Alignment,

    /// The vertical alignment of the [Closeable].
    vertical_alignment: Alignment,

    /// THe padding of the content.
    padding: Padding,

    /// The content stored in the [Closeable].
    content: Element<'a, Message, Theme, Renderer>,

    /// Optional message triggered when clicking the content.
    on_click: Option<Message>,

    /// The padding of the [close button](Close).
    close_padding: Padding,

    /// Optional [close button](Close).
    close_button: Option<Element<'a, Message, Theme, Renderer>>,

    /// The [style](StyleSheet::Style) of the [Closeable].
    style: <Theme as StyleSheet>::Style
}

impl<'a, Message, Theme, Renderer> Closeable<'a, Message, Theme, Renderer>
where
    Message: 'a + Clone,
    Renderer: 'a + iced::advanced::Renderer+iced::advanced::image::Renderer<Handle=Handle>,
    Theme: 'a + StyleSheet
{
    /// Instantiates a new [Closeable] with the given content.
    pub fn new(content: impl Into<Element<'a, Message, Theme, Renderer>>) -> Self
    {
        Closeable {
            width: Length::Shrink,
            height: Length::Shrink,
            horizontal_alignment: Alignment::Center,
            vertical_alignment: Alignment::Center,
            padding: DEFAULT_PADDING.into(),
            content: content.into(),
            on_click: None,
            close_padding: DEFAULT_CLOSE_PADDING.into(),
            close_button: None,
            style: <Theme as StyleSheet>::Style::default()
        }
    }

    /// Sets the width of the [Closeable].
    pub fn width(mut self, width: impl Into<Length>) -> Self
    {
        self.width = width.into();

        self
    }

    /// Sets the height of the [Closeable].
    pub fn height(mut self, height: impl Into<Length>) -> Self
    {
        self.height = height.into();

        self
    }

    /// Sets the horizontal alignment of the [Closeable].
    pub fn horizontal_alignment(mut self, horizontal_alignment: impl Into<Alignment>) -> Self
    {
        self.horizontal_alignment = horizontal_alignment.into();

        self
    }

    /// Sets the vertical alignment of the [Closeable].
    pub fn vertical_alignment(mut self, vertical_alignment: impl Into<Alignment>) -> Self
    {
        self.vertical_alignment = vertical_alignment.into();

        self
    }

    /// Sets the padding of the content.
    pub fn padding(mut self, padding: impl Into<Padding>) -> Self
    {
        self.padding = padding.into();

        self
    }

    /// Sets the triggered message for when the content is pressed.
    pub fn on_click(mut self, on_click: impl Into<Message>) -> Self
    {
        self.on_click = Some(on_click.into());

        self
    }

    /// Sets the padding of the [close button](Close).
    pub fn close_padding(mut self, close_padding: impl Into<Padding>) -> Self
    {
        self.close_padding = close_padding.into();

        self
    }

    /// Sets the message triggered when the [close button](Close) is pressed.
    pub fn on_close(mut self, on_close: impl Into<Message>, size: impl Into<f32>) -> Self
    {
        self.close_button = Some(
            Close::new(on_close).size(size.into()).into()
        );

        self
    }

    /// Sets the [style](StyleSheet::Style) of the [Closeable].
    pub fn style(mut self, style: impl Into<<Theme as StyleSheet>::Style>) -> Self
    {
        self.style = style.into();

        self
    }
}

impl<'a, Message, Theme, Renderer> Widget<Message, Theme, Renderer> for Closeable<'a, Message, Theme, Renderer>
where
    Message: 'a + Clone,
    Renderer: 'a + iced::advanced::Renderer+iced::advanced::image::Renderer<Handle=Handle>,
    Theme: 'a + StyleSheet
{
    fn size(&self) -> Size<Length> {
        Size::new(
            self.width,
            self.height
        )
    }

    fn layout(&self, tree: &mut Tree, renderer: &Renderer, limits: &Limits) -> Node {
        let limits = limits
            .loose()
            .width(self.width)
            .height(self.height);

        let content_limits = limits.shrink(self.padding);

        let mut content_node = self.content.as_widget().layout(&mut tree.children[0], renderer, &content_limits);
        let size = limits.resolve(self.width, self.height, content_node.size());

        let mut close_node = if let Some(close_button) = &self.close_button {
            let close_node = close_button.as_widget().layout(
                &mut tree.children[1],
                renderer,
                &limits
            );

            Some(close_node)
        } else {
            None
        };

        if let Some(close_node) = close_node.as_mut() {
            let close_size = close_node.size();
            close_node.align_mut(Alignment::End, Alignment::Start, size);
            close_node.move_to_mut(Point::new(
                size.width - self.close_padding.right - close_size.width,
                self.close_padding.top
            ));
        }

        content_node.move_to_mut(Point::new(self.padding.left, self.padding.top));
        content_node.align_mut(self.horizontal_alignment, self.vertical_alignment, size);

        Node::with_children(
            size.expand(self.padding),
            if let Some(close_node) = close_node {
                vec![content_node, close_node]
            } else {
                vec![content_node]
            }
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
        viewport: &Rectangle
    ) {
        let bounds = layout.bounds();
        let mut children = layout.children();

        let appearance = theme.active(&self.style);

        renderer.fill_quad(
            Quad {
                bounds,
                border: Default::default(),
                shadow: Default::default(),
            },
            appearance.background
        );

        let content_node = children.next().expect("Closeable needs to have content.");
        self.content.as_widget().draw(
            &state.children[0],
            renderer,
            theme,
            style,
            content_node,
            cursor,
            viewport
        );

        if let Some(close_button) = &self.close_button {
            let close_node = children.next().expect("Closeable should have close button");
            close_button.as_widget().draw(
                &state.children[1],
                renderer,
                theme,
                style,
                close_node,
                cursor,
                viewport
            );
        }
    }

    fn children(&self) -> Vec<Tree> {
        if let Some(close_button) = &self.close_button {
            vec![Tree::new(&self.content), Tree::new(close_button)]
        } else {
            vec![Tree::new(&self.content)]
        }
    }

    fn diff(&self, tree: &mut Tree) {
        if let Some(close_button) = self.close_button.as_ref() {
            tree.diff_children(&[&self.content, close_button])
        } else {
            tree.diff_children(&[&self.content])
        }
    }

    fn operate(
        &self,
        state: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation<Message>
    ) {
        let mut children = layout.children();

        let content_node = children.next().expect("Closeable needs to have content.");

        self.content.as_widget().operate(
            &mut state.children[0],
            content_node,
            renderer,
            operation
        );
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
        viewport: &Rectangle
    ) -> Status {
        let mut children = layout.children();

        let content_node = children.next().expect("Closeable needs to have content.");
        let content_status = if let Some(on_click) = &self.on_click {
            let image_bounds = content_node.bounds();
            if cursor.is_over(image_bounds) {
                match event {
                    Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                        shell.publish(on_click.clone());
                        Status::Captured
                    }
                    _ => Status::Ignored
                }
            } else {
                Status::Ignored
            }
        } else {
            self.content.as_widget_mut().on_event(
                &mut state.children[0],
                event.clone(),
                content_node,
                cursor,
                renderer,
                clipboard,
                shell,
                viewport
            )
        };

        let close_status = if let Some(close_button) = self.close_button.as_mut() {
            let close_node = children.next().expect("Image should have close button");
            close_button.as_widget_mut().on_event(
                &mut state.children[1],
                event,
                close_node,
                cursor,
                renderer,
                clipboard,
                shell,
                viewport
            )
        } else {
            Status::Ignored
        };

        content_status.merge(close_status)
    }

    fn mouse_interaction(
        &self,
        state: &Tree,
        layout: Layout<'_>,
        cursor: Cursor,
        viewport: &Rectangle,
        renderer: &Renderer
    ) -> Interaction {
        let mut children = layout.children();

        let content_node = children.next().expect("Closeable needs to have content.");
        let content_interaction = if self.on_click.is_some() {
            let content_bounds = content_node.bounds();
            if cursor.is_over(content_bounds) {
                Interaction::Pointer
            } else {
                Interaction::default()
            }
        } else {
            self.content.as_widget().mouse_interaction(
                &state.children[0],
                content_node,
                cursor,
                viewport,
                renderer
            )
        };

        let mouse_interaction = if let Some(close_button) = self.close_button.as_ref() {
            let close_node = children.next().expect("Image should have a close button");

            close_button.as_widget().mouse_interaction(
                &state.children[1],
                close_node,
                cursor,
                viewport,
                renderer
            )
        } else {
            Interaction::default()
        };

        mouse_interaction.max(content_interaction)
    }

    fn overlay<'b>(
        &'b mut self,
        state: &'b mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        translation: Vector
    ) -> Option<iced::advanced::overlay::Element<'b, Message, Theme, Renderer>> {
        self.content.as_widget_mut().overlay(
            &mut state.children[0],
            layout.children().next().expect("Closeable needs to have content."),
            renderer,
            translation
        )
    }
}

impl<'a, Message, Theme, Renderer> From<Closeable<'a, Message, Theme, Renderer>> for Element<'a, Message, Theme, Renderer>
where
    Message: 'a + Clone,
    Renderer: 'a + iced::advanced::Renderer+iced::advanced::image::Renderer<Handle=Handle>,
    Theme: 'a + StyleSheet
{
    fn from(value: Closeable<'a, Message, Theme, Renderer>) -> Self {
        Element::new(value)
    }
}

/// The appearance of a [Closeable].
pub struct Appearance {
    /// The [Background] of the [Closeable].
    pub(crate) background: Background
}

impl Default for Appearance {
    fn default() -> Self {
        Appearance {
            background: Background::Color(Color::WHITE)
        }
    }
}

/// The style sheet of a [Closeable].
pub trait StyleSheet {
    /// The possible style values; generally an enum.
    type Style: Default;

    /// The appearance of the [Closeable] when it is active.
    fn active(&self, style: &Self::Style) -> Appearance;
}