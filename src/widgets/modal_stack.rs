use crate::scene::Message;
use crate::utils::theme::Theme;
use iced::advanced::layout::{Limits, Node};
use iced::advanced::renderer::{Quad, Style};
use iced::advanced::widget::{Operation, Tree};
use iced::advanced::{overlay, Clipboard, Layout, Overlay, Shell, Widget};
use iced::event::Status;
use iced::mouse::{Cursor, Interaction};
use iced::{
    Alignment, Background, Color, Element, Event, Length, Rectangle, Renderer, Size, Vector,
};

/// A structure that can stack overlays on top of each other
/// Useful for scenes with multiple overlays
#[derive(Clone)]
pub struct ModalStack<ModalTypes: Clone + Eq + PartialEq> {
    /// The stack of modals.
    stack: Vec<ModalTypes>,
}

impl<ModalTypes> ModalStack<ModalTypes>
where
    ModalTypes: Clone + Eq + PartialEq,
{
    /// Initializes a [modal stack](ModalStack).
    pub fn new() -> ModalStack<ModalTypes> {
        ModalStack { stack: vec![] }
    }

    /// Attempts to toggle the given modal in the stack.
    /// If the modal is at the top of the stack, the function pops it.
    /// Otherwise, the function pushes it.
    pub fn toggle_modal(&mut self, modal: ModalTypes) {
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
    pub fn get_modal<'a, F>(
        &self,
        underlay: impl Into<Element<'a, Message, Theme, Renderer>>,
        into_element: F,
    ) -> Element<'a, Message, Theme, Renderer>
    where
        F: Fn(ModalTypes) -> Element<'a, Message, Theme, Renderer>,
    {
        let modals = self.stack.clone();

        Modal::new(
            underlay.into(),
            modals
                .iter()
                .map(|modal| into_element(modal.clone()))
                .collect(),
        )
        .into()
    }
}

/// Widget that can handle stacked modals.
struct Modal<'a, Message, Theme, Renderer>
where
    Message: 'a + Clone,
    Renderer: 'a + iced::advanced::Renderer,
    Theme: 'a,
{
    /// The underlay of the modal.
    underlay: Element<'a, Message, Theme, Renderer>,

    /// A list of the overlays, stacked from first to last.
    overlays: Vec<Element<'a, Message, Theme, Renderer>>,
}

