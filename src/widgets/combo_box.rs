use std::fmt::Display;
use iced::advanced::layout::{Limits, Node};
use iced::advanced::renderer::{Quad, Style};
use iced::advanced::{Clipboard, Layout, Overlay, Shell, Widget};
use iced::advanced::widget::{Operation, Tree};
use iced::{Border, Color, Event, Length, mouse, Point, Rectangle, Size, Vector};
use iced::advanced::overlay::Element;
use iced::event::Status;
use iced::mouse::{Cursor, Interaction};
use iced::widget::{Column, Text, TextInput};
use difflib::sequencematcher::SequenceMatcher;

/// A widget where the user can input text and is offered choices from a given list of options
///that are similar to that input.
pub struct ComboBox<'a, Tag, Message, Theme, Renderer>
where
    Tag: 'a + Clone + Display,
    Message: 'a + Clone,
    Renderer: 'a + iced::advanced::Renderer + iced::advanced::text::Renderer,
    Theme: 'a + iced::widget::text_input::StyleSheet + iced::widget::text::StyleSheet
{
    /// The list of options the user can choose from.
    tags: Vec<Tag>,

    /// The text input object where the user will write.
    text_input: TextInput<'a, Message, Theme, Renderer>,

    /// The column of options that is displayed on the screen.
    column: Column<'a, Message, Theme, Renderer>,

    /// The message that will be triggered when the user selects an option.
    on_selected: fn(Tag) -> Message,

    /// The index of the tab that is hovered on.
    tag_hovered: Option<usize>,
}

impl<'a, Tag, Message, Theme, Renderer> ComboBox<'a, Tag, Message, Theme, Renderer>
where
    Tag: 'a + Clone + Display,
    Message: 'a + Clone,
    Renderer: 'a + iced::advanced::Renderer + iced::advanced::text::Renderer,
    Theme: 'a + iced::widget::text_input::StyleSheet + iced::widget::text::StyleSheet
{
    /// Creates a new combo box.
    pub fn new(tags: Vec<Tag>, placeholder: &'a str, value: &'a str, on_selected: fn(Tag) -> Message) -> Self
    {
        let filtered_tags :Vec<Tag>= filter_tags(tags, value, 10);

        ComboBox {
            tags: filtered_tags.clone(),
            text_input: TextInput::new(placeholder, value),
            column: Column::with_children(
                filtered_tags.iter().map(|tag| Text::new(tag.to_string()).into())
                    .collect::<Vec<iced::Element<'a, Message, Theme, Renderer>>>()
            )
                .padding(1.0),
            on_selected,
            tag_hovered: None,
        }
    }

    /// Sets the event for when the user writes.
    pub fn on_input(mut self, on_input: fn(String) -> Message) -> Self
    {
        self.text_input = self.text_input.on_input(on_input);

        self
    }

    /// Sets the width of the [ComboBox].
    pub fn width(mut self, width: impl Into<Length>) -> Self
    {
        self.text_input = self.text_input.width(width);

        self
    }
}

