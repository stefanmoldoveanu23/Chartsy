use iced::{Element, Event, Length, Padding, Point, Rectangle, Size, Vector};
use iced::advanced::layout::{Limits, Node};
use iced::advanced::renderer::Style;
use iced::advanced::{Clipboard, Layout, Shell, Widget};
use iced::advanced::widget::{Operation, Tree};
use iced::event::Status;
use iced::mouse::{Cursor, Interaction};

/// The default value of the spacing between the contents of the [Grid].
const DEFAULT_SPACE :f32= 10.0;

/// A grid where contents are displayed horizontally, and then vertically.
pub struct Grid<'a, Message, Theme, Renderer>
where
    Message: 'a + Clone,
    Renderer: 'a + iced::advanced::Renderer,
    Theme: 'a,
{
    /// The width of the [Grid].
    width: Length,

    /// The height of the [Grid].
    height: Length,

    /// The contents of the [Grid].
    elements: Vec<Element<'a, Message, Theme, Renderer>>,

    /// The padding of the contents' container.
    padding: Padding,

    /// The spacing between the contents.
    spacing: f32,
}

impl<'a, Message, Theme, Renderer> Grid<'a, Message, Theme, Renderer>
where
    Message: 'a + Clone,
    Renderer: 'a + iced::advanced::Renderer,
    Theme: 'a,
{
    /// Creates a new [Grid] given the list of contents.
    pub fn new(elements: Vec<impl Into<Element<'a, Message, Theme, Renderer>>>) -> Self
    {
        let mut contents = vec![];
        for element in elements {
            contents.push(element.into());
        }

        Grid {
            width: Length::Fill,
            height: Length::Shrink,
            elements: contents,
            padding: DEFAULT_SPACE.into(),
            spacing: DEFAULT_SPACE,
        }
    }

    /// Sets the width of the [Grid].
    pub fn width(mut self, width: impl Into<Length>) -> Self
    {
        self.width = width.into();

        self
    }

    /// Sets the height of the [Grid].
    pub fn height(mut self, height: impl Into<Length>) -> Self
    {
        self.height = height.into();

        self
    }

    /// Sets the padding of the [Grid].
    pub fn padding(mut self, padding: impl Into<Padding>) -> Self
    {
        self.padding = padding.into();

        self
    }

    /// Sets the spacing of the [Grid].
    pub fn spacing(mut self, spacing: impl Into<f32>) -> Self
    {
        self.spacing = spacing.into();

        self
    }
}

impl<'a, Message, Theme, Renderer> Widget<Message, Theme, Renderer> for Grid<'a, Message, Theme, Renderer>
where
    Message: 'a + Clone,
    Renderer: 'a + iced::advanced::Renderer,
    Theme: 'a,
{
    fn size(&self) -> Size<Length> {
        Size::new(
            self.width,
            self.height
        )
    }

    fn layout(&self, tree: &mut Tree, renderer: &Renderer, limits: &Limits) -> Node {
        let limits = limits.loose()
            .width(self.width)
            .height(self.height)
            .shrink(self.padding);

        let width = limits.max().width;

        let mut pos_x = 0.0;
        let mut pos_y = 0.0;
        let mut max_y = 0.0;

        let mut nodes = vec![];
        let mut index = 0;

        for element in &self.elements {
            let mut node = element.as_widget().layout(&mut tree.children[index], renderer, &limits);
            let size = node.size();

            if pos_x + size.width > width {
                pos_y = pos_y + max_y + self.spacing;
                pos_x = 0.0;
                max_y = 0.0;
            }

            node.move_to_mut(Point::new(pos_x + self.padding.left, pos_y + self.padding.top));
            pos_x = pos_x + size.width + self.spacing;
            max_y = size.height.max(max_y);

            nodes.push(node);
            index += 1;
        }

        Node::with_children(
            Size::new(
                width,
                pos_y + max_y
            )
                .expand(self.padding),
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

        for index in 0..self.elements.len() {
            self.elements[index].as_widget().draw(
                &tree.children[index],
                renderer,
                theme,
                style,
                children.next().expect(&*format!("Grid needs to have at least {} children.", index + 1)),
                cursor,
                viewport
            );
        }
    }

    fn children(&self) -> Vec<Tree> {
        self.elements.iter().map(
            |element| Tree::new(element)
        ).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(
            self.elements.as_slice()
        )
    }

    fn operate(
        &self,
        state: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation<Message>
    ) {
        let mut children = layout.children();

        for index in 0..self.elements.len() {
            self.elements[index].as_widget().operate(
                &mut state.children[index],
                children.next().expect(&*format!("Grid needs to have at least {} children.", index)),
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
        let mut children = layout.children();
        let mut status = Status::Ignored;

        for index in 0..self.elements.len() {
            status = status.merge(self.elements[index].as_widget_mut().on_event(
                &mut state.children[index],
                event.clone(),
                children.next().expect(&*format!("Grid needs to have at least {} children.", index)),
                cursor,
                renderer,
                clipboard,
                shell,
                viewport
            ));
        }

        status
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
        let mut interaction = Interaction::default();

        for index in 0..self.elements.len() {
            interaction = interaction.max(self.elements[index].as_widget().mouse_interaction(
                &state.children[index],
                children.next().expect(&*format!("Grid needs to have at least {} children.", index)),
                cursor,
                viewport,
                renderer
            ));
        }

        interaction
    }

    fn overlay<'b>(
        &'b mut self,
        state: &'b mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        translation: Vector
    ) -> Option<iced::advanced::overlay::Element<'b, Message, Theme, Renderer>> {
        let overlays :Vec<iced::advanced::overlay::Element<'b, Message, Theme, Renderer>>=
            state.children.iter_mut().zip(self.elements.iter_mut().zip(layout.children())).filter_map(
                |(state, (element, layout))|
                element.as_widget_mut().overlay(
                    state,
                    layout,
                    renderer,
                    translation
                )
            ).collect();

        (!overlays.is_empty()).then_some(iced::advanced::overlay::Group::with_children(overlays).overlay())
    }
}

impl<'a, Message, Theme, Renderer> From<Grid<'a, Message, Theme, Renderer>> for Element<'a, Message, Theme, Renderer>
where
    Message: 'a + Clone,
    Renderer: 'a + iced::advanced::Renderer,
    Theme: 'a,
{
    fn from(value: Grid<'a, Message, Theme, Renderer>) -> Self {
        Element::new(value)
    }
}