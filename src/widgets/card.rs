use iced::{Alignment, Background, Border, Color, Element, Event, Length, Padding, Point, Rectangle, Size, Vector};
use iced::advanced::layout::{Limits, Node};
use iced::advanced::renderer::{Quad, Style};
use iced::advanced::{Clipboard, Layout, Shell, Widget};
use iced::advanced::widget::{Operation, Tree};
use iced::border::Radius;
use iced::event::Status;
use iced::mouse::{Cursor, Interaction};

const DEFAULT_PADDING :f32= 10.0;

/// A container with a header, content and optional footer sections.
pub struct Card<'a, Message, Theme, Renderer>
where
    Message: 'a + Clone,
    Renderer: 'a + iced::advanced::Renderer,
    Theme: 'a + StyleSheet
{
    /// The width of the [Card].
    width: Length,

    /// The height of the [Card].
    height: Length,

    /// The header of the [Card].
    header: Element<'a, Message, Theme, Renderer>,

    /// The content of the [Card].
    content: Element<'a, Message, Theme, Renderer>,

    /// The optional footer of the [Card].
    footer: Option<Element<'a, Message, Theme, Renderer>>,

    /// The padding of the header.
    header_padding: Padding,

    /// The padding of the content.
    content_padding: Padding,

    /// The padding of the footer.
    footer_padding: Padding,

    /// The style of the [Card].
    style: <Theme as StyleSheet>::Style,
}