impl<'a, Message, Theme, Renderer> Modal<'a, Message, Theme, Renderer>
where
    Message: 'a + Clone,
    Renderer: 'a + iced::advanced::Renderer,
    Theme: 'a,
{
    /// Creates a new [Modal] given the underlay and the overlays.
    fn new(
        underlay: impl Into<Element<'a, Message, Theme, Renderer>>,
        overlays: Vec<impl Into<Element<'a, Message, Theme, Renderer>>>,
    ) -> Self {
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

impl<'a, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for Modal<'a, Message, Theme, Renderer>
where
    Message: 'a + Clone,
    Renderer: 'a + iced::advanced::Renderer,
    Theme: 'a,
{
    fn size(&self) -> Size<Length> {
        self.underlay.as_widget().size()
    }

    fn layout(&self, tree: &mut Tree, renderer: &Renderer, limits: &Limits) -> Node {
        self.underlay
            .as_widget()
            .layout(&mut tree.children[0], renderer, limits)
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
        self.underlay.as_widget().draw(
            &state.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
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
        operation: &mut dyn Operation<Message>,
    ) {
        if let Some(overlay) = self.overlays.last() {
            overlay
                .as_widget()
                .diff(&mut state.children[self.overlays.len()]);

            overlay.as_widget().operate(
                &mut state.children[self.overlays.len()],
                layout,
                renderer,
                operation,
            );
        } else {
            self.underlay
                .as_widget()
                .operate(&mut state.children[0], layout, renderer, operation);
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
        if self.overlays.is_empty() {
            self.underlay.as_widget_mut().on_event(
                &mut state.children[0],
                event,
                layout,
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

    fn mouse_interaction(
        &self,
        state: &Tree,
        layout: Layout<'_>,
        cursor: Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> Interaction {
        if self.overlays.is_empty() {
            self.underlay.as_widget().mouse_interaction(
                &state.children[0],
                layout,
                cursor,
                viewport,
                renderer,
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
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        if !self.overlays.is_empty() {
            self.overlays[0].as_widget().diff(&mut state.children[1]);

            Some(overlay::Element::new(Box::new(ModalOverlay::new(
                state,
                &mut self.overlays,
                0usize,
                translation,
            ))))
        } else {
            self.underlay.as_widget_mut().overlay(
                &mut state.children[0],
                layout,
                renderer,
                translation,
            )
        }
    }
}

impl<'a, Message, Theme, Renderer> From<Modal<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a + Clone,
    Renderer: 'a + iced::advanced::Renderer,
    Theme: 'a,
{
    fn from(value: Modal<'a, Message, Theme, Renderer>) -> Self {
        Element::new(value)
    }
}

/// A structure used to propagate [overlays](Overlay) over a [Modal].
struct ModalOverlay<'a, 'b, Message, Theme, Renderer>
where
    Message: 'a + Clone,
    Renderer: 'a + iced::advanced::Renderer,
    Theme: 'a,
{
    /// A reference to the [state](Tree) of the original [Modal]. Holds the underlay and all overlays.
    state: &'b mut Tree,

    /// A reference to the vector of overlays.
    overlays: &'b mut Vec<Element<'a, Message, Theme, Renderer>>,

    /// The index of the current overlay. Is incremented when instantiating its own overlay.
    depth: usize,

    /// The translation of the overlay.
    translation: Vector,
}

impl<'a, 'b, Message, Theme, Renderer> ModalOverlay<'a, 'b, Message, Theme, Renderer>
where
    Message: 'a + Clone,
    Renderer: 'a + iced::advanced::Renderer,
    Theme: 'a,
{
    /// Creates a new [ModalOverlay] given the data from the [Modal].
    fn new(
        state: &'b mut Tree,
        overlays: &'b mut Vec<Element<'a, Message, Theme, Renderer>>,
        depth: impl Into<usize>,
        translation: impl Into<Vector>,
    ) -> Self {
        ModalOverlay {
            state,
            overlays,
            depth: depth.into(),
            translation: translation.into(),
        }
    }
}

impl<'a, 'b, Message, Theme, Renderer> Overlay<Message, Theme, Renderer>
    for ModalOverlay<'a, 'b, Message, Theme, Renderer>
where
    Message: 'a + Clone,
    Renderer: 'a + iced::advanced::Renderer,
    Theme: 'a,
{
    fn layout(&mut self, renderer: &Renderer, bounds: Size) -> Node {
        let limits = Limits::new(Size::ZERO, bounds);
        let mut underlay = self
            .overlays
            .get(self.depth)
            .expect("Wrong depth.")
            .as_widget()
            .layout(&mut self.state.children[self.depth + 1], renderer, &limits);
        let max_size = limits.max();

        underlay.align_mut(Alignment::Center, Alignment::Center, max_size);

        Node::with_children(max_size, vec![underlay])
    }

    fn draw(
        &self,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &Style,
        layout: Layout<'_>,
        cursor: Cursor,
    ) {
        let bounds = layout.bounds();

        renderer.fill_quad(
            Quad {
                bounds,
                border: Default::default(),
                shadow: Default::default(),
            },
            Background::Color(Color::from_rgba(0.8, 0.8, 0.8, 0.5)),
        );

        let underlay_node = layout
            .children()
            .next()
            .expect("Overlay needs to have content.");

        self.overlays
            .get(self.depth)
            .expect("Wrong depth.")
            .as_widget()
            .draw(
                &self.state.children[self.depth + 1],
                renderer,
                theme,
                style,
                underlay_node,
                cursor,
                &bounds,
            );
    }

    fn on_event(
        &mut self,
        event: Event,
        layout: Layout<'_>,
        cursor: Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
    ) -> Status {
        let viewport = layout.bounds();

        self.overlays
            .get_mut(self.depth)
            .expect("Wrong depth.")
            .as_widget_mut()
            .on_event(
                &mut self.state.children[self.depth + 1],
                event,
                layout
                    .children()
                    .next()
                    .expect("Overlay needs to have content."),
                cursor,
                renderer,
                clipboard,
                shell,
                &viewport,
            )
    }

    fn mouse_interaction(
        &self,
        layout: Layout<'_>,
        cursor: Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> Interaction {
        self.overlays
            .get(self.depth)
            .expect("Wrong depth.")
            .as_widget()
            .mouse_interaction(
                &self.state.children[self.depth + 1],
                layout
                    .children()
                    .next()
                    .expect("Overlay needs to have content."),
                cursor,
                viewport,
                renderer,
            )
    }

    fn overlay<'c>(
        &'c mut self,
        layout: Layout<'_>,
        renderer: &Renderer,
    ) -> Option<overlay::Element<'c, Message, Theme, Renderer>> {
        let overlay = self.overlays.get_mut(self.depth + 1);

        if let Some(overlay) = overlay {
            overlay
                .as_widget()
                .diff(&mut self.state.children[self.depth + 2]);

            Some(overlay::Element::new(Box::new(ModalOverlay::new(
                &mut self.state,
                self.overlays,
                self.depth + 1,
                self.translation,
            ))))
        } else {
            let underlay = self.overlays.get_mut(self.depth).expect("Wrong depth.");
            underlay.as_widget_mut().overlay(
                &mut self.state.children[self.depth + 1],
                layout
                    .children()
                    .next()
                    .expect("Modal needs to have overlay."),
                renderer,
                self.translation,
            )
        }
    }
}