impl<'a, Tag, Message, Theme, Renderer> Widget<Message, Theme, Renderer> for ComboBox<'a, Tag, Message, Theme, Renderer>
where
    Tag: 'a + Clone + Display,
    Message: 'a + Clone,
    Renderer: 'a + iced::advanced::Renderer + iced::advanced::text::Renderer,
    Theme: 'a + iced::widget::text_input::StyleSheet + iced::widget::text::StyleSheet
{
    fn size(&self) -> Size<Length> {
        (&self.text_input as &dyn Widget<Message, Theme, Renderer>).size()
    }

    fn layout(&self, tree: &mut Tree, renderer: &Renderer, limits: &Limits) -> Node {
        self.text_input.layout(&mut tree.children[0], renderer, limits, None)
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
        (&self.text_input as &dyn Widget<Message, Theme, Renderer>).draw(
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
        vec![
            Tree::new(&self.text_input as &dyn Widget<Message, Theme, Renderer>),
            Tree::new(&self.column as &dyn Widget<Message, Theme, Renderer>)
        ]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&[
            &self.text_input as &dyn Widget<Message, Theme, Renderer>,
            &self.column as &dyn Widget<Message, Theme, Renderer>
        ]);
    }

    fn operate(
        &self,
        state: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation<Message>
    ) {
        self.text_input.operate(
            &mut state.children[0],
            layout,
            renderer,
            operation
        );
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
        self.text_input.on_event(
            &mut state.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport
        )
    }

    fn mouse_interaction(
        &self,
        state: &Tree,
        layout: Layout<'_>,
        cursor: Cursor,
        viewport: &Rectangle,
        renderer: &Renderer
    ) -> Interaction {
        self.text_input.mouse_interaction(
            &state.children[0],
            layout,
            cursor,
            viewport,
            renderer
        )
    }

    fn overlay<'b>(
        &'b mut self,
        state: &'b mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        translation: Vector,
    ) -> Option<Element<'b, Message, Theme, Renderer>> {
        let mut children = state.children.iter_mut();
        let text_input_overlay = self.text_input.overlay(
            children.next().expect("Need to have text input child."),
            layout,
            renderer,
            translation
        );

        if self.tags.len() > 0 {
            let bounds = layout.bounds();
            let column = Column::<Message, Theme, Renderer>::with_children(
                self.tags.iter().map(|tag| Text::new(tag.to_string()).into())
                    .collect::<Vec<iced::Element<'a, Message, Theme, Renderer>>>()
            )
                .width(Length::Fixed(bounds.width));
            self.column = column;
            
            let state = children.next().expect("Need to have menu child.");
            self.column.diff(state);
            
            let menu_overlay = Element::new(
                Box::new(Menu::new(
                    state,
                    &mut self.column,
                    &mut self.tags,
                    self.on_selected,
                    Vector::new(bounds.x + translation.x, bounds.y + translation.y + bounds.height),
                    &mut self.tag_hovered
                ))
            );
            
            if let Some(element) = text_input_overlay {
                Some(iced::advanced::overlay::Group::with_children(vec![
                    element,
                    menu_overlay
                ]).overlay())
            } else {
                Some(iced::advanced::overlay::Group::with_children(vec![
                    menu_overlay
                ]).overlay())
            }
        } else {
            text_input_overlay
        }
    }
}

impl<'a, Tag, Message, Theme, Renderer> From<ComboBox<'a, Tag, Message, Theme, Renderer>> for iced::Element<'a, Message, Theme, Renderer>
where
    Tag: 'a + Clone + Display,
    Message: 'a + Clone,
    Renderer: 'a + iced::advanced::Renderer + iced::advanced::text::Renderer,
    Theme: 'a + iced::widget::text_input::StyleSheet + iced::widget::text::StyleSheet
{
    fn from(value: ComboBox<'a, Tag, Message, Theme, Renderer>) -> Self {
        iced::Element::new(value)
    }
}

/// An overlay for the [ComboBox] that displays the options.
struct Menu<'a, 'b, Tag, Message, Theme, Renderer>
where
    Tag: 'a + Clone + Display,
    Message: 'a + Clone,
    Renderer: 'a + iced::advanced::Renderer,
{
    /// The tree object of the column.
    state: &'b mut Tree,

    /// The column object.
    column: &'b mut Column<'a, Message, Theme, Renderer>,

    /// The list of options.
    tags: &'b mut Vec<Tag>,

    /// The message that will be triggered when the user selects an option.
    on_select: fn(Tag) -> Message,

    /// The translation of the overlay.
    translation: Vector,

    /// The tag which the user is hovering on.
    tag_hovered: &'b mut Option<usize>,
}

impl<'a, 'b, Tag, Message, Theme, Renderer> Menu<'a, 'b, Tag, Message, Theme, Renderer>
where
    Tag: 'a + Clone + Display,
    Message: 'a + Clone,
    Renderer: 'a + iced::advanced::Renderer
{
    /// Creates a new [Menu].
    fn new(
        state: &'b mut Tree,
        column: &'b mut Column<'a, Message, Theme, Renderer>,
        tags: &'b mut Vec<Tag>,
        on_select: fn(Tag) -> Message,
        translation: Vector,
        tag_hovered: &'b mut Option<usize>,
    ) -> Self {
        Menu {
            state,
            column,
            tags,
            on_select,
            translation,
            tag_hovered,
        }
    }
}

