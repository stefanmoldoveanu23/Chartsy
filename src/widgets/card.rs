use iced::{Alignment, Background, Border, Element, Event, Length, Padding, Point, Rectangle, Size, Vector};
use iced::advanced::layout::{Limits, Node};
use iced::advanced::renderer::{Quad, Style};
use iced::advanced::{Clipboard, Layout, Shell, Widget};
use iced::advanced::widget::{Operation, Tree};
use iced::border::Radius;
use iced::event::Status;
use iced::mouse::{Cursor, Interaction};

const DEFAULT_PADDING :f32= 10.0;

pub struct Card<'a, Message, Theme, Renderer>
where
    Message: 'a + Clone,
    Theme: 'a + StyleSheet,
    Renderer: 'a + iced::advanced::Renderer
{
    width: Length,
    height: Length,
    header: Element<'a, Message, Theme, Renderer>,
    content: Element<'a, Message, Theme, Renderer>,
    footer: Option<Element<'a, Message, Theme, Renderer>>,
    padding: Padding,
    style: <Theme as StyleSheet>::Style,
}

impl<'a, Message, Theme, Renderer> Card<'a, Message, Theme, Renderer>
where
    Message: 'a + Clone,
    Theme: 'a + StyleSheet,
    Renderer: 'a + iced::advanced::Renderer
{
    pub fn new(
        header: impl Into<Element<'a, Message, Theme, Renderer>>,
        content: impl Into<Element<'a, Message, Theme, Renderer>>
    ) -> Self {
        Card {
            width: Length::Fill,
            height: Length::Shrink,
            header: header.into(),
            content: content.into(),
            footer: None,
            padding: DEFAULT_PADDING.into(),
            style: <Theme as StyleSheet>::Style::default(),
        }
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self
    {
        self.width = width.into();

        self
    }

    pub fn height(mut self, height: impl Into<Length>) -> Self
    {
        self.height = height.into();

        self
    }
    
    pub fn footer(mut self, footer: impl Into<Element<'a, Message, Theme, Renderer>>) -> Self
    {
        self.footer = Some(footer.into());

        self
    }

    pub fn padding(mut self, padding: impl Into<Padding>) -> Self
    {
        self.padding = padding.into();

        self
    }

    pub fn style(mut self, style: impl Into<<Theme as StyleSheet>::Style>) -> Self
    {
        self.style = style.into();

        self
    }
}

impl<'a, Message, Theme, Renderer> Widget<Message, Theme, Renderer> for Card<'a, Message, Theme, Renderer>
where
    Message: 'a + Clone,
    Theme: 'a + StyleSheet,
    Renderer: 'a + iced::advanced::Renderer
{
    fn size(&self) -> Size<Length> {
        Size::new(
            self.width,
            self.height
        )
    }

    fn layout(&self, tree: &mut Tree, renderer: &Renderer, limits: &Limits) -> Node {
        let header_limits = limits
            .loose()
            .width(self.width)
            .height(self.header.as_widget().size().height)
            .shrink(self.padding);

        let mut header_node = self.header.as_widget().layout(
            &mut tree.children[0],
            renderer,
            &header_limits
        );
        let header_size = header_node.size().expand(self.padding);

        header_node.move_to_mut(Point::new(self.padding.left, self.padding.top));
        header_node.align_mut(Alignment::Start, Alignment::Start, header_node.size());

        let content_limits = limits
            .loose()
            .width(self.width)
            .height(self.content.as_widget().size().height)
            .shrink(self.padding);

        let mut content_node = self.content.as_widget().layout(
            &mut tree.children[1],
            renderer,
            &content_limits
        );
        let content_size = content_node.size().expand(self.padding);

        content_node.move_to_mut(Point::new(
            self.padding.left,
            header_size.height + self.padding.top
        ));
        content_node.align_mut(Alignment::Start, Alignment::Start, content_node.size());

        let (footer_node, footer_size) = if let Some(footer) = self.footer.as_ref() {
            let footer_limits = limits
                .loose()
                .width(self.width)
                .height(footer.as_widget().size().height)
                .shrink(self.padding);

            let mut footer_node = footer.as_widget().layout(
                &mut tree.children[2],
                renderer,
                &footer_limits
            );
            let footer_size = footer_node.size().expand(self.padding);

            footer_node.move_to_mut(Point::new(
                self.padding.left,
                header_size.height + content_size.height + self.padding.top
            ));
            footer_node.align_mut(Alignment::Start, Alignment::Start, footer_node.size());

            (footer_node, footer_size)
        } else {
            let footer_node = Node::default();
            (footer_node, Size::ZERO)
        };

        Node::with_children(
            Size::new(
                content_size.width,
                header_size.height + content_size.width + footer_size.width
            ),
            vec![header_node, content_node, footer_node]
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
        let bounds = layout.bounds();

        let appearance = theme.active(&self.style);
        let radii :[f32; 4]= appearance.border.radius.into();

        renderer.fill_quad(
            Quad {
                bounds,
                border: appearance.border,
                shadow: Default::default(),
            },
            appearance.content_background
        );

        let mut children = layout.children();

        let header_layout = children.next().expect("Card needs to have header.");
        let header_bounds = header_layout.bounds().expand(self.padding.top);

        renderer.fill_quad(
            Quad {
                bounds: header_bounds,
                border: Border::with_radius(Radius::from(
                    [radii[0], radii[1], 0.0, 0.0]
                )),
                shadow: Default::default(),
            },
            appearance.header_background
        );

        self.header.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            header_layout,
            cursor,
            viewport
        );

        let content_layout = children.next().expect("Card needs to have content.");
        self.content.as_widget().draw(
            &tree.children[1],
            renderer,
            theme,
            style,
            content_layout,
            cursor,
            viewport
        );

        if let Some(footer) = self.footer.as_ref() {
            let footer_layout = children.next().expect("Card should have footer");
            footer.as_widget().draw(
                &tree.children[2],
                renderer,
                theme,
                style,
                footer_layout,
                cursor,
                viewport
            );
        }
    }

    fn children(&self) -> Vec<Tree> {
        if let Some(footer) = self.footer.as_ref() {
            vec![Tree::new(&self.header), Tree::new(&self.content), Tree::new(footer)]
        } else {
            vec![Tree::new(&self.header), Tree::new(&self.content)]
        }
    }

    fn diff(&self, tree: &mut Tree) {
        if let Some(footer) = self.footer.as_ref() {
            tree.diff_children(&[&self.header, &self.content, footer]);
        } else {
            tree.diff_children(&[&self.header, &self.content]);
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

        let header_layout = children.next().expect("Card needs to have header.");
        self.header.as_widget().operate(
            &mut state.children[0],
            header_layout,
            renderer,
            operation
        );

        let content_layout = children.next().expect("Card needs to have content.");
        self.content.as_widget().operate(
            &mut state.children[1],
            content_layout,
            renderer,
            operation
        );

        if let Some(footer) = self.footer.as_ref() {
            let footer_layout = children.next().expect("Card should have footer.");
            footer.as_widget().operate(
                &mut state.children[2],
                footer_layout,
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

        let header_layout = children.next().expect("Card needs to have header.");
        let header_status = self.header.as_widget_mut().on_event(
            &mut state.children[0],
            event.clone(),
            header_layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport
        );

        let content_layout = children.next().expect("Card needs to have content.");
        let content_status = self.content.as_widget_mut().on_event(
            &mut state.children[1],
            event.clone(),
            content_layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport
        );

        let footer_status = if let Some(footer) = self.footer.as_mut() {
            let footer_layout = children.next().expect("Card should have footer.");
            footer.as_widget_mut().on_event(
                &mut state.children[2],
                event,
                footer_layout,
                cursor,
                renderer,
                clipboard,
                shell,
                viewport
            )
        } else {
            Status::Ignored
        };

        header_status
            .merge(content_status)
            .merge(footer_status)
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

        let header_layout = children.next().expect("Card needs to have header.");
        let header_interaction = self.header.as_widget().mouse_interaction(
            &state.children[0],
            header_layout,
            cursor,
            viewport,
            renderer
        );

        let content_layout = children.next().expect("Card needs to have content.");
        let content_interaction = self.content.as_widget().mouse_interaction(
            &state.children[1],
            content_layout,
            cursor,
            viewport,
            renderer
        );

        let footer_interaction = if let Some(footer) = self.footer.as_ref() {
            let footer_layout = children.next().expect("Card should have footer.");
            footer.as_widget().mouse_interaction(
                &state.children[2],
                footer_layout,
                cursor,
                viewport,
                renderer
            )
        } else {
            Interaction::default()
        };

        header_interaction
            .max(content_interaction)
            .max(footer_interaction)
    }

    fn overlay<'b>(
        &'b mut self,
        state: &'b mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        translation: Vector
    ) -> Option<iced::advanced::overlay::Element<'b, Message, Theme, Renderer>> {
        let mut children = vec![&mut self.header, &mut self.content];
        if let Some(footer) = self.footer.as_mut() {
            children.push(footer);
        }

        let children = children.into_iter()
            .zip(&mut state.children)
            .zip(layout.children())
            .filter_map(|((element, state), layout)| {
                element.as_widget_mut().overlay(
                    state,
                    layout,
                    renderer,
                    translation
                )
            })
            .collect::<Vec<iced::advanced::overlay::Element<'b, Message, Theme, Renderer>>>();

        if children.is_empty() {
            None
        } else {
            Some(iced::advanced::overlay::Group::with_children(children).overlay())
        }
    }
}

impl<'a, Message, Theme, Renderer> From<Card<'a, Message, Theme, Renderer>> for Element<'a, Message, Theme, Renderer>
where
    Message: 'a + Clone,
    Theme: 'a + StyleSheet,
    Renderer: 'a + iced::advanced::Renderer
{
    fn from(value: Card<'a, Message, Theme, Renderer>) -> Self {
        Element::new(value)
    }
}

pub struct Appearance
{
    pub header_background: Background,
    pub content_background: Background,
    pub border: Border,
}

pub trait StyleSheet
{
    type Style: Default;

    fn active(&self, style: &Self::Style) -> Appearance;
}