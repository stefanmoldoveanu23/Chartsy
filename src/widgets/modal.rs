use iced::{Alignment, Element, Event, Length, Rectangle, Size, Vector};
use iced::advanced::layout::{Limits, Node};
use iced::advanced::renderer::Style;
use iced::advanced::{Clipboard, Layout, Overlay, overlay, Shell, Widget};
use iced::advanced::widget::{Operation, Tree};
use iced::event::Status;
use iced::mouse::{Cursor, Interaction};

pub struct Modal<'a, Message, Theme, Renderer>
where
    Message: Clone
{
    underlay: Element<'a, Message, Theme, Renderer>,
    overlay: Option<Element<'a, Message, Theme, Renderer>>,
    horizontal_alignment: Alignment,
    vertical_alignment: Alignment,
}

impl<'a, Message, Theme, Renderer> Modal<'a, Message, Theme, Renderer>
where
    Message: Clone,
    Renderer: iced::advanced::Renderer
{
    pub fn new(
        underlay: impl Into<Element<'a, Message, Theme, Renderer>>,
        overlay: Option<impl Into<Element<'a, Message, Theme, Renderer>>>
    ) -> Self {
        Modal {
            underlay: underlay.into(),
            overlay: overlay.map_or_else(|| None, |overlay| Some(overlay.into())),
            horizontal_alignment: Alignment::Center,
            vertical_alignment: Alignment::Center
        }
    }

    pub fn horizontal_alignment(mut self, horizontal_alignment: impl Into<Alignment>) -> Self
    {
        self.horizontal_alignment = horizontal_alignment.into();

        self
    }

    pub fn vertical_alignment(mut self, vertical_alignment: impl Into<Alignment>) -> Self
    {
        self.vertical_alignment = vertical_alignment.into();

        self
    }
}

impl<'a, Message, Theme, Renderer> Widget<Message, Theme, Renderer> for Modal<'a, Message, Theme, Renderer>
where
    Message: Clone,
    Renderer: iced::advanced::Renderer
{
    fn size(&self) -> Size<Length> {
        self.underlay.as_widget().size()
    }

    fn layout(
        &self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &Limits
    ) -> Node {
        self.underlay.as_widget().layout(tree, renderer, limits)
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
        self.underlay.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport
        )
    }

    fn children(&self) -> Vec<Tree> {
        if let Some(overlay) = self.overlay.as_ref() {
            vec![Tree::new(&self.underlay), Tree::new(overlay)]
        } else {
            vec![Tree::new(&self.underlay)]
        }
    }

    fn diff(&self, tree: &mut Tree) {
        if let Some(overlay) = self.overlay.as_ref() {
            tree.diff_children(&[&self.underlay, overlay])
        } else {
            tree.diff_children(&[&self.underlay])
        }
    }

    fn operate(
        &self,
        state: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation<Message>
    ) {
        if let Some(overlay) = self.overlay.as_ref() {
            overlay.as_widget().diff(&mut state.children[1]);

            overlay.as_widget().operate(&mut state.children[1], layout, renderer, operation);
        } else {
            self.underlay.as_widget().operate(&mut state.children[0], layout, renderer, operation);
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
        viewport: &Rectangle
    ) -> Status {
        if self.overlay.is_none() {
            self.underlay.as_widget_mut().on_event(
                &mut state.children[0],
                event,
                layout,
                cursor,
                renderer,
                clipboard,
                shell,
                viewport
            )
        } else {
            Status::Ignored
        }
    }

    fn mouse_interaction(
        &self,
        state: &Tree,
        layout: Layout<'_>,
        cursor: Cursor,
        viewport: &Rectangle,
        renderer: &Renderer
    ) -> Interaction {
        if self.overlay.is_none() {
            self.underlay.as_widget().mouse_interaction(
                &state.children[0],
                layout,
                cursor,
                viewport,
                renderer
            )
        } else {
            Interaction::default()
        }
    }

    fn overlay<'b>(
        &'b mut self,
        state: &'b mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        translation: Vector
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        if let Some(overlay) = self.overlay.as_mut() {
            overlay.as_widget_mut().diff(&mut state.children[1]);

            Some(overlay::Element::new(
                Box::new(
                    ModalOverlay {
                        overlay,
                        state: &mut state.children[1],
                        vertical_alignment: self.vertical_alignment,
                        horizontal_alignment: self.horizontal_alignment,
                    }
                )
            ))
        } else {
            self.underlay.as_widget_mut().overlay(
                &mut state.children[0],
                layout,
                renderer,
                translation
            )
        }

    }
}

impl<'a, Message, Theme, Renderer> From<Modal<'a, Message, Theme, Renderer>> for Element<'a, Message, Theme, Renderer>
where
    Message: 'a + Clone,
    Renderer: 'a + iced::advanced::Renderer,
    Theme: 'a
{
    fn from(value: Modal<'a, Message, Theme, Renderer>) -> Self {
        Element::new(value)
    }
}


struct ModalOverlay<'a, 'b, Message, Theme, Renderer>
where
    Message: Clone,
    Renderer: iced::advanced::Renderer
{
    overlay: &'b mut Element<'a, Message, Theme, Renderer>,
    state: &'b mut Tree,
    vertical_alignment: Alignment,
    horizontal_alignment: Alignment
}

impl<'a, 'b, Message, Theme, Renderer> Overlay<Message, Theme, Renderer> for ModalOverlay<'a, 'b, Message, Theme, Renderer>
where
    Message: Clone,
    Renderer: iced::advanced::Renderer
{
    fn layout(
        &mut self,
        renderer: &Renderer,
        bounds: Size
    ) -> Node {
        let limits = Limits::new(Size::ZERO, bounds);

        Node::with_children(
            bounds,
            vec![self.overlay.as_widget().layout(self.state, renderer, &limits)]
        )
    }

    fn draw(
        &self,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &Style,
        layout: Layout<'_>,
        cursor: Cursor
    ) {
        let overlay_layout = layout.children().next().expect("Error: Modal overlay needs to have element.");

        self.overlay.as_widget().draw(
            self.state,
            renderer,
            theme,
            style,
            overlay_layout,
            cursor,
            &layout.bounds()
        );
    }

    fn on_event(
        &mut self,
        event: Event,
        layout: Layout<'_>,
        cursor: Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>
    ) -> Status {
        self.overlay.as_widget_mut().on_event(
            self.state,
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            &layout.bounds()
        )
    }

    fn mouse_interaction(
        &self,
        layout: Layout<'_>,
        cursor: Cursor,
        viewport: &Rectangle,
        renderer: &Renderer
    ) -> Interaction {
        self.overlay.as_widget().mouse_interaction(
            self.state,
            layout,
            cursor,
            viewport,
            renderer
        )
    }
}