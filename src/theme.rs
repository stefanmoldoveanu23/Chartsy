use iced::application::{StyleSheet, Appearance};
use iced_style::theme::Application;

/// Custom theme created for the drawing [Application].
#[derive(Default, Debug, Clone, Copy)]
pub struct Theme;

impl StyleSheet for Theme {
    type Style = ();

    fn appearance(&self, _style: &Self::Style) -> Appearance {
        iced::Theme::Light.appearance(&Application::default())
    }
}

/// Module that implements the [text](iced::widget::text::Text) [StyleSheet] for the custom [Theme].
pub(crate) mod text {
    use iced::widget::text::{StyleSheet, Appearance};
    use crate::theme::Theme;
    use iced_style::theme::Text;

    impl StyleSheet for Theme {
        type Style = Text;

        fn appearance(&self, style: Self::Style) -> Appearance {
            iced::Theme::Light.appearance(style)
        }
    }
}

pub(crate) mod text_input {
    use iced::Color;
    use iced::widget::text_input::{StyleSheet, Appearance};
    use crate::theme::Theme;
    use iced_style::theme::TextInput;

    impl StyleSheet for Theme {
        type Style = TextInput;

        fn active(&self, style: &Self::Style) -> Appearance {
            iced::Theme::Light.active(style)
        }

        fn focused(&self, style: &Self::Style) -> Appearance {
            iced::Theme::Light.focused(style)
        }

        fn placeholder_color(&self, style: &Self::Style) -> Color {
            iced::Theme::Light.placeholder_color(style)
        }

        fn value_color(&self, style: &Self::Style) -> Color {
            iced::Theme::Light.value_color(style)
        }

        fn disabled_color(&self, style: &Self::Style) -> Color {
            iced::Theme::Light.disabled_color(style)
        }

        fn selection_color(&self, style: &Self::Style) -> Color {
            iced::Theme::Light.selection_color(style)
        }

        fn disabled(&self, style: &Self::Style) -> Appearance {
            iced::Theme::Light.disabled(style)
        }
    }
}

/// Module that implements the [button](iced::widget::button::Button) [StyleSheet] for the custom [Theme].
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

/// Module that implements the [container](iced::widget::container::Container) [StyleSheet]
/// for the custom [Theme].
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

/// Module that implements the [scrollable](iced::widget::scrollable::Scrollable) [StyleSheet]
/// for the custom [Theme].
pub(crate) mod scrollable {
<<<<<<< Updated upstream
    use iced::widget::scrollable::{StyleSheet, Appearance};
=======
    use crate::theme::Theme;
    use iced::widget::scrollable::{Appearance, StyleSheet};
>>>>>>> Stashed changes
    use iced_style::theme::Scrollable;
    use crate::theme::Theme;

    impl StyleSheet for Theme {
        type Style = Scrollable;

        fn active(&self, style: &Self::Style) -> Appearance {
            iced::Theme::Light.active(style)
        }

        fn hovered(&self, style: &Self::Style, is_mouse_over_scrollbar: bool) -> Appearance {
            iced::Theme::Light.hovered(style, is_mouse_over_scrollbar)
        }
    }
}

/// Module that implements the [slider](iced::widget::slider::Slider) [StyleSheet] for the
/// custom [Theme].
pub(crate) mod slider {
    use iced::widget::slider::{Appearance, StyleSheet};
    use iced_style::theme::Slider;
    use crate::theme::Theme;

    impl StyleSheet for Theme {
        type Style = Slider;

        fn active(&self, style: &Self::Style) -> Appearance {
            iced::Theme::Light.active(style)
        }

        fn hovered(&self, style: &Self::Style) -> Appearance {
            iced::Theme::Light.hovered(style)
        }

        fn dragging(&self, style: &Self::Style) -> Appearance {
            iced::Theme::Light.dragging(style)
        }
    }
}

/// Module that implements the [svg](iced::widget::svg::Svg) [StyleSheet] for the custom [Theme].
pub(crate) mod svg {
    use crate::theme::Theme;
    use iced::widget::svg::{Appearance, StyleSheet};
    use iced_style::theme::Svg;

    impl StyleSheet for Theme {
        type Style = Svg;

        fn appearance(&self, style: &Self::Style) -> Appearance {
            iced::Theme::Light.appearance(style)
        }
    }
}

/// Module that implements the [modal](iced_aw::modal::Modal) [StyleSheet] for the custom [Theme].

/// Module that implements the [card](iced_aw::card::Card) [StyleSheet] for the custom [Theme].
pub(crate) mod card {
    use iced::{Border, Color};
    use iced_runtime::core::Background;
    use super::Theme;
    use crate::widgets::card::{Appearance, StyleSheet};

