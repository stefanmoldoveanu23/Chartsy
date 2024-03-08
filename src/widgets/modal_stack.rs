use iced::{Alignment, Background, Color, Element, Event, Length, Point, Rectangle, Renderer, Size};
use iced::advanced::layout::{Limits, Node};
use iced::advanced::renderer::{Quad, Style};
use iced::advanced::{Clipboard, Layout, overlay, Overlay, Shell, Widget};
use iced::advanced::widget::{Operation, Tree};
use iced::event::Status;
use iced::mouse::{Cursor, Interaction};
use iced_aw::modal;
use crate::scene::Message;
use crate::theme::Theme;

/// A structure that can stack overlays on top of each other
/// Useful for scenes with multiple overlays
#[derive(Clone)]
pub struct ModalStack<ModalTypes: Clone + Eq + PartialEq>
{
    stack: Vec<ModalTypes>
}

impl<ModalTypes> ModalStack<ModalTypes>
where
    ModalTypes: Clone+Eq+PartialEq
{
    pub fn new() -> ModalStack<ModalTypes>
    {
        ModalStack {
            stack: vec![]
        }
    }

    /// Attempts to toggle the given modal in the stack.
    /// If the modal is at the top of the stack, the function pops it.
    /// Otherwise, the function pushes it.
    pub fn toggle_modal(&mut self, modal: ModalTypes)
    {
        if let Some(last) = self.stack.last() {
            if modal == last.clone() {
                self.stack.pop();
            } else {
                self.stack.push(modal);
            }
        } else {
            self.stack.push(modal);
        }
    }

    /// Returns an element with the modals overlaid on top of each other.
    pub fn get_modal<'a, F>(&self, underlay: Element<'a, Message, Renderer<Theme>>, into_element: F)
        -> Element<'a, Message, Renderer<Theme>>
    where F: Fn(ModalTypes) -> Element<'a, Message, Renderer<Theme>>
    {
        let modals = self.stack.clone();

        Modal::new(
            underlay,
            modals.iter().map(|modal| into_element(modal.clone())).collect()
        )
            .into()
    }
}

struct Modal<'a, Message, Renderer>
where
    Message: 'a+Clone,
    Renderer: 'a+iced::advanced::Renderer,
    Renderer::Theme: modal::StyleSheet
{
    underlay: Element<'a, Message, Renderer>,
    overlays: Vec<Element<'a, Message, Renderer>>,
}

impl<'a, Message, Renderer> Modal<'a, Message, Renderer>
where
    Message: 'a+Clone,
    Renderer: 'a+iced::advanced::Renderer,
    Renderer::Theme: modal::StyleSheet
{
    fn new(underlay: impl Into<Element<'a, Message, Renderer>>, overlays: Vec<impl Into<Element<'a, Message, Renderer>>>) -> Self
    {
        let mut overs = vec![];
        for overlay in overlays {
            overs.push(overlay.into());
        }

        Modal {
            underlay: underlay.into(),
            overlays: overs,
        }
    }
}