impl<'a, Message, Theme, Renderer> Card<'a, Message, Theme, Renderer>
where
    Message: 'a + Clone,
    Renderer: 'a + iced::advanced::Renderer,
    Theme: 'a + StyleSheet
{
    /// Creates a new [Card].
    pub fn new(
        header: impl Into<Element<'a, Message, Theme, Renderer>>,
        content: impl Into<Element<'a, Message, Theme, Renderer>>
    ) -> Self {
        Card {
            width: Length::Shrink,
            height: Length::Shrink,
            header: header.into(),
            content: content.into(),
            footer: None,
            header_padding: DEFAULT_PADDING.into(),
            content_padding: DEFAULT_PADDING.into(),
            footer_padding: DEFAULT_PADDING.into(),
            style: <Theme as StyleSheet>::Style::default(),
        }
    }

    /// Sets the width of the [Card].
    pub fn width(mut self, width: impl Into<Length>) -> Self
    {
        self.width = width.into();

        self
    }

    /// Sets the height of the [Card].
    pub fn height(mut self, height: impl Into<Length>) -> Self
    {
        self.height = height.into();

        self
    }

    /// Adds a footer to the [Card].
    pub fn footer(mut self, footer: impl Into<Element<'a, Message, Theme, Renderer>>) -> Self
    {
        self.footer = Some(footer.into());

        self
    }

    /// Sets the padding of the header.
    pub fn header_padding(mut self, header_padding: impl Into<Padding>) -> Self
    {
        self.header_padding = header_padding.into();

        self
    }

    /// Sets the padding of the content.
    pub fn content_padding(mut self, content_padding: impl Into<Padding>) -> Self
    {
        self.content_padding = content_padding.into();

        self
    }

    /// Sets the padding of the footer.
    pub fn footer_padding(mut self, footer_padding: impl Into<Padding>) -> Self
    {
        self.footer_padding = footer_padding.into();

        self
    }

    /// Sets the style of the [Card].
    pub fn style(mut self, style: impl Into<<Theme as StyleSheet>::Style>) -> Self
    {
        self.style = style.into();

        self
    }
}

impl<'a, Message, Theme, Renderer> Widget<Message, Theme, Renderer> for Card<'a, Message, Theme, Renderer>
where
    Message: 'a + Clone,
    Renderer: 'a + iced::advanced::Renderer,
    Theme: 'a + StyleSheet
{
    fn size(&self) -> Size<Length> {
        Size::new(
            self.width,
            self.height
        )
    }

    fn layout(&self, tree: &mut Tree, renderer: &Renderer, limits: &Limits) -> Node {
        let limits_header = limits
            .loose()
            .width(self.width)
            .height(self.header.as_widget().size().height)
            .shrink(self.header_padding);

        let mut header_node = self.header.as_widget().layout(&mut tree.children[0], renderer, &limits_header);
        let mut header_size = header_node.size();
        header_node.align_mut(Alignment::Start, Alignment::Start, header_size);
        header_node.move_to_mut(Point::new(self.header_padding.left, self.header_padding.top));
        header_size = header_size.expand(self.header_padding);

        let limits_content = limits
            .loose()
            .width(self.width)
            .height(self.content.as_widget().size().height)
            .shrink(self.content_padding);

        let mut content_node = self.content.as_widget().layout(&mut tree.children[1], renderer, &limits_content);
        let mut content_size = content_node.size();
        content_node.align_mut(Alignment::Start, Alignment::Start, content_size);
        content_node.move_to_mut(Point::new(
            self.content_padding.left,
            header_size.height + self.content_padding.top
        ));
        content_size = content_size.expand(self.content_padding);

        let (footer_node, footer_size) = if let Some(footer) = &self.footer {
            let limits_footer = limits
                .loose()
                .width(self.width)
                .height(footer.as_widget().size().height)
                .shrink(self.footer_padding);

            let mut footer_node = footer.as_widget().layout(&mut tree.children[2], renderer, &limits_footer);
            let mut footer_size = footer_node.size();
            footer_node.align_mut(Alignment::Start, Alignment::Start, footer_size);
            footer_node.move_to_mut(Point::new(
                self.footer_padding.left,
                header_size.height + content_size.height + self.footer_padding.top
            ));
            footer_size = footer_size.expand(self.footer_padding);

            (Some(footer_node), footer_size)
        } else {
            (None, Size::ZERO)
        };

        if let Some(footer_node) = footer_node {
            Node::with_children(
                Size::new(
                    content_size.width,
                    header_size.height + content_size.height + footer_size.height
                ),
                vec![header_node, content_node, footer_node]
            )
        } else {
            Node::with_children(
                Size::new(
                    content_size.width,
                    header_size.height + content_size.height
                ),
                vec![header_node, content_node]
            )
        }
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
        let appearance = theme.active(&self.style);
        let bounds = layout.bounds();

        renderer.fill_quad(
            Quad {
                bounds,
                border: Border {
                    color: appearance.border_color,
                    width: 2.0,
                    radius: 10.0.into()
                },
                shadow: Default::default(),
            },
            appearance.background
        );

        let mut children = layout.children();

        let header_layout = children.next().expect("Card needs to have header.");
        renderer.fill_quad(
            Quad {
                bounds: header_layout.bounds().expand(self.header_padding.top),
                border: Border {
                    color: Default::default(),
                    width: 0.0,
                    radius: Radius::from([10.0, 10.0, 0.0, 0.0])
                },
                shadow: Default::default(),
            },
            appearance.header_background
        );
        self.header.as_widget().draw(
            &state.children[0],
            renderer,
            theme,
            style,
            header_layout,
            cursor,
            viewport
        );

        let content_layout = children.next().expect("Card needs to have content.");
        self.content.as_widget().draw(
            &state.children[1],
            renderer,
            theme,
            style,
            content_layout,
            cursor,
            viewport
        );

        if let Some(footer) = &self.footer {
            let footer_layout = children.next().expect("Card should have footer.");
            footer.as_widget().draw(
                &state.children[2],
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
        if let Some(footer) = &self.footer {
            vec![
                Tree::new(&self.header),
                Tree::new(&self.content),
                Tree::new(footer)
            ]
        } else {
            vec![
                Tree::new(&self.header),
                Tree::new(&self.content)
            ]
        }
    }

    fn diff(&self, tree: &mut Tree) {
        if let Some(footer) = &self.footer {
            tree.diff_children(&[
                &self.header, &self.content, footer
            ]);
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

        if let Some(footer) = &self.footer {
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
        
        let footer_interaction = if let Some(footer) = &self.footer {
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
        translation: Vector,
    ) -> Option<iced::advanced::overlay::Element<'b, Message, Theme, Renderer>> {
        let mut children = vec![&mut self.header, &mut self.content];
        if let Some(footer) = self.footer.as_mut() {
            children.push(footer);
        }
        
        let overlays :Vec<iced::advanced::overlay::Element<'b, Message, Theme, Renderer>>=
            layout.children().zip(state.children.iter_mut()).zip(children).filter_map(
                |((layout, state), element)| {
                    element.as_widget_mut().overlay(
                        state,
                        layout,
                        renderer,
                        translation
                    )
                }
            ).collect();

        (!overlays.is_empty()).then_some(iced::advanced::overlay::Group::with_children(overlays).overlay())
    }
}

impl<'a, Message, Theme, Renderer> From<Card<'a, Message, Theme, Renderer>> for Element<'a, Message, Theme, Renderer>
where
    Message: 'a + Clone,
    Renderer: 'a + iced::advanced::Renderer,
    Theme: 'a + StyleSheet
{
    fn from(value: Card<'a, Message, Theme, Renderer>) -> Self {
        Element::new(value)
    }
}

/// The appearance of the [Card].
pub struct Appearance
{
    /// The background of the content.
    pub background: Background,

    /// The background of the header.
    pub header_background: Background,

    /// The color of the border.
    pub border_color: Color
}

pub trait StyleSheet
{
    type Style: Default;

    fn active(&self, style: &Self::Style) -> Appearance;
}