use std::fs;
use iced::advanced::layout::{Limits, Node};
use iced::advanced::renderer::{Quad, Style};
use iced::advanced::{Clipboard, Layout, Shell, Widget};
use iced::advanced::widget::Tree;
use iced::{Background, Color, Element, Event, Length, mouse, Rectangle, Size};
use iced::event::Status;
use iced::mouse::{Cursor, Interaction};
use iced::widget::image::Handle;

/// The default size of a close button.
const DEFAULT_SIZE :f32= 40.0;

/// A [Widget] for a close button. Will be displayed using an [image](Handle). It can be resized.
pub struct Close<'a, Message, Renderer>
where
    Message: Clone,
    Renderer: iced::advanced::Renderer+iced::advanced::image::Renderer<Handle = Handle>
{
    /// The size of the button.
    size: f32,
    /// The [Message] which will be triggered when the button is pressed.
    on_trigger: Message,
    /// The [Handle] which stores the X image. Necessary for resizing the image.
    handle: Handle,
    /// The [Element] which stores the [Handle].
    image: Element<'a, Message, Renderer>
}

impl<'a, Message, Renderer> Close<'a, Message, Renderer>
where
    Message: Clone,
    Renderer: iced::advanced::Renderer+iced::advanced::image::Renderer<Handle = Handle>
{
    /// Creates a new [Close] instance with the given trigger [Message].
    pub fn new(on_trigger: impl Into<Message>) -> Self
    {
        let image = fs::read("src/images/close.png").unwrap();
        let handle = Handle::from_memory(image);

        Close {
            size: DEFAULT_SIZE,
            on_trigger: on_trigger.into(),
            handle: handle.clone(),
            image: iced::widget::image::Image::new(
                handle.clone()
            )
                .width(DEFAULT_SIZE)
                .height(DEFAULT_SIZE)
                .into()
        }
    }

    /// Updates the size of the [close button](Close) and resizes the [Handle].
    pub fn size(mut self, size: impl Into<f32>) -> Self
    {
        self.size = size.into();
        self.image = iced::widget::image::Image::new(
            self.handle.clone()
        )
            .width(self.size)
            .height(self.size)
            .into();

        self
    }
}

impl<'a, Message, Renderer> Widget<Message, Renderer> for Close<'a, Message, Renderer>
where
    Message: Clone,
    Renderer: iced::advanced::Renderer+iced::advanced::image::Renderer<Handle = Handle>
{
    fn width(&self) -> Length {
        Length::Fixed(self.size)
    }

    fn height(&self) -> Length {
        Length::Fixed(self.size)
    }

    fn layout(&self, renderer: &Renderer, limits: &Limits) -> Node {
        let limits = limits.loose().width(self.size).height(self.size);
        let image_layout = self.image.as_widget().layout(renderer, &limits);

        Node::with_children(
            Size::new(self.size, self.size),
            vec![image_layout]
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

        let background = Background::Color(
            if cursor.is_over(bounds) {
                Color::from_rgba(0.5, 0.5, 0.5, 0.5)
            } else {
                Color::TRANSPARENT
            }
        );

        let mut children = layout.children();

        self.image.as_widget().draw(
            &state.children[0],
            renderer,
            theme,
            style,
            children.next().expect("Close button needs to have image."),
            cursor,
            viewport
        );

        renderer.fill_quad(
            Quad {
                bounds,
                border_radius: 45.0.into(),
                border_width: 2.0,
                border_color: Color::from_rgb(0.5, 0.5, 0.5),
            },
            background
        );
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.image)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&[&self.image])
    }

    fn on_event(
        &mut self,
        _state: &mut Tree,
        event: Event,
        layout: Layout<'_>,
        cursor: Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle
    ) -> Status {
        let bounds = layout.bounds();

        if cursor.is_over(bounds) {
            match event {
                Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                    shell.publish(self.on_trigger.clone());
                    Status::Captured
                }
                _ => Status::Ignored
            }
        } else {
            Status::Ignored
        }
    }

    fn mouse_interaction(
        &self, _state: &Tree,
        layout: Layout<'_>,
        cursor: Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer
    ) -> Interaction {
        let bounds = layout.bounds();
        
        if cursor.is_over(bounds) {
            Interaction::Pointer
        } else {
            Interaction::default()
        }
    }
}

impl<'a, Message, Renderer> From<Close<'a, Message, Renderer>> for Element<'a, Message, Renderer>
where
    Message: 'a+Clone,
    Renderer: 'a+iced::advanced::Renderer+iced::advanced::image::Renderer<Handle = Handle>
{
    fn from(value: Close<'a, Message, Renderer>) -> Self {
        Element::new(value)
    }
}