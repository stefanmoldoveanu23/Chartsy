use iced::{Alignment, Background, Color, Element, Event, Length, mouse, Padding, Point, Rectangle};
use iced::advanced::layout::{Limits, Node};
use iced::advanced::renderer::{Quad, Style};
use iced::advanced::{Clipboard, Layout, Shell, Widget};
use iced::advanced::widget::{Operation, Tree};
use iced::event::Status;
use iced::mouse::{Cursor, Interaction};
use iced::widget::image::Handle;
use crate::widgets::close::Close;

/// The default padding for the [close button](Close).
const DEFAULT_PADDING :f32= 10.0;

/// A [Widget] for a container which can be closed.
pub struct Closeable<'a, Message, Renderer>
where
    Message: 'a+Clone,
    Renderer: 'a+iced::advanced::Renderer+iced::advanced::image::Renderer<Handle=Handle>,
    Renderer::Theme: StyleSheet
{
    /// The width of the [Closeable].
    width: Length,
    /// The height of the [Closeable].
    height: Length,
    /// The content stored in the [Closeable].
    content: Element<'a, Message, Renderer>,
    /// Optional message triggered when clicking the content.
    on_click: Option<Message>,
    /// The padding of the [close button](Close).
    close_padding: Padding,
    /// Optional [close button](Close).
    close_button: Option<Element<'a, Message, Renderer>>,
    /// The [style](StyleSheet::Style) of the [Closeable].
    style: <Renderer::Theme as StyleSheet>::Style
}

impl<'a, Message, Renderer> Closeable<'a, Message, Renderer>
where
    Message: 'a+Clone,
    Renderer: 'a+iced::advanced::Renderer+iced::advanced::image::Renderer<Handle=Handle>,
    Renderer::Theme: StyleSheet
{
    /// Instantiates a new [Closeable] with the given content.
    pub fn new(content: impl Into<Element<'a, Message, Renderer>>) -> Self
    {
        Closeable {
            width: Length::Shrink,
            height: Length::Shrink,
            content: content.into(),
            on_click: None,
            close_padding: DEFAULT_PADDING.into(),
            close_button: None,
            style: <Renderer::Theme as StyleSheet>::Style::default()
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
    pub fn on_close(mut self, on_close: impl Into<Message>) -> Self
    {
        self.close_button = Some(
            Close::new(on_close).into()
        );

        self
    }

    /// Sets the [style](StyleSheet::Style) of the [Closeable].
    pub fn style(mut self, style: impl Into<<Renderer::Theme as StyleSheet>::Style>) -> Self
    {
        self.style = style.into();

        self
    }
}

impl<'a, Message, Renderer> Widget<Message, Renderer> for Closeable<'a, Message, Renderer>
where
    Message: 'a+Clone,
    Renderer: 'a+iced::advanced::Renderer+iced::advanced::image::Renderer<Handle=Handle>,
    Renderer::Theme: StyleSheet
{
    fn width(&self) -> Length {
        self.width
    }

    fn height(&self) -> Length {
        self.height
    }

    fn layout(&self, renderer: &Renderer, limits: &Limits) -> Node {
        let mut limits = limits
            .loose()
            .width(self.width)
            .height(self.height);
        let size = limits.max();

        let mut close_node = if let Some(close_button) = &self.close_button {
            let close_node = close_button.as_widget().layout(
                renderer,
                &limits
            );

            let close_size = close_node.size().pad(self.close_padding);
            limits = limits.shrink(close_size);

            Some(close_node)
        } else {
            None
        };

        let mut content_node = self.content.as_widget().layout(renderer, &limits);

        if let Some(close_node) = close_node.as_mut() {
            let close_size = close_node.size();
            close_node.align(Alignment::End, Alignment::Start, size);
            close_node.move_to(Point::new(
                size.width - self.close_padding.right - close_size.width,
                self.close_padding.top
            ));
        }

        content_node.align(Alignment::Center, Alignment::Center, size);

        Node::with_children(
            size,
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
        theme: &Renderer::Theme,
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
                border_radius: Default::default(),
                border_width: 0.0,
                border_color: Default::default(),
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
}

impl<'a, Message, Renderer> From<Closeable<'a, Message, Renderer>> for Element<'a, Message, Renderer>
where
    Message: 'a+Clone,
    Renderer: 'a+iced::advanced::Renderer+iced::advanced::image::Renderer<Handle=Handle>,
    Renderer::Theme: StyleSheet
{
    fn from(value: Closeable<'a, Message, Renderer>) -> Self {
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