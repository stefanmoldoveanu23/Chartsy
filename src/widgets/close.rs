use crate::utils::icons::{Icon, ICON};
use crate::utils::theme;
use iced::advanced::layout::{Limits, Node};
use iced::advanced::renderer::Style;
use iced::advanced::widget::Tree;
use iced::advanced::{Clipboard, Layout, Shell, Text, Widget};
use iced::alignment::{Horizontal, Vertical};
use iced::event::Status;
use iced::mouse::{Cursor, Interaction};
use iced::{mouse, Element, Event, Length, Point, Rectangle, Size};

/// The default size of a close button.
const DEFAULT_SIZE: f32 = 40.0;

/// A [Widget] for a close button. Will be displayed using an [image](Handle). It can be resized.
pub struct Close<Message>
where
    Message: Clone,
{
    /// The size of the button.
    size: f32,

    /// The [Message] which will be triggered when the button is pressed.
    on_trigger: Message,
}

impl<Message> Close<Message>
where
    Message: Clone,
{
    /// Creates a new [Close] instance with the given trigger [Message].
    pub fn new(on_trigger: impl Into<Message>) -> Self {
        Close {
            size: DEFAULT_SIZE,
            on_trigger: on_trigger.into(),
        }
    }

    /// Updates the size of the [close button](Close) and resizes the [Handle].
    pub fn size(mut self, size: impl Into<f32>) -> Self {
        self.size = size.into();
        self
    }
}

impl<'a, Message, Theme, Renderer> Widget<Message, Theme, Renderer> for Close<Message>
where
    Message: 'a + Clone,
    Renderer:
        'a + iced::advanced::renderer::Renderer + iced::advanced::text::Renderer<Font = iced::Font>,
{
    fn size(&self) -> Size<Length> {
        Size::new(Length::Fixed(self.size), Length::Fixed(self.size))
    }

    fn layout(&self, _tree: &mut Tree, _renderer: &Renderer, _limits: &Limits) -> Node {
        Node::new(Size::new(self.size, self.size))
    }

    fn draw(
        &self,
        _state: &Tree,
        renderer: &mut Renderer,
        _theme: &Theme,
        _style: &Style,
        layout: Layout<'_>,
        _cursor: Cursor,
        viewport: &Rectangle,
    ) {
        renderer.fill_text(
            Text {
                content: Icon::X.to_string(),
                bounds: layout.bounds().size(),
                size: self.size.into(),
                line_height: Default::default(),
                font: ICON,
                horizontal_alignment: Horizontal::Left,
                vertical_alignment: Vertical::Top,
                shaping: Default::default(),
            },
            Point::new(layout.bounds().x, layout.bounds().y - 1.0),
            theme::PALETTE.danger,
            *viewport,
        );
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
        _viewport: &Rectangle,
    ) -> Status {
        let bounds = layout.bounds();

        if cursor.is_over(bounds) {
            match event {
                Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                    shell.publish(self.on_trigger.clone());
                    Status::Captured
                }
                _ => Status::Ignored,
            }
        } else {
            Status::Ignored
        }
    }

    fn mouse_interaction(
        &self,
        _state: &Tree,
        layout: Layout<'_>,
        cursor: Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> Interaction {
        let bounds = layout.bounds();

        if cursor.is_over(bounds) {
            Interaction::Pointer
        } else {
            Interaction::default()
        }
    }
}

impl<'a, Message, Theme, Renderer> From<Close<Message>> for Element<'a, Message, Theme, Renderer>
where
    Message: 'a + Clone,
    Theme: 'a,
    Renderer: 'a + iced::advanced::Renderer + iced::advanced::text::Renderer<Font = iced::Font>,
{
    fn from(value: Close<Message>) -> Self {
        Element::new(value)
    }
}
