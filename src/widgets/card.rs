use iced::{Alignment, Border, Color, Element, Event, Length, mouse, Padding, Point, Rectangle, Size};
use iced::advanced::layout::{Limits, Node};
use iced::advanced::renderer::{Quad, Style};
use iced::advanced::{Clipboard, Layout, Shell, Widget};
use iced::advanced::widget::{Operation, Tree};
use iced::border::Radius;
use iced::event::Status;
use iced::mouse::{Cursor, Interaction};

const DEFAULT_PADDING :f32= 10.0;
const DEFAULT_CLOSE :f32= 15.0;
const BORDER_RADIUS :f32= 0.0;
const CLOSE_RADIUS :f32= 1.0;

pub struct Card<'a, Message, Theme, Renderer>
where
    Message: Clone,
    Renderer: iced::advanced::Renderer
{
    width: Length,
    height: Length,
    border_color: Color,
    background_color: Color,
    padding_header: f32,
    header: Element<'a, Message, Theme, Renderer>,
    padding_content: f32,
    content: Element<'a, Message, Theme, Renderer>,
    padding_footer: f32,
    footer: Option<Element<'a, Message, Theme, Renderer>>,
    on_close: Option<Message>,
    close_size: f32,
}

impl<'a, Message, Theme, Renderer> Card<'a, Message, Theme, Renderer>
where
    Message: Clone,
    Renderer: iced::advanced::Renderer
{
    pub fn new(header: impl Into<Element<'a, Message, Theme, Renderer>>, content: impl Into<Element<'a, Message, Theme, Renderer>>) -> Self
    {
        Card {
            width: Length::Shrink,
            height: Length::Shrink,
            border_color: Color { r: 0.5f32, g: 0.5f32, b: 0.5f32, a: 1.0f32 },
            background_color: Color::WHITE,
            padding_header: DEFAULT_PADDING,
            header: header.into(),
            padding_content: DEFAULT_PADDING,
            content: content.into(),
            padding_footer: DEFAULT_PADDING,
            footer: None,
            on_close: None,
            close_size: DEFAULT_CLOSE,
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

    pub fn border_color(mut self, border_color: impl Into<Color>) -> Self
    {
        self.border_color = border_color.into();

        self
    }

    pub fn background_color(mut self, background_color: impl Into<Color>) -> Self
    {
        self.background_color = background_color.into();

        self
    }

    pub fn padding_header(mut self, padding: impl Into<f32>) -> Self
    {
        self.padding_header = padding.into();

        self
    }

    pub fn header(mut self, header: impl Into<Element<'a, Message, Theme, Renderer>>) -> Self
    {
        self.header = header.into();

        self
    }

    pub fn padding_content(mut self, padding: impl Into<f32>) -> Self
    {
        self.padding_content = padding.into();

        self
    }

    pub fn content(mut self, content: impl Into<Element<'a, Message, Theme, Renderer>>) -> Self
    {
        self.content = content.into();

        self
    }

    pub fn padding_footer(mut self, padding: impl Into<f32>) -> Self
    {
        self.padding_footer = padding.into();

        self
    }

    pub fn footer(mut self, footer: impl Into<Element<'a, Message, Theme, Renderer>>) -> Self
    {
        self.footer = Some(footer.into());

        self
    }

    pub fn on_close(mut self, on_close: impl Into<Message>) -> Self
    {
        self.on_close = Some(on_close.into());

        self
    }

    pub fn close_size(mut self, close_size: impl Into<f32>) -> Self
    {
        self.close_size = close_size.into();

        self
    }
}

impl<'a, Message, Theme, Renderer> Widget<Message, Theme, Renderer> for Card<'a, Message, Theme, Renderer>
where
    Message: Clone,
    Renderer: iced::advanced::Renderer,
{
    fn size(&self) -> Size<Length> {
        Size::new(
            self.width,
            self.height,
        )
    }

    fn layout(&self, tree: &mut Tree, renderer: &Renderer, limits: &Limits) -> Node {
        let header_layout = get_header_layout(
            tree,
            renderer,
            limits,
            &self.header,
            self.padding_header,
            self.width,
            self.on_close.clone().map_or_else(|| None, |_| Some(self.close_size))
        );

        let mut content_layout = get_content_layout(
            tree,
            renderer,
            limits,
            &self.content,
            self.padding_content,
            self.width
        );

        content_layout.move_to_mut(Point::new(0.0, content_layout.clone().bounds().y + header_layout.clone().bounds().height));

        let mut footer_layout = self.footer.as_ref().map_or_else(|| Node::default(), |footer| get_footer_layout(
            tree,
            renderer,
            limits,
            footer,
            self.padding_footer,
            self.width
        ));

        footer_layout.move_to_mut(Point::new(0.0, footer_layout.bounds().y + content_layout.bounds().height + header_layout.bounds().y));

        Node::with_children(
            Size::new(
                content_layout.bounds().width,
                header_layout.bounds().height + content_layout.bounds().height + footer_layout.bounds().height
            ),
            vec![header_layout, content_layout, footer_layout]
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
        let mut children = layout.children();

        renderer.fill_quad(
            Quad {
                bounds,
                border: Border {
                    color: self.border_color,
                    width: 1.0,
                    radius: Radius::from(BORDER_RADIUS),
                },
                shadow: Default::default(),
            },
            self.background_color
        );

        let header_layout = children
            .next()
            .expect("Error: Card should have a header.");
        draw_header(
            &tree.children[0],
            renderer,
            theme,
            style,
            header_layout,
            cursor,
            viewport,
            &self.header,
            self.background_color
        );

        let content_layout = children
            .next()
            .expect("Error: Card should have a content.");
        draw_content(
            &tree.children[1],
            renderer,
            theme,
            style,
            content_layout,
            cursor,
            viewport,
            &self.content,
        );

        let footer_layout = children
            .next()
            .expect("Error: Card should have a footer.");
        draw_footer(
            tree.children.get(2),
            renderer,
            theme,
            style,
            footer_layout,
            cursor,
            viewport,
            &self.footer
        );
    }

    fn children(&self) -> Vec<Tree> {
        self.footer.as_ref().map_or_else(
            || vec![Tree::new(&self.header), Tree::new(&self.content)],
            |footer| vec![Tree::new(&self.header), Tree::new(&self.content), Tree::new(footer)]
        )
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

        let header_layout = children.next().expect("Error: Card needs to have header.");
        let content_layout = children.next().expect("Error: Card needs to have content.");
        let footer_layout = children.next().expect("Error: Card needs to have footer.");

        self.header.as_widget().operate(
            &mut state.children[0],
            header_layout,
            renderer,
            operation
        );

        self.content.as_widget().operate(
            &mut state.children[1],
            content_layout,
            renderer,
            operation
        );

        if let Some(footer) = self.footer.as_ref() {
            footer.as_widget().operate(
                &mut state.children[2],
                footer_layout,
                renderer,
                operation
            )
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

        let header_layout = children
            .next()
            .expect("Error: Card needs to have header.");
        let mut header_children = header_layout.children();

        let header_status = &self.header.as_widget_mut().on_event(
            &mut state.children[0],
            event.clone(),
            header_children
                .next()
                .expect("Error: Card header needs to have element."),
            cursor,
            renderer,
            clipboard,
            shell,
            viewport
        );

        let close_status = header_children
            .next()
            .map_or_else(|| Status::Ignored, |close_layout| {
                match event {
                    Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                        self.on_close.clone().filter(
                            |_| close_layout.bounds().contains(cursor.position().unwrap())
                        ).map_or(Status::Ignored, |on_close| {
                            shell.publish(on_close);
                            Status::Captured
                        })
                    }
                    _ => Status::Ignored
                }
            });

        let content_layout = children
            .next()
            .expect("Error: Card needs to have content.");
        let mut content_children = content_layout.children();
        let content_status = self.content.as_widget_mut().on_event(
            &mut state.children[1],
            event.clone(),
            content_children
                .next()
                .expect("Error: Card content needs to have element"),
            cursor,
            renderer,
            clipboard,
            shell,
            viewport
        );

        let footer_layout = children
            .next()
            .expect("Error: Card needs to have footer.");
        let mut footer_children = footer_layout.children();
        let footer_status = self.footer.as_mut().map_or_else(|| Status::Ignored, |footer| {
            footer.as_widget_mut().on_event(
                &mut state.children[2],
                event,
                footer_children
                    .next()
                    .expect("Error: Footer needs to have element."),
                cursor,
                renderer,
                clipboard,
                shell,
                viewport
            )
        });

        header_status
            .merge(content_status)
            .merge(close_status)
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

        let header_layout = children
            .next()
            .expect("Error: Card needs to have header.");
        let mut header_children = header_layout.children();
        let header_layout = header_children
            .next()
            .expect("Error: Card header needs to have element.");

        let close_layout = header_children.next();
        let is_mouse_over_close = close_layout.map_or_else(
            || false,
            |close_layout| close_layout.bounds().contains(cursor.position().unwrap())
        );

        let close_interaction = if is_mouse_over_close {
            Interaction::Pointer
        } else {
            Interaction::default()
        };

        let content_layout = children
            .next()
            .expect("Error: Card needs to have content.");
        let mut content_children = content_layout.children();

        let footer_layout = children
            .next()
            .expect("Error: Card needs to have footer.");
        let mut footer_children = footer_layout.children();

        close_interaction
            .max(
                self.header.as_widget().mouse_interaction(
                    &state.children[0],
                    header_layout,
                    cursor,
                    viewport,
                    renderer
                )
            )
            .max(
                self.content.as_widget().mouse_interaction(
                    &state.children[1],
                    content_children
                        .next()
                        .expect("Error: Card content needs to have element."),
                    cursor,
                    viewport,
                    renderer
                )
            )
            .max(
                self.footer.as_ref().map_or_else(
                    || Interaction::default(),
                    |footer| {
                        footer.as_widget().mouse_interaction(
                            &state.children[2],
                            footer_children
                                .next()
                                .expect("Error: Card footer needs to have element."),
                            cursor,
                            viewport,
                            renderer
                        )
                    }
                )
            )

    }
}

fn get_header_layout<Message, Theme, Renderer>(
    tree: &mut Tree,
    renderer: &Renderer,
    limits: &Limits,
    header: &Element<'_, Message, Theme, Renderer>,
    padding: f32,
    width: Length,
    close_size: Option<f32>,
) -> Node
where
    Message: Clone,
    Renderer: iced::advanced::Renderer,
{
    let padding = Padding::from(padding);

    let mut limits = limits
        .loose()
        .width(width)
        .height(header.as_widget().size().height);

    let mut close_layout = close_size.map(|size| {
        limits = limits.shrink(Size::new(size, 0.0));
        Node::new(Size::new(size + 1.0, size + 1.0))
    });

    let mut header_layout = header.as_widget().layout(tree, renderer, &limits);
    let size = limits.resolve(header_layout.size().width, header_layout.size().height, header_layout.size());

    header_layout.move_to_mut(Point::new(padding.left, padding.top));
    header_layout.align_mut(Alignment::Start, Alignment::Center, header_layout.size());

    if let Some(ref mut close_layout) = close_layout {
        size.expand(Size::new(close_size.unwrap(), 0.0));

        close_layout.move_to_mut(Point::new(size.width - padding.right, padding.top));
        close_layout.align_mut(Alignment::End, Alignment::Center, close_layout.size());
    };

    Node::with_children(
        size.expand(Size::from(padding)),
        match close_layout {
            Some(ref mut close_layout) => vec![header_layout, close_layout.clone()],
            None => vec![header_layout]
        }
    )
}

fn draw_header<Message, Theme, Renderer>(
    tree: &Tree,
    renderer: &mut Renderer,
    theme: &Theme,
    style: &Style,
    layout: Layout<'_>,
    cursor: Cursor,
    viewport: &Rectangle,
    header: &Element<'_, Message, Theme, Renderer>,
    background_color: Color,
)
where
    Message: Clone,
    Renderer: iced::advanced::Renderer
{
    let mut header_children = layout.children();
    let header_bounds = layout.bounds();

    renderer.fill_quad(
        Quad {
            bounds: header_bounds,
            border: Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: Radius::from([BORDER_RADIUS, BORDER_RADIUS, 0.0, 0.0]),
            },
            shadow: Default::default(),
        },
        background_color
    );

    header.as_widget().draw(
        tree,
        renderer,
        theme,
        style,
        header_children
            .next()
            .expect("Error: Need to have base header element in card."),
        cursor,
        viewport
    );

    if let Some(close_layout) = header_children.next() {
        let close_bounds = close_layout.bounds();
        let is_mouse_over_close = close_bounds.contains(cursor.position().unwrap_or_default());

        renderer.fill_quad(
            Quad {
                bounds: Rectangle {
                    width: close_bounds.width + if is_mouse_over_close {1.0} else {0.0},
                    height: close_bounds.height + if is_mouse_over_close {1.0} else {0.0},
                    ..close_bounds
                },
                border: Border {
                    color: Color::TRANSPARENT,
                    width: 0.0,
                    radius: Radius::from(CLOSE_RADIUS),
                },
                shadow: Default::default(),
            },
            Color::from([1.0, 0.0, 0.0])
        );
    }
}

fn get_content_layout<Message, Theme, Renderer>(
    tree: &mut Tree,
    renderer: &Renderer,
    limits: &Limits,
    content: &Element<'_, Message, Theme, Renderer>,
    padding: f32,
    width: Length
) -> Node
where
    Message: Clone,
    Renderer: iced::advanced::Renderer
{
    let padding = Padding::from(padding);

    let limits = limits
        .loose()
        .width(width)
        .height(content.as_widget().size().height);

    let mut content_layout = content.as_widget().layout(tree, renderer, &limits);
    let size = limits.resolve(content_layout.size().width, content_layout.size().height, content_layout.size());

    content_layout.move_to_mut(Point::new(padding.left, padding.top));
    content_layout.align_mut(Alignment::Start, Alignment::Start, content_layout.size());

    Node::with_children(
        size.expand(Size::from(padding)),
        vec![content_layout]
    )
}

fn draw_content<Message, Theme, Renderer>(
    tree: &Tree,
    renderer: &mut Renderer,
    theme: &Theme,
    style: &Style,
    layout: Layout<'_>,
    cursor: Cursor,
    viewport: &Rectangle,
    content: &Element<'_, Message, Theme, Renderer>
)
where
    Message: Clone,
    Renderer: iced::advanced::Renderer
{
    let mut content_children = layout.children();

    content.as_widget().draw(
        tree,
        renderer,
        theme,
        style,
        content_children
            .next()
            .expect("Error: Need to have element in content."),
        cursor,
        viewport
    );
}

fn get_footer_layout<Message, Theme, Renderer>(
    tree: &mut Tree,
    renderer: &Renderer,
    limits: &Limits,
    footer: &Element<'_, Message, Theme, Renderer>,
    padding: f32,
    width: Length
) -> Node
where
    Message: Clone,
    Renderer: iced::advanced::Renderer,
{
    let padding = Padding::new(padding);

    let limits = limits
        .loose()
        .width(width)
        .height(footer.as_widget().size().height);

    let mut footer_layout = footer.as_widget().layout(tree, renderer, &limits);
    let size = limits.resolve(footer_layout.size().width, footer_layout.size().height, footer_layout.size());

    footer_layout.move_to_mut(Point::new(padding.left, padding.top));
    footer_layout.align_mut(Alignment::Start, Alignment::Center, footer_layout.size());

    Node::with_children(
        size.expand(Size::from(padding)),
        vec![footer_layout]
    )
}

fn draw_footer<Message, Theme, Renderer>(
    tree: Option<&Tree>,
    renderer: &mut Renderer,
    theme: &Theme,
    style: &Style,
    layout: Layout<'_>,
    cursor: Cursor,
    viewport: &Rectangle,
    footer: &Option<Element<'_, Message, Theme, Renderer>>
)
where
    Message: Clone,
    Renderer: iced::advanced::Renderer
{
    if let Some((footer, tree)) = footer.as_ref().zip(tree) {
        let mut footer_children = layout.children();

        footer.as_widget().draw(
            tree,
            renderer,
            theme,
            style,
            footer_children
                .next()
                .expect("Error: Need to have element in footer"),
            cursor,
            viewport
        );
    }
}

impl<'a, Message, Theme, Renderer> From<Card<'a, Message, Theme, Renderer>> for Element<'a, Message, Theme, Renderer>
where
    Message: Clone + 'a,
    Renderer: 'a + iced::advanced::Renderer,
    Theme: 'a
{
    fn from(value: Card<'a, Message, Theme, Renderer>) -> Self {
        Element::new(value)
    }
}