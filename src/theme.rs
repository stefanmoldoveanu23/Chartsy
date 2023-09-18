use iced::application::{StyleSheet, Appearance};
use iced_style::theme::Application;

#[derive(Default, Debug, Clone, Copy)]
pub struct Theme;

impl StyleSheet for Theme {
    type Style = ();

    fn appearance(&self, _style: &Self::Style) -> Appearance {
        iced::Theme::Light.appearance(&Application::default())
    }
}

pub(crate) mod text {
    use iced::Color;
    use iced::widget::text::{StyleSheet, Appearance};
    use crate::theme::Theme;
    use iced_style::theme::Text;

    impl StyleSheet for Theme {
        type Style = ();

        fn appearance(&self, _style: Self::Style) -> Appearance {
            iced::Theme::Light.appearance(Text::Color(Color::BLACK))
        }
    }
}

pub(crate) mod button {
    use iced::widget::button::{StyleSheet, Appearance};
    use crate::theme::Theme;
    use iced_style::theme::Button;

    impl StyleSheet for Theme {
        type Style = ();

        fn active(&self, _style: &Self::Style) -> Appearance {
            iced::Theme::Dark.active(&Button::default())
        }
    }
}

pub(crate) mod container {
    use iced::widget::container::{Appearance, StyleSheet};
    use iced::Color;
    use iced_runtime::core::Background;

    #[derive(Default)]
    pub enum Container {
        #[default] Default,
        Canvas
    }
    impl StyleSheet for super::Theme {
        type Style = Container;

        fn appearance(&self, style: &Self::Style) -> Appearance {
            match style {
                Container::Default => {
                    iced::Theme::Light.appearance(&iced_style::theme::Container::Transparent)
                }
                Container::Canvas => {
                    Appearance {
                        background: Some(Background::Color(Color::BLACK)),
                        ..Appearance::default()
                    }
                }
            }
        }
    }
}

pub(crate) mod scrollable {
    use iced::widget::scrollable::{Scrollbar, StyleSheet};
    use iced_style::theme::Scrollable;
    use crate::theme::Theme;

    impl StyleSheet for Theme {
        type Style = Scrollable;

        fn active(&self, style: &Self::Style) -> Scrollbar {
            iced::Theme::Light.active(style)
        }

        fn hovered(&self, style: &Self::Style, is_mouse_over_scrollbar: bool) -> Scrollbar {
            iced::Theme::Light.hovered(style, is_mouse_over_scrollbar)
        }
    }
}

pub(crate) mod modal {
    use iced_aw::modal::StyleSheet;
    use iced_aw::style::modal::Appearance;
    use crate::theme::Theme;

    impl StyleSheet for Theme {
        type Style = ();

        fn active(&self, _style: &Self::Style) -> Appearance {
            Appearance::default()
        }
    }
}

pub(crate) mod card {
    use iced_aw::card::{Appearance, StyleSheet};
    use crate::theme::Theme;

    impl StyleSheet for Theme {
        type Style = ();

        fn active(&self, _style: &Self::Style) -> Appearance {
            Appearance::default()
        }
    }
}