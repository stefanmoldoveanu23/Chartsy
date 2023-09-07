use iced::advanced::layout::{self, Layout};
use iced::advanced::{Clipboard, renderer, Shell, text, Text};
use iced::advanced::widget::{Tree, Widget};
use iced::{Element, Event, mouse};
use iced::{Color, Length, Rectangle, Size};
use iced::advanced::renderer::Quad;
use iced::alignment::{Horizontal, Vertical};
use iced::event::Status;
use iced::mouse::Cursor;

pub struct Menu<T: Clone, Message> {
    width: u32,
    options: Box<[(String, Box<[(String, T)]>)]>,
    action: Box<dyn Fn(T) -> Message>,
    hovered_option: Option<(usize, usize)>,
    is_pressing: bool,
}

impl<T: Clone, Message> Menu<T, Message> {
    fn new(width: u32, options: Box<[(String, Box<[(String, T)]>)]>, action: Box<dyn Fn(T) -> Message>) -> Menu<T, Message> {
        Menu {width, options, action, hovered_option: None, is_pressing: false}
    }
}

pub fn menu<T: Clone, Message>(width: u32, options: Box<[(String, Box<[(String, T)]>)]>, action: Box<dyn Fn(T) -> Message>) -> Menu<T, Message> {
    Menu::new(width, options, action)
}

impl<T: Clone, Message, Renderer> Widget<Message, Renderer> for Menu<T, Message>
where Renderer: text::Renderer
{
    fn width(&self) -> Length {
        Length::Fill
    }

    fn height(&self) -> Length {
        Length::Fill
    }

    fn layout(
        &self,
        _renderer: &Renderer,
        limits: &layout::Limits
    ) -> layout::Node {
        layout::Node::new(Size::new((&self).width as f32, limits.max().height))
    }

    fn draw(
        &self,
        _state: &Tree,
        renderer: &mut Renderer,
        _theme: &Renderer::Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: Cursor,
        _viewport: &Rectangle,
    ) {
        renderer.fill_quad(
            Quad {
                bounds: layout.bounds(),
                border_radius: 0.0.into(),
                border_width: 0.0,
                border_color: Color::TRANSPARENT,
            },
            Color::from_rgb8(8, 0, 64),
        );

        let mut count_options = 0;

        for (i, (name, values)) in self.options.iter().enumerate() {
            let bounds = layout.bounds();

            renderer.fill_text(Text {
                content: name,
                bounds: Rectangle {
                    x: bounds.center_x(),
                    y: 10.0 + (30.0 * (2.0 * i as f32)) + (30.0 * count_options as f32),
                    width: bounds.width,
                    height: bounds.height,
                },
                size: 30.0,
                line_height: text::LineHeight::Relative(1.0),
                color: Color::WHITE,
                font: Renderer::default_font(renderer),
                horizontal_alignment: Horizontal::Center,
                vertical_alignment: Vertical::Top,
                shaping: Default::default(),
            });

            for (j, (name, _value)) in values.iter().enumerate() {
                let button_color = if Some((i.clone(), j)) == self.hovered_option {
                    if self.is_pressing {
                        Color::from_rgb8(0, 64, 64)
                    } else {
                        Color::from_rgb8(0, 196, 196)
                    }
                } else {
                    Color::from_rgb8(0, 128, 128)
                };

                renderer.fill_quad(Quad {
                    bounds: Rectangle {
                        x: 76.0,
                        y: 8.0 + (30.0 * (2.0 * i.clone() as f32 + 1.0)) + (30.0 * (count_options.clone() + j.clone()) as f32),
                        width: self.width as f32 - 90.0,
                        height: 25.0,
                    },
                    border_radius: 5.0.into(),
                    border_width: 0.0,
                    border_color: Color::TRANSPARENT,
                },
                    button_color
                );

                renderer.fill_text(Text{
                    content: name,
                    bounds: Rectangle {
                        x: 80.0,
                        y: 10.0 + (30.0 * (2.0 * i.clone() as f32 + 1.0)) + (30.0 * (count_options.clone() + j.clone()) as f32),
                        width: bounds.clone().width,
                        height: bounds.clone().height,
                    },
                    size: 20.0,
                    line_height: text::LineHeight::Relative(1.0),
                    color: Color::WHITE,
                    font: Renderer::default_font(renderer),
                    horizontal_alignment: Horizontal::Left,
                    vertical_alignment: Vertical::Top,
                    shaping: Default::default(),
                });
            }

            count_options = count_options.clone() + values.len();
        }
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
        match event {
            Event::Mouse(mouse_event) => {
                match mouse_event {
                    mouse::Event::ButtonReleased(mouse::Button::Left) => {
                        if cursor.is_over(layout.bounds()) {
                            if let Some(pos) = self.hovered_option {
                                shell.publish((self.action)(((*self).options[pos.0].1[pos.1].1).clone()));
                                Status::Captured
                            } else {
                                Status::Ignored
                            }
                        } else {
                            Status::Ignored
                        }
                    }
                    mouse::Event::ButtonPressed(mouse::Button::Left) => {
                        if cursor.is_over(layout.bounds()) {
                            self.is_pressing = true;
                            Status::Captured
                        } else {
                            Status::Ignored
                        }
                    }
                    mouse::Event::CursorMoved{ .. } => {
                        if let Some(cursor_position) = cursor.position_in(layout.bounds()) {
                            let mut count_options = 0;
                            let mut hovered = false;

                            if 76.0 > cursor_position.x || cursor_position.x > (self.width) as f32 - 14.0 {
                                self.hovered_option = None;
                                Status::Ignored
                            } else {
                                for (i, (_name, values)) in self.options.iter().enumerate() {
                                    if cursor_position.y < 10.0 + (30.0 * (2.0 * i as f32 + 1.0)) + (30.0 * (count_options.clone() + values.len()) as f32) {
                                        for j in 0..values.len() {
                                            if 8.0 + (30.0 * (2.0 * i.clone() as f32 + 1.0)) + (30.0 * (count_options.clone() + j) as f32) <= cursor_position.y &&
                                                cursor_position.y <= 33.0 + (30.0 * (2.0 * i.clone() as f32 + 1.0)) + (30.0 * (count_options.clone() + j.clone()) as f32) {
                                                self.hovered_option = Some((i.clone(), j.clone()));
                                                hovered = true;
                                                break
                                            }
                                        }

                                        break
                                    }

                                    count_options = count_options.clone() + values.len();
                                }

                                if hovered == false {
                                    self.hovered_option = None;
                                }

                                Status::Captured
                            }
                        } else {
                            self.hovered_option = None;
                            Status::Ignored
                        }
                    }
                    _ => Status::Ignored
                }
            }

            _ => Status::Ignored
        }
    }


}

impl<'a, T: 'a+Clone, Message, Renderer> From<Menu<T, Message>> for Element<'a, Message, Renderer>
where Renderer: text::Renderer,
    Message: 'a
{
    fn from(menu: Menu<T, Message>) -> Self {
        Self::new(menu)
    }
}