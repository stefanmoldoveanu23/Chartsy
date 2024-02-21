/*use iced::{Element, Length, Size};
use iced::advanced::layout::{Limits, Node};
use iced::advanced::{Widget};
use iced::advanced::widget::Tree;
use iced::widget::{Container, text};

const DEFAULT_PADDING :f32= 10.0;

pub struct Tabs<'a, TabId, Message, Theme, Renderer>
where
    TabId: PartialEq + Eq + Default,
    Message: Clone,
    Renderer: iced::advanced::Renderer + iced::advanced::text::Renderer
{
    width: Length,
    height: Length,
    tabs: Vec<(Element<'a, Message, Theme, Renderer>, TabId, Element<'a, Message, Theme, Renderer>)>,
    content_padding: f32,
}

impl<'a, TabId, Message, Theme, Renderer> Tabs<'a, TabId, Message, Theme, Renderer>
where
    TabId: PartialEq + Eq + Default,
    Message: Clone,
    Theme: iced::widget::container::StyleSheet + text::StyleSheet,
    Renderer: iced::advanced::Renderer + iced::advanced::text::Renderer,
{
    pub fn new(tabs: Vec<(impl Into<String>, impl Into<TabId>, impl Into<Element<'a, Message, Theme, Renderer>>)>) -> Self
    {
        let mut res = Tabs {
            width: Length::Shrink,
            height: Length::Shrink,
            tabs: vec![],
            content_padding: DEFAULT_PADDING
        };

        for (label, id, content) in tabs {
            let size = Size::new(20.0, 10.0);
            let textbox :Element<'a, Message, Theme, Renderer>= Container::new(text(label.into())).into();
            res.tabs.push((textbox, id.into(), content.into()));
        }

        res
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

    pub fn content_padding(mut self, content_padding: impl Into<f32>) -> Self
    {
        self.content_padding = content_padding.into();

        self
    }
}

impl<'a, TabId, Message, Theme, Renderer> Widget<Message, Theme, Renderer> for Tabs<'a, TabId, Message, Theme, Renderer>
where
    TabId: PartialEq + Eq + Default,
    Message: Clone,
    Renderer: iced::advanced::Renderer + iced::advanced::text::Renderer
{
    fn size(&self) -> Size<Length>
    {
        Size::new(
            self.width,
            self.height
        )
    }

    fn layout(
        &self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &Limits
    ) -> Node {


        todo!()
    }
}

fn get_tab_bar_layout<Renderer> (
    tree: &mut Tree,
    renderer: &Renderer,
    limits: &Limits,
    tab_count: usize,
    width: Length,
) -> Node
where
    Renderer: iced::advanced::Renderer + iced::advanced::text::Renderer
{
    let limits = limits
        .loose()
        .width(width);

    limits.
}*/
