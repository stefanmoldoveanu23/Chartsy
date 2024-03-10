use iced::{Alignment, Background, Border, Element, Event, Length, mouse, Padding, Point, Rectangle, Size, Vector};
use iced::advanced::layout::{Limits, Node};
use iced::advanced::renderer::{Quad, Style};
use iced::advanced::{Clipboard, Layout, Shell, Widget};
use iced::advanced::widget::{Operation, Tree};
use iced::border::Radius;
use iced::event::Status;
use iced::mouse::{Cursor, Interaction};
use iced::widget::{Container, Row};

const DEFAULT_BAR_PADDING :f32= 5.0;
const DEFAULT_CONTENT_PADDING :f32= 10.0;

pub struct Tab<'a, TabId, Message, Theme, Renderer>
where
    TabId: 'a + Eq + Default,
    Message: 'a + Clone,
    Theme: 'a + StyleSheet,
    Renderer: 'a + iced::advanced::Renderer
{
    tab_id: TabId,
    tab_label: Element<'a, Message, Theme, Renderer>,
    tab_content: Element<'a, Message, Theme, Renderer>
}

impl<'a, TabId, Message, Theme, Renderer> Tab<'a, TabId, Message, Theme, Renderer>
where
    TabId: 'a + Eq + Default,
    Message: 'a + Clone,
    Theme: 'a + StyleSheet,
    Renderer: 'a + iced::advanced::Renderer
{
    pub fn new(
        tab_id: impl Into<TabId>,
        tab_label: impl Into<Element<'a, Message, Theme, Renderer>>,
        tab_content: impl Into<Element<'a, Message, Theme, Renderer>>
    ) -> Self {
        Tab {
            tab_id: tab_id.into(),
            tab_label: tab_label.into(),
            tab_content: tab_content.into()
        }
    }
}

