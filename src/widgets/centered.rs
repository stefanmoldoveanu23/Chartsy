use iced::{
    advanced::{
        layout::{Limits, Node},
        renderer::Style,
        widget::{Operation, Tree},
        Clipboard, Layout, Shell, Widget,
    },
    event::Status,
    mouse::{Cursor, Interaction},
    Alignment, Element, Event, Length, Rectangle, Size, Vector,
};

/// A widget for content that is centered on the screen with ratios.
pub struct Centered<'a, Message, Theme, Renderer>
where
    Message: 'a + Clone,
{
    /// The content to be centered.
    content: Element<'a, Message, Theme, Renderer>,

    /// The width of the content. Takes value in (0, 1].
    width: f32,

    /// The height of the content. Takes value in (0, 1].
    height: f32,
}

impl<'a, Message, Theme, Renderer> Centered<'a, Message, Theme, Renderer>
where
    Message: 'a + Clone,
{
    /// Centeres the given content with a 1/2 ratio.
    pub fn new(content: impl Into<Element<'a, Message, Theme, Renderer>>) -> Self {
        Centered {
            content: content.into(),
            width: 0.5,
            height: 0.5,
        }
    }

    /// Sets the width of the [Centered].
    pub fn width(mut self, width: impl Into<f32>) -> Self {
        self.width = width.into();

        self
    }

    /// Sets the height of the [Centered].
    pub fn height(mut self, height: impl Into<f32>) -> Self {
        self.height = height.into();

        self
    }
}

impl<'a, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for Centered<'a, Message, Theme, Renderer>
where
    Message: 'a + Clone,
    Renderer: 'a + iced::advanced::Renderer,
{
    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fill)
    }

    fn children(&self) -> Vec<iced::advanced::widget::Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&[&self.content])
    }

    fn layout(&self, tree: &mut Tree, renderer: &Renderer, limits: &Limits) -> Node {
        let size = limits.max();

        let child_size = Size::new(size.width * self.width, size.height * self.height);
        let child_limits = Limits::new(child_size, child_size);

        let mut child_node =
            self.content
                .as_widget()
                .layout(&mut tree.children[0], renderer, &child_limits);

        child_node.align_mut(Alignment::Center, Alignment::Center, size);

        Node::with_children(size, vec![child_node])
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &Style,
        layout: Layout<'_>,
        cursor: Cursor,
        viewport: &Rectangle,
    ) {
        let content_layout = layout
            .children()
            .next()
            .expect("Centered needs content node.");

        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            content_layout,
            cursor,
            viewport,
        );
    }

    fn mouse_interaction(
        &self,
        state: &Tree,
        layout: Layout<'_>,
        cursor: Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> Interaction {
        let content_layout = layout
            .children()
            .next()
            .expect("Centered needs to have content node.");

        if cursor.is_over(content_layout.bounds()) {
            self.content.as_widget().mouse_interaction(
                &state.children[0],
                content_layout,
                cursor,
                viewport,
                renderer,
            )
        } else {
            Interaction::Idle
        }
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
        let content_layout = layout
            .children()
            .next()
            .expect("Centered needs to have content node.");

        if cursor.is_over(content_layout.bounds()) {
            self.content.as_widget_mut().on_event(
                &mut state.children[0],
                event,
                content_layout,
                cursor,
                renderer,
                clipboard,
                shell,
                viewport,
            )
        } else {
            Status::Ignored
        }
    }

    fn operate(
        &self,
        state: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation<Message>,
    ) {
        let content_layout = layout
            .children()
            .next()
            .expect("Centered needs to have content node.");

        self.content.as_widget().operate(
            &mut state.children[0],
            content_layout,
            renderer,
            operation,
        );
    }

    fn overlay<'b>(
        &'b mut self,
        state: &'b mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        translation: Vector,
    ) -> Option<iced::advanced::overlay::Element<'b, Message, Theme, Renderer>> {
        let content_layout = layout
            .children()
            .next()
            .expect("Centered needs to have content node.");

        self.content.as_widget_mut().overlay(
            &mut state.children[0],
            content_layout,
            renderer,
            translation,
        )
    }
}

impl<'a, Message, Theme, Renderer> From<Centered<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a + Clone,
    Theme: 'a,
    Renderer: 'a + iced::advanced::Renderer,
{
    fn from(value: Centered<'a, Message, Theme, Renderer>) -> Self {
        Element::new(value)
    }
}
