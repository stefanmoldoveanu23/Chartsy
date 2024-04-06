use iced::application::{Appearance, StyleSheet};
use iced::theme::Application;

/// Custom theme created for the drawing [Application].
#[derive(Default, Debug, Clone, Copy)]
pub struct Theme;

impl StyleSheet for Theme {
    type Style = ();

    fn appearance(&self, _style: &Self::Style) -> Appearance {
        iced::Theme::CatppuccinMacchiato.appearance(&Application::default())
    }
}

pub mod pallete {
    use iced::{Color, color};

    pub const BACKGROUND :Color= color!(0x24273a);
    pub const TEXT :Color= color!(0xcad3f5);
    pub const DANGER :Color= color!(0xed8796);
    pub const SUCCESS :Color= color!(0xa6da95);
    pub const PRIMARY :Color= color!(0x8aadf4);
    pub const HIGHLIGHT :Color= color!(0x3d4967);
}

/// Module that implements the [text](iced::widget::text::Text) [StyleSheet] for the custom [Theme].
pub(crate) mod text {
    use crate::theme::Theme;
    use iced::widget::text::{Appearance, StyleSheet};
    use iced::theme::Text;

    impl StyleSheet for Theme {
        type Style = Text;

        fn appearance(&self, style: Self::Style) -> Appearance {
            iced::Theme::CatppuccinMacchiato.appearance(style)
        }
    }
}

/// Module that implements the [text input](iced::widget::text_input::TextInput) [StyleSheet]
/// for the custom [Theme].
pub(crate) mod text_input {
    use crate::theme::Theme;
    use iced::widget::text_input::{Appearance, StyleSheet};
    use iced::Color;
    use iced::theme::TextInput;

    impl StyleSheet for Theme {
        type Style = TextInput;

        fn active(&self, style: &Self::Style) -> Appearance {
            iced::Theme::CatppuccinMacchiato.active(style)
        }

        fn focused(&self, style: &Self::Style) -> Appearance {
            iced::Theme::CatppuccinMacchiato.focused(style)
        }

        fn placeholder_color(&self, style: &Self::Style) -> Color {
            iced::Theme::CatppuccinMacchiato.placeholder_color(style)
        }

        fn value_color(&self, style: &Self::Style) -> Color {
            iced::Theme::CatppuccinMacchiato.value_color(style)
        }

        fn disabled_color(&self, style: &Self::Style) -> Color {
            iced::Theme::CatppuccinMacchiato.disabled_color(style)
        }

        fn selection_color(&self, style: &Self::Style) -> Color {
            iced::Theme::CatppuccinMacchiato.selection_color(style)
        }

        fn disabled(&self, style: &Self::Style) -> Appearance {
            iced::Theme::CatppuccinMacchiato.disabled(style)
        }
    }
}

/// Module that implements the [button](iced::widget::button::Button) [StyleSheet] for the custom [Theme].
pub(crate) mod button {
    use iced::{Background, Border};
    use crate::theme::Theme;
    use iced::widget::button::{Appearance, StyleSheet};

    pub enum Button {
        Button(iced::theme::Button),
        Transparent,
        UnselectedLayer,
        SelectedLayer,
    }

    impl Default for Button {
        fn default() -> Self {
            Button::Button(iced::theme::Button::default())
        }
    }

    impl StyleSheet for Theme {
        type Style = Button;

        fn active(&self, style: &Self::Style) -> Appearance {
            match style {
                Button::Button(style) => iced::Theme::CatppuccinMacchiato.active(style),
                Button::Transparent => Appearance {
                    background: None,
                    text_color: super::pallete::TEXT,
                    ..Default::default()
                },
                Button::UnselectedLayer => Appearance {
                    background: None,
                    text_color: super::pallete::TEXT,
                    border: Border {
                        color: super::pallete::TEXT,
                        width: 1.0,
                        radius: 10.0.into(),
                    },
                    ..Default::default()
                },
                Button::SelectedLayer => Appearance {
                    background: Some(Background::Color(super::pallete::PRIMARY)),
                    border: Border {
                        color: super::pallete::TEXT,
                        width: 1.0,
                        radius: 10.0.into()
                    },
                    ..Default::default()
                }
            }
        }

        fn hovered(&self, style: &Self::Style) -> Appearance {
            match style {
                Button::UnselectedLayer => Appearance {
                    background: Some(Background::Color(super::pallete::HIGHLIGHT)),
                    text_color: super::pallete::TEXT,
                    border: Border {
                        color: super::pallete::TEXT,
                        width: 1.0,
                        radius: 10.0.into()
                    },
                    ..Default::default()
                },
                _ => self.active(style)
            }
        }
    }
}

/// Module that implements the [container](iced::widget::container::Container) [StyleSheet]
/// for the custom [Theme].
pub(crate) mod container {
    use iced::widget::container::{Appearance, StyleSheet};
    use iced::Border;

    #[derive(Default)]
    pub enum Container {
        #[default]
        Default,
        Bordered,
    }
    impl StyleSheet for super::Theme {
        type Style = Container;