pub struct Tabs<'a, TabId, Message, Theme, Renderer>
where
    TabId: 'a + Eq + Default,
    Message: 'a + Clone,
    Theme: 'a + StyleSheet + iced::widget::container::StyleSheet,
    Renderer: 'a + iced::advanced::Renderer
{
    width: Length,
    height: Length,
    tab_bar_padding: Padding,
    tab_content_padding: Padding,
    tab_bar: Row<'a, Message, Theme, Renderer>,
    tabs: Vec<(TabId, Element<'a, Message, Theme, Renderer>)>,
    active_tab: TabId,
    on_change_tab: fn(&TabId) -> Message,
    style: <Theme as StyleSheet>::Style,
}

impl<'a, TabId, Message, Theme, Renderer> Tabs<'a, TabId, Message, Theme, Renderer>
where
    TabId: 'a + Eq + Default,
    Message: 'a + Clone,
    Theme: 'a + StyleSheet + iced::widget::container::StyleSheet,
    Renderer: 'a + iced::advanced::Renderer
{
    pub fn new(tabs: Vec<Tab<'a, TabId, Message, Theme, Renderer>>, on_change_tab: fn(&TabId) -> Message) -> Self
    {
        let mut tabs_vec = vec![];
        let mut row = Row::new()
            .width(Length::Fill)
            .height(Length::Shrink)
            .padding(DEFAULT_BAR_PADDING)
            .spacing(DEFAULT_BAR_PADDING * 2.0);

        for tab in tabs {
            tabs_vec.push((tab.tab_id, tab.tab_label));
            row = row.push(Container::new(tab.tab_content).width(Length::FillPortion(1)));
        }

        Tabs {
            width: Length::Fill,
            height: Length::Shrink,
            tab_bar_padding: DEFAULT_BAR_PADDING.into(),
            tab_content_padding: DEFAULT_CONTENT_PADDING.into(),
            tab_bar: row,
            tabs: tabs_vec,
            active_tab: TabId::default(),
            on_change_tab: on_change_tab.into(),
            style: <Theme as StyleSheet>::Style::default()
        }
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self
    {
        self.width = width.into();
        self.tab_bar = self.tab_bar.width(self.width);

        self
    }

    pub fn height(mut self, height: impl Into<Length>) -> Self
    {
        self.height = height.into();

        self
    }

    pub fn tab_bar_padding(mut self, tab_bar_padding: impl Into<Padding>) -> Self
    {
        self.tab_bar_padding = tab_bar_padding.into();
        self.tab_bar = self.tab_bar
            .padding(self.tab_bar_padding)
            .spacing(self.tab_bar_padding.right + self.tab_bar_padding.left);

        self
    }

    pub fn tab_content_padding(mut self, tab_content_padding: impl Into<Padding>) -> Self
    {
        self.tab_content_padding = tab_content_padding.into();

        self
    }

    pub fn active_tab(mut self, active_tab: impl Into<TabId>) -> Self
    {
        self.active_tab = active_tab.into();

        self
    }

    pub fn style(mut self, style: impl Into<<Theme as StyleSheet>::Style>) -> Self
    {
        self.style = style.into();

        self
    }
}

impl<'a, TabId, Message, Theme, Renderer> Widget<Message, Theme, Renderer> for Tabs<'a, TabId, Message, Theme, Renderer>
where
    TabId: 'a + Eq + Default,
    Message: 'a + Clone,
    Theme: 'a + StyleSheet + iced::widget::container::StyleSheet,
    Renderer: 'a + iced::advanced::Renderer
{
    fn size(&self) -> Size<Length> {
        Size::new(
            self.width,
            self.height
        )
    }

    fn layout(&self, tree: &mut Tree, renderer: &Renderer, limits: &Limits) -> Node {
        let tab_bar_limits = limits
            .loose()
            .width(self.width)
            .height(self.tab_bar.size().height);
        let tab_bar_node = self.tab_bar.layout(
            &mut tree.children[0],
            renderer,
            &tab_bar_limits
        );
        let tab_bar_size = tab_bar_node.size();

        let tab_limits = limits
            .loose()
            .width(self.width)
            .height(self.height)
            .shrink(Size::new(0.0, tab_bar_size.height))
            .shrink(self.tab_content_padding);
        let mut tab_node = self.tabs.iter()
            .find_map(|(id, element)| if *id == self.active_tab {Some(element)} else {None})
            .expect("Tab id needs to be in list of given tabs.")
            .as_widget().layout(&mut tree.children[1], renderer, &tab_limits);
        let mut tab_size = tab_node.size();

        tab_node.align_mut(Alignment::Center, Alignment::Center, tab_size);
        tab_node.move_to_mut(Point::new(0.0, tab_bar_size.height));

        tab_size = tab_size.expand(self.tab_content_padding);

        Node::with_children(
            Size::new(
                tab_size.width,
                tab_bar_size.height + tab_size.height
            ),
            vec![tab_bar_node, tab_node]
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
        
        renderer.fill_quad(
            Quad {
                bounds,
                border: appearance.border,
                shadow: Default::default(),
            },
            appearance.content_background
        );

        let mut children = layout.children();

        let tab_bar_layout = children.next().expect("Tabs needs to have a tab bar.");
        let mut tab_bar_children = tab_bar_layout.children();
        let border_radius :[f32;4]= appearance.border.radius.into();
        for (i, (tab, _)) in (0..self.tabs.len()).zip(&self.tabs) {
            let tab_layout = tab_bar_children.next().expect(&*format!("Tabs needs to have at least {} children.", i));
            let tab_bounds = tab_layout.bounds();

            let background = if *tab == self.active_tab {
                appearance.bar_selected
            } else if cursor.is_over(tab_bounds) {
                appearance.bar_hover
            } else {
                appearance.bar_background
            };

            let border = if i == 0 {
                Border::with_radius(Radius::from([border_radius[0], 0.0, 0.0, 0.0]))
            } else if i == self.tabs.len() - 1 {
                Border::with_radius(Radius::from([0.0, border_radius[1], 0.0, 0.0]))
            } else {
                Border::default()
            };

            renderer.fill_quad(
                Quad {
                    bounds: tab_bounds,
                    border,
                    shadow: Default::default(),
                },
                background
            );
        }

        self.tab_bar.draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            tab_bar_layout,
            cursor,
            viewport
        );

        let tab_layout = children.next().expect("Tabs needs to have tab.");
        self.tabs.iter().find_map(
            |(id, tab)| if *id == self.active_tab {Some(tab)} else {None}
        ).unwrap().as_widget().draw(
            &tree.children[1],
            renderer,
            theme,
            style,
            tab_layout,
            cursor,
            viewport
        );
    }

    fn children(&self) -> Vec<Tree> {
        vec![
            Tree::new(&self.tab_bar as &dyn Widget<Message, Theme, Renderer>),
            Tree::new(self.tabs.iter().find_map(
                |(id, tab)| if *id == self.active_tab {Some(tab)} else {None}
            ).unwrap())
        ]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&[
            &self.tab_bar as &dyn Widget<Message, Theme, Renderer>,
            self.tabs.iter().find_map(
                |(id, tab)| if *id == self.active_tab {Some(tab)} else {None}
            ).unwrap().as_widget()
        ])
    }

    fn operate(
        &self,
        state: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation<Message>
    ) {
        let mut children = layout.children();

        let tab_bar_layout = children.next().expect("Tabs needs to have bar.");
        self.tab_bar.operate(
            &mut state.children[0],
            tab_bar_layout,
            renderer,
            operation
        );

        let tab = children.next().expect("Tabs needs to have tab.");
        self.tabs.iter().find_map(
            |(id, tab)| if *id == self.active_tab {Some(tab)} else {None}
        ).unwrap().as_widget().operate(
            &mut state.children[1],
            tab,
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
        let mut children = layout.children();

        let tab_bar_layout = children.next().expect("Tabs needs to have bar.");
        let tab_bar_bounds = tab_bar_layout.bounds();

        if cursor.is_over(tab_bar_bounds) {
            if let Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) = event {
                let pos =
                    (((cursor.position().unwrap().x - tab_bar_bounds.x) / (tab_bar_bounds.width)
                        * (self.tabs.len() as f32)).floor() as usize).min(self.tabs.len() - 1);

                shell.publish((self.on_change_tab)(&self.tabs[pos].0));

                Status::Captured
            } else {
                Status::Ignored
            }
        } else {
            let tab_layout = children.next().expect("Tabs needs to have tab.");

            self.tabs.iter_mut().find_map(
                |(id, element)| if *id == self.active_tab {Some(element)} else {None}
            ).unwrap().as_widget_mut().on_event(
                &mut state.children[1],
                event,
                tab_layout,
                cursor,
                renderer,
                clipboard,
                shell,
                viewport
            )
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
        let mut children = layout.children();

        let tab_bar_layout = children.next().expect("Tabs needs to have bar.");
        let tab_bar_bounds = tab_bar_layout.bounds();

        if cursor.is_over(tab_bar_bounds) {
            Interaction::Pointer
        } else {
            let tab_layout = children.next().expect("Tabs needs to have tab.");

            self.tabs.iter().find_map(
                |(id, element)| if *id == self.active_tab {Some(element)} else {None}
            ).unwrap().as_widget().mouse_interaction(
                &state.children[1],
                tab_layout,
                cursor,
                viewport,
                renderer
            )
        }
    }

    fn overlay<'b>(
        &'b mut self,
        state: &'b mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        translation: Vector
    ) -> Option<iced::advanced::overlay::Element<'b, Message, Theme, Renderer>> {
        let mut children = layout.children();

        let mut states = state.children.iter_mut();
        let state_0 = states.next().unwrap();
        let state_1 = states.next().unwrap();

        let tab_bar_layout = children.next().expect("Tabs needs to have bar.");
        let tab_bar_overlay = self.tab_bar.overlay(
            state_0,
            tab_bar_layout,
            renderer,
            translation
        );

        let tab_layout = children.next().expect("Tabs needs to have tab.");
        let tab_overlay = self.tabs.iter_mut().find_map(
            |(id, element)| if *id == self.active_tab {Some(element)} else {None}
        ).unwrap().as_widget_mut().overlay(
            state_1,
            tab_layout,
            renderer,
            translation
        );

        let mut overlays = vec![];
        if let Some(tab_bar_overlay) = tab_bar_overlay {
            overlays.push(tab_bar_overlay);
        }
        if let Some(tab_overlay) = tab_overlay {
            overlays.push(tab_overlay);
        }

        if overlays.len() > 0 {
            Some(iced::advanced::overlay::Group::with_children(overlays).overlay())
        } else {
            None
        }
    }
}

impl<'a, TabId, Message, Theme, Renderer> From<Tabs<'a, TabId, Message, Theme, Renderer>> for Element<'a, Message, Theme, Renderer>
where
    TabId: 'a + Eq + Default,
    Message: 'a + Clone,
    Theme: 'a + StyleSheet + iced::widget::container::StyleSheet,
    Renderer: 'a + iced::advanced::Renderer
{
    fn from(value: Tabs<'a, TabId, Message, Theme, Renderer>) -> Self {
        Element::new(value)
    }
}

pub struct Appearance
{
    pub content_background: Background,
    pub bar_background: Background,
    pub bar_hover: Background,
    pub bar_selected: Background,
    pub border: Border
}

pub trait StyleSheet
{
    type Style: Default;

    fn active(&self, style: &Self::Style) -> Appearance;
}