    #[derive(Default)]
    pub enum Card {
        #[default]
        Default,
    }
    
    impl StyleSheet for Theme {
        type Style = Card;

        fn active(&self, style: &Self::Style) -> Appearance {
            match style {
                Card::Default => {
                    Appearance {
                        header_background: Background::Color(Color::from_rgb(0.0, 0.2, 1.0)),
                        content_background: Background::Color(Color::WHITE),
                        border: Border {
                            color: Color::from_rgb(0.0, 0.2, 1.0),
                            width: 2.0,
                            radius: 10.0.into(),
                        },
                    }
                }
            }
        }
    }
}

/// Module that implements the [tabs](crate::widgets::tabs::Tabs) [StyleSheet] for the
/// custom [Theme].
pub(crate) mod tabs {
    use iced::{Background, Border, Color};
    use super::Theme;
    use crate::widgets::tabs::{Appearance, StyleSheet};

    #[derive(Default)]
    pub enum Tabs {
        #[default]
        Default,
        NoBorder,
        SquareBorder,
    }

    impl StyleSheet for Theme {
        type Style = Tabs;

        fn active(&self, style: &Self::Style) -> Appearance {
            match style {
                Tabs::Default => {
                    Appearance {
                        content_background: Background::Color(Color::WHITE),
                        bar_background: Background::Color(Color::from_rgb(0.0, 0.2, 1.0)),
                        bar_hover: Background::Color(Color::from_rgb(0.5, 0.7, 1.0)),
                        bar_selected: Background::Color(Color::WHITE),
                        border: Border {
                            color: Color::from_rgb(0.0, 0.2, 1.0),
                            width: 2.0,
                            radius: 10.0.into(),
                        },
                    }
                },
                Tabs::NoBorder => {
                    Appearance {
                        content_background: Background::Color(Color::WHITE),
                        bar_background: Background::Color(Color::from_rgb(0.0, 0.2, 1.0)),
                        bar_hover: Background::Color(Color::from_rgb(0.5, 0.7, 1.0)),
                        bar_selected: Background::Color(Color::WHITE),
                        border: Default::default(),
                    }
                },
                Tabs::SquareBorder => {
                    Appearance {
                        content_background: Background::Color(Color::WHITE),
                        bar_background: Background::Color(Color::from_rgb(0.0, 0.2, 1.0)),
                        bar_hover: Background::Color(Color::from_rgb(0.5, 0.7, 1.0)),
                        bar_selected: Background::Color(Color::WHITE),
                        border: Border {
                            color: Color::from_rgb(0.0, 0.2, 1.0),
                            width: 2.0,
                            radius: Default::default(),
                        }
                    }
                }
            }
        }
    }
}

/// Module that implements the [post](crate::widgets::post_summary::PostSummary) [StyleSheet]
/// for the custom [Theme].
pub(crate) mod post {
    use iced::Color;
    use crate::theme::Theme;
    use crate::widgets::post_summary::{Appearance, StyleSheet};

    #[derive(Default)]
    pub enum PostSummary {
        #[default]
        Default
    }

    impl StyleSheet for Theme {
        type Style = PostSummary;

        fn active(&self, style: &Self::Style) -> Appearance {
            match style {
                PostSummary::Default => {
                    Appearance {
                        border_color: Color::from_rgb(0.5, 0.5, 0.5),
                        ..Appearance::default()
                    }
                }
            }
        }
    }
}

/// Module that implements the [closeable](crate::widgets::closeable::Closeable) [StyleSheet]
/// for the custom [Theme].
pub(crate) mod closeable {
    use iced::Color;
    use iced_runtime::core::Background;
    use crate::theme::Theme;
    use crate::widgets::closeable::{Appearance, StyleSheet};

    #[derive(Default)]
    pub enum Closeable {
        #[default]
        Default,
        Monochrome(Color),
        SpotLight,
        Transparent
    }
    
    impl StyleSheet for Theme {
        type Style = Closeable;

        fn active(&self, style: &Self::Style) -> Appearance {
            match style {
                Closeable::Default => {
                    Appearance::default()
                }
                Closeable::Monochrome(color) => {
                    Appearance {
                        background: Background::Color(*color)
                    }
                }
                Closeable::SpotLight => {
                    Appearance {
                        background: Background::Color(Color::from_rgba(0.8, 0.8, 0.8, 0.5))
                    }
                }
                Closeable::Transparent => {
                    Appearance {
                        background: Background::Color(Color::TRANSPARENT)
                    }
                }
            }
        }
    }
}