        fn appearance(&self, style: &Self::Style) -> Appearance {
            match style {
                Container::Default => {
                    iced::Theme::CatppuccinMacchiato.appearance(&iced::theme::Container::Transparent)
                }
                Container::Bordered => Appearance {
                    border: Border {
                        color: iced::theme::palette::Palette::GRUVBOX_DARK.text,
                        width: 2.0,
                        radius: Default::default(),
                    },
                    ..Appearance::default()
                },
            }
        }
    }
}

/// Module that implements the [scrollable](iced::widget::scrollable::Scrollable) [StyleSheet]
/// for the custom [Theme].
pub(crate) mod scrollable {
    use crate::theme::Theme;
    use iced::widget::scrollable::{Appearance, StyleSheet};
    use iced::theme::Scrollable;

    impl StyleSheet for Theme {
        type Style = Scrollable;

        fn active(&self, style: &Self::Style) -> Appearance {
            iced::Theme::CatppuccinMacchiato.active(style)
        }

        fn hovered(&self, style: &Self::Style, is_mouse_over_scrollbar: bool) -> Appearance {
            iced::Theme::CatppuccinMacchiato.hovered(style, is_mouse_over_scrollbar)
        }
    }
}

/// Module that implements the [slider](iced::widget::slider::Slider) [StyleSheet] for the
/// custom [Theme].
pub(crate) mod slider {
    use crate::theme::Theme;
    use iced::widget::slider::{Appearance, StyleSheet};
    use iced::theme::Slider;

    impl StyleSheet for Theme {
        type Style = Slider;

        fn active(&self, style: &Self::Style) -> Appearance {
            iced::Theme::CatppuccinMacchiato.active(style)
        }

        fn hovered(&self, style: &Self::Style) -> Appearance {
            iced::Theme::CatppuccinMacchiato.hovered(style)
        }

        fn dragging(&self, style: &Self::Style) -> Appearance {
            iced::Theme::CatppuccinMacchiato.dragging(style)
        }
    }
}

/// Module that implements the [svg](iced::widget::svg::Svg) [StyleSheet] for the custom [Theme].
pub(crate) mod svg {
    use crate::theme::Theme;
    use iced::widget::svg::{Appearance, StyleSheet};
    use iced::theme::Svg;

    impl StyleSheet for Theme {
        type Style = Svg;

        fn appearance(&self, style: &Self::Style) -> Appearance {
            iced::Theme::CatppuccinMacchiato.appearance(style)
        }
    }
}

/// Module that implements the [modal](iced_aw::modal::Modal) [StyleSheet] for the custom [Theme].
pub(crate) mod modal {
    use crate::theme::Theme;
    use iced_aw::modal::StyleSheet;
    use iced_aw::style::modal::Appearance;

    impl StyleSheet for Theme {
        type Style = ();

        fn active(&self, _style: &Self::Style) -> Appearance {
            Appearance::default()
        }
    }
}

/// Module that implements the [card](iced_aw::card::Card) [StyleSheet] for the custom [Theme].
pub(crate) mod card {
    use iced::{Background, Color};
    use crate::theme::Theme;
    use crate::widgets::card::{Appearance, StyleSheet};
    
    #[derive(Default)]
    pub enum Card {
        #[default]
        Default
    }

    impl StyleSheet for Theme {
        type Style = Card;

        fn active(&self, style: &Self::Style) -> Appearance {
            match style {
                Card::Default => {
                    Appearance {
                        background: Background::Color(Color::WHITE),
                        header_background: Background::Color(Color::from_rgb(0.0, 0.2, 1.0)),
                        border_color: Color::from_rgb(0.0, 0.2, 1.0),
                    }
                }
            }
        }
    }
}

/// Module that implements the [tab_bar](iced_aw::tab_bar::TabBar) [StyleSheet] for the
/// custom [Theme].
pub(crate) mod tab_bar {
    use crate::theme::Theme;
    use iced_aw::tab_bar::{Appearance, StyleSheet};

    impl StyleSheet for Theme {
        type Style = iced_aw::style::tab_bar::TabBarStyles;

        fn active(&self, style: &Self::Style, is_active: bool) -> Appearance {
            iced::Theme::CatppuccinMacchiato.active(style, is_active)
        }

        fn hovered(&self, style: &Self::Style, is_active: bool) -> Appearance {
            iced::Theme::CatppuccinMacchiato.hovered(style, is_active)
        }
    }
}

/// Module that implements the [badge](iced_aw::badge::Badge) [StyleSheet] for the custom [Theme].
pub(crate) mod badge {
    use super::Theme;
    use iced_aw::style::badge::{Appearance, StyleSheet};
    
    impl StyleSheet for Theme {
        type Style = iced_aw::style::BadgeStyles;

        fn active(&self, style: &Self::Style) -> Appearance {
            iced::Theme::CatppuccinMacchiato.active(&style)
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
    use iced::{Background, Color};
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