impl<'a, Message, Renderer> Widget<Message, Renderer> for Modal<'a, Message, Renderer>
where
    Message: 'a+Clone,
    Renderer: 'a+iced::advanced::Renderer,
    Renderer::Theme: modal::StyleSheet
{
    fn width(&self) -> Length {
        self.underlay.as_widget().width()
    }

    fn height(&self) -> Length {
        self.underlay.as_widget().height()
    }

    fn layout(&self, renderer: &Renderer, limits: &Limits) -> Node {
        self.underlay.as_widget().layout(renderer, limits)
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
        self.underlay.as_widget().draw(
            &state.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport
        );
    }

    fn children(&self) -> Vec<Tree> {
        let mut children = vec![Tree::new(&self.underlay)];
        for overlay in &self.overlays {
            children.push(Tree::new(overlay));
        }

        children
    }

    fn diff(&self, tree: &mut Tree) {
        let mut children = vec![&self.underlay];
        for overlay in &self.overlays {
            children.push(overlay);
        }

        tree.diff_children(children.as_slice());
    }

    fn operate<'b>(
        &'b self,
        state: &'b mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation<Message>
    ) {
        if let Some(overlay) = self.overlays.last() {
            overlay.as_widget().diff(&mut state.children[self.overlays.len()]);

            overlay.as_widget().operate(
                &mut state.children[self.overlays.len()],
                layout,
                renderer,
                operation
            );
        } else {
            self.underlay.as_widget().operate(
                &mut state.children[0],
                layout,
                renderer,
                operation
            );
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
        if self.overlays.is_empty() {
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
        if self.overlays.is_empty() {
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
        renderer: &Renderer
    ) -> Option<overlay::Element<'b, Message, Renderer>> {
        if !self.overlays.is_empty() {
            let bounds = layout.bounds();
            let position = Point::new(bounds.x, bounds.y);
            self.overlays[0].as_widget().diff(&mut state.children[1]);

            Some(overlay::Element::new(
                position,
                Box::new(ModalOverlay::new(
                    state,
                    &mut self.overlays,
                    0usize
                ))
            ))
        } else {
            self.underlay.as_widget_mut().overlay(&mut state.children[0], layout, renderer)
        }
    }
}

impl<'a, Message, Renderer> From<Modal<'a, Message, Renderer>> for Element<'a, Message, Renderer>
where
    Message: 'a+Clone,
    Renderer: 'a+iced::advanced::Renderer,
    Renderer::Theme: modal::StyleSheet
{
    fn from(value: Modal<'a, Message, Renderer>) -> Self {
        Element::new(value)
    }
}

/// A structure used to propagate [overlays](Overlay) over a [Modal].
struct ModalOverlay<'a, 'b, Message, Renderer>
where
    Message: 'a+Clone,
    Renderer: 'a+iced::advanced::Renderer,
{
    /// A reference to the [state](Tree) of the original [Modal]. Holds the underlay and all overlays.
    state: &'b mut Tree,
    /// A reference to the vector of overlays.
    overlays: &'b mut Vec<Element<'a, Message, Renderer>>,
    /// The index of the current overlay. Is incremented when instantiating its own overlay.
    depth: usize,
}

impl<'a, 'b, Message, Renderer> ModalOverlay<'a, 'b, Message, Renderer>
where
    Message: 'a+Clone,
    Renderer: 'a+iced::advanced::Renderer
{
    /// Creates a new [ModalOverlay] given the data from the [Modal].
    fn new(state: &'b mut Tree, overlays: &'b mut Vec<Element<'a, Message, Renderer>>, depth: impl Into<usize>) -> Self
    {
        ModalOverlay {
            state,
            overlays,
            depth: depth.into()
        }
    }
}

impl<'a, 'b, Message, Renderer> Overlay<Message, Renderer> for ModalOverlay<'a, 'b, Message, Renderer>
where
    Message: 'a+Clone,
    Renderer: 'a+iced::advanced::Renderer
{
    fn layout(&self, renderer: &Renderer, bounds: Size, _position: Point) -> Node {
        let limits = Limits::new(Size::ZERO, bounds);
        let mut underlay = self.overlays.get(self.depth).expect("Wrong depth.").as_widget().layout(renderer, &limits);
        let max_size = limits.max();

        underlay.align(Alignment::Center, Alignment::Center, max_size);

        Node::with_children(max_size, vec![underlay])
    }

    fn draw(
        &self,
        renderer: &mut Renderer,
        theme: &Renderer::Theme,
        style: &Style,
        layout: Layout<'_>,
        cursor: Cursor
    ) {
        let bounds = layout.bounds();

        renderer.fill_quad(
            Quad {
                bounds,
                border_radius: Default::default(),
                border_width: 0.0,
                border_color: Default::default(),
            },
            Background::Color(Color::from_rgba(0.8, 0.8, 0.8, 0.5))
        );

        let underlay_node = layout.children().next().expect("Overlay needs to have content.");

        self.overlays.get(self.depth).expect("Wrong depth.").as_widget().draw(
            &self.state.children[self.depth + 1],
            renderer,
            theme,
            style,
            underlay_node,
            cursor,
            &bounds
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
        let viewport = layout.bounds();

        self.overlays.get_mut(self.depth).expect("Wrong depth.").as_widget_mut().on_event(
            &mut self.state.children[self.depth + 1],
            event,
            layout.children().next().expect("Overlay needs to have content."),
            cursor,
            renderer,
            clipboard,
            shell,
            &viewport
        )
    }

    fn mouse_interaction(
        &self,
        layout: Layout<'_>,
        cursor: Cursor,
        viewport: &Rectangle,
        renderer: &Renderer
    ) -> Interaction {
        self.overlays.get(self.depth).expect("Wrong depth.").as_widget().mouse_interaction(
            &self.state.children[self.depth + 1],
            layout.children().next().expect("Overlay needs to have content."),
            cursor,
            viewport,
            renderer
        )
    }

    fn overlay<'c>(
        &'c mut self,
        layout: Layout<'_>,
        renderer: &Renderer
    ) -> Option<overlay::Element<'c, Message, Renderer>> {
        let overlay = self.overlays.get_mut(self.depth + 1);

        if let Some(overlay) = overlay {
            let bounds = layout.bounds();
            let position = Point::new(bounds.x, bounds.y);
            overlay.as_widget().diff(&mut self.state.children[self.depth + 2]);

            Some(overlay::Element::new(
                position,
                Box::new(ModalOverlay::new(
                    &mut self.state,
                    self.overlays,
                    self.depth + 1
                ))
            ))
        } else {
            let underlay = self.overlays.get_mut(self.depth).expect("Wrong depth.");
            underlay.as_widget_mut().overlay(&mut self.state.children[self.depth + 1], layout, renderer)
        }
    }
}