impl<'a, 'b, Tag, Message, Theme, Renderer> Overlay<Message, Theme, Renderer> for Menu<'a, 'b, Tag, Message, Theme, Renderer>
where
    Tag: 'a + Clone + Display,
    Message: 'a + Clone,
    Renderer: 'a + iced::advanced::Renderer
{
    fn layout(
        &mut self,
        renderer: &Renderer,
        bounds: Size,
    ) -> Node {
        let limits = Limits::new(Size::ZERO, bounds);

        let mut node = self.column.layout(self.state, renderer, &limits);
        node.move_to_mut(Point::new(self.translation.x, self.translation.y));

        node
    }

    fn draw(
        &self,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &Style,
        layout: Layout<'_>,
        cursor: Cursor
    ) {
        let bounds = layout.bounds();
        let width = bounds.width;

        let children = layout.children();
        let mut index :usize= 0;
        for node in children {
            let mut bounds = node.bounds();
            bounds.width = width;

            let color = if let Some(pos) = self.tag_hovered.as_ref() {
                if *pos == index {
                    Color::from_rgb(0.7, 0.7, 0.7)
                } else {
                    Color::WHITE
                }
            } else {
                Color::WHITE
            };

            renderer.fill_quad(
                Quad {
                    bounds,
                    border: Default::default(),
                    shadow: Default::default(),
                },
                color
            );

            index += 1;
        }

        renderer.fill_quad(
            Quad {
                bounds,
                border: Border {
                    color: Color::BLACK,
                    width: 1.0,
                    radius: Default::default()
                },
                shadow: Default::default(),
            },
            Color::TRANSPARENT
        );

        self.column.draw(
            self.state,
            renderer,
            theme,
            style,
            layout,
            cursor,
            &layout.bounds()
        );
    }

    fn operate(
        &mut self,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation<Message>
    ) {
        self.column.operate(
            self.state,
            layout,
            renderer,
            operation
        );
    }

    fn on_event(
        &mut self,
        event: Event,
        layout: Layout<'_>,
        cursor: Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>
    ) -> Status {
        let width = layout.bounds().width;

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let mut index :usize= 0;
                let children = layout.children();

                for node in children {
                    let mut bounds = node.bounds();
                    bounds.width = width;

                    if cursor.is_over(bounds) {
                        shell.publish((self.on_select)(self.tags[index].clone()));
                        return Status::Captured;
                    }

                    index = index + 1;
                }

                Status::Ignored
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                let mut index :usize= 0;
                let children = layout.children();

                for node in children {
                    let mut bounds = node.bounds();
                    bounds.width = width;

                    if cursor.is_over(bounds) {
                        if let Some(tag_hovered) = self.tag_hovered {
                            if *tag_hovered != index {
                                *self.tag_hovered = Some(index);
                                return Status::Captured;
                            } else {
                                return Status::Ignored;
                            }
                        } else {
                            *self.tag_hovered = Some(index);
                            return Status::Captured;
                        }
                    }

                    index = index + 1;
                }

                *self.tag_hovered = None;
                Status::Ignored
            }
            _ => Status::Ignored
        }
    }

    fn mouse_interaction(
        &self,
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

/// Filters the given list of tags and returns only the ones similar to the user input.
fn filter_tags<Tag>(tags: Vec<Tag>, user_input: &str, count: usize) -> Vec<Tag>
where
    Tag: Clone + Display
{
    let mut scores :Vec<f64>= vec![];
    let mut filtered :Vec<usize>= vec![];
    let user_input = user_input.to_lowercase();
    let user_words = user_input.split(" ");

    for (tag, i) in tags.iter().zip(0..tags.len()) {
        let mut i = i;
        let tag_name = tag.to_string().to_lowercase();
        let tag_words = tag_name.split(" ");
        let mut total_score :f64= 0.0;

        for (user_word, tag_word) in user_words.clone().zip(tag_words) {
            let mut matcher = SequenceMatcher::new(user_word, tag_word);
            let score = (matcher.ratio() * 100.0) as f64;
            total_score += score;
        }

        if total_score > 30.0 {
            if filtered.len() == count && *(scores.last().unwrap()) > total_score {
                continue;
            }

            for j in 0..filtered.len() {
                if scores[j] < total_score {
                    (i, filtered[j]) = (filtered[j], i);
                    (scores[j], total_score) = (total_score, scores[j]);
                }
            }

            if filtered.len() < count {
                filtered.push(i);
                scores.push(total_score);
            }
        }
    }
    
    filtered.iter().map(|pos| tags[*pos].clone()).collect()
}