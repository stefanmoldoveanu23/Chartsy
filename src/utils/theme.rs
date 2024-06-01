use iced::theme::{palette, Palette};
use iced::{color, Color};

pub type Theme = iced::Theme;

pub const BACKGROUND: Color = color!(0x24273a);
pub const TEXT: Color = color!(0xcad3f5);
pub const DANGER: Color = color!(0xed8796);
pub const SUCCESS: Color = color!(0xa6da95);
pub const PRIMARY: Color = color!(0x8aadf4);
pub const SECONDARY: Color = color!(0x3d4967);

pub const PALETTE: Palette = Palette {
    background: BACKGROUND,
    text: TEXT,
    primary: PRIMARY,
    success: SUCCESS,
    danger: DANGER,
};

pub mod text {
    use iced::{widget::text::Style, Color};

    use super::Theme;

    pub fn danger(theme: &Theme) -> Style {
        Style {
            color: Some(theme.palette().danger),
        }
    }

    pub fn gray(theme: &Theme) -> Style {
        Style {
            color: Some(Color::from_rgb(0.5, 0.5, 0.5)),
        }
    }
}

pub mod button {
    use iced::{
        widget::button::{Status, Style},
        Border,
    };

    use super::Theme;

    pub fn danger(theme: &Theme, status: Status) -> Style {
        let background = match status {
            Status::Disabled | Status::Hovered => theme.extended_palette().background.weak,
            Status::Pressed => theme.extended_palette().background.strong,
            Status::Active => theme.extended_palette().background.base,
        };

        Style {
            background: Some(background.color.into()),
            text_color: background.text,
            border: Border {
                color: theme.extended_palette().secondary.base.color,
                width: 2.0,
                radius: 20.0.into(),
            },
            ..Default::default()
        }
    }
}

pub mod container {
    use iced::{widget::container::Style, Border};

    use super::Theme;

    pub fn badge(theme: &Theme) -> Style {
        Style {
            background: Some(iced::Background::Color(theme.palette().text)),
            border: Border {
                color: theme.extended_palette().secondary.base.color,
                width: 2.0,
                radius: 20.0.into(),
            },
            ..Default::default()
        }
    }
}

/// Module that implements the [closeable](crate::widgets::closeable::Closeable) [StyleSheet]
/// for the custom [Theme].
pub mod closeable {
    use super::Theme;
    use crate::widgets::closeable::{Appearance, StyleSheet};
    use iced::{Background, Color};

    #[derive(Default)]
    pub enum Closeable {
        #[default]
        Default,
        Monochrome(Color),
        SpotLight,
        Transparent,
    }

    impl StyleSheet for Theme {
        type Style = Closeable;

        fn active(&self, style: &Self::Style) -> Appearance {
            match style {
                Closeable::Default => Appearance::default(),
                Closeable::Monochrome(color) => Appearance {
                    background: Background::Color(*color),
                },
                Closeable::SpotLight => Appearance {
                    background: Background::Color(Color::from_rgba(0.8, 0.8, 0.8, 0.5)),
                },
                Closeable::Transparent => Appearance {
                    background: Background::Color(Color::TRANSPARENT),
                },
            }
        }
    }
}

/// Module that implements the [card](iced_aw::card::Card) [StyleSheet] for the custom [Theme].
pub mod card {
    use super::Theme;
    use crate::widgets::card::{Appearance, StyleSheet};
    use iced::Background;

    #[derive(Default)]
    pub enum Card {
        #[default]
        Default,
    }

    impl StyleSheet for Theme {
        type Style = Card;

        fn active(&self, style: &Self::Style) -> Appearance {
            match style {
                Card::Default => Appearance {
                    background: Background::Color(super::BACKGROUND),
                    header_background: Background::Color(super::SECONDARY),
                    border_color: super::SECONDARY,
                },
            }
        }
    }
}

/// Module that implements the [post](crate::widgets::post_summary::PostSummary) [StyleSheet]
/// for the custom [Theme].
pub mod post {
    use super::Theme;
    use crate::widgets::post_summary::{Appearance, StyleSheet};
    use iced::Color;

    #[derive(Default)]
    pub enum PostSummary {
        #[default]
        Default,
    }

    impl StyleSheet for Theme {
        type Style = PostSummary;

        fn active(&self, style: &Self::Style) -> Appearance {
            match style {
                PostSummary::Default => Appearance {
                    border_color: Color::from_rgb(0.5, 0.5, 0.5),
                    ..Appearance::default()
                },
            }
        }
    }
}

/*use iced::application::{Appearance, StyleSheet};
use iced::theme::Application;
use iced::theme::Theme;

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
    use iced::{color, Color};

    pub const BACKGROUND: Color = color!(0x24273a);
    pub const TEXT: Color = color!(0xcad3f5);
    pub const DANGER: Color = color!(0xed8796);
    pub const SUCCESS: Color = color!(0xa6da95);
    pub const PRIMARY: Color = color!(0x8aadf4);
    pub const HIGHLIGHT: Color = color!(0x3d4967);
}

/// Module that implements the [text](iced::widget::text::Text) [StyleSheet] for the custom [Theme].
pub mod text {
    use super::Theme;
    use iced::widget::text::{Appearance, StyleSheet};
    use iced::Color;

    #[derive(Clone, Default)]
    pub enum Text {
        #[default]
        Default,
        Light,
        Dark,
        Error,
        Gray,
        Custom(Color),
    }

    impl StyleSheet for Theme {
        type Style = Text;

        fn appearance(&self, style: Self::Style) -> Appearance {
            match style {
                Text::Default => Appearance { color: None },
                Text::Light => Appearance {
                    color: Some(super::pallete::TEXT),
                },
                Text::Dark => Appearance {
                    color: Some(super::pallete::BACKGROUND),
                },
                Text::Error => Appearance {
                    color: Some(super::pallete::DANGER),
                },
                Text::Gray => Appearance {
                    color: Some(Color::from_rgb(0.5, 0.5, 0.5)),
                },
                Text::Custom(color) => Appearance { color: Some(color) },
            }
        }
    }
}

/// Module that implements the [text input](iced::widget::text_input::TextInput) [StyleSheet]
/// for the custom [Theme].
pub mod text_input {
    use super::Theme;
    use iced::theme::TextInput;
    use iced::widget::text_input::{Appearance, StyleSheet};
    use iced::Color;

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
pub mod button {
    use super::Theme;
    use iced::widget::button::{Appearance, StyleSheet};
    use iced::{Background, Border};

    pub enum Button {
        Button(iced::theme::Button),
        Transparent,
        UnselectedLayer,
        SelectedLayer,
        Danger,
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
                        radius: 10.0.into(),
                    },
                    ..Default::default()
                },
                Button::Danger => Appearance {
                    background: Some(Background::Color(super::pallete::DANGER)),
                    border: Border {
                        color: super::pallete::TEXT,
                        width: 1.0,
                        radius: 10.0.into(),
                    },
                    text_color: super::pallete::TEXT,
                    ..Default::default()
                },
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
                        radius: 10.0.into(),
                    },
                    ..Default::default()
                },
                _ => self.active(style),
            }
        }
    }
}

/// Module that implements the [container](iced::widget::container::Container) [StyleSheet]
/// for the custom [Theme].
pub mod container {
    use iced::widget::container::{Appearance, StyleSheet};
    use iced::{Background, Border, Color};

    #[derive(Default)]
    pub enum Container {
        #[default]
        Default,
        Bordered,
        Badge(Color),
        Panel(Background),
    }
    impl StyleSheet for super::Theme {
        type Style = Container;

        fn appearance(&self, style: &Self::Style) -> Appearance {
            match style {
                Container::Default => iced::Theme::CatppuccinMacchiato
                    .appearance(&iced::theme::Container::Transparent),
                Container::Bordered => Appearance {
                    border: Border {
                        color: super::pallete::HIGHLIGHT,
                        width: 2.0,
                        radius: Default::default(),
                    },
                    ..Appearance::default()
                },
                Container::Badge(background) => {
                    let contrast = if *background == super::pallete::TEXT
                        || *background == super::pallete::HIGHLIGHT
                    {
                        super::pallete::BACKGROUND
                    } else {
                        super::pallete::TEXT
                    };

                    Appearance {
                        border: Border {
                            color: contrast,
                            width: 2.0,
                            radius: 20.0.into(),
                        },
                        background: Some(Background::Color(*background)),
                        text_color: Some(contrast),
                        ..Default::default()
                    }
                }
                Container::Panel(background) => Appearance {
                    background: Some(*background),
                    ..Default::default()
                },
            }
        }
    }
}

/// Module that implements the [scrollable](iced::widget::scrollable::Scrollable) [StyleSheet]
/// for the custom [Theme].
pub mod scrollable {
    use super::Theme;
    use iced::theme::Scrollable;
    use iced::widget::scrollable::{Appearance, StyleSheet};

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
pub mod slider {
    use super::Theme;
    use iced::theme::Slider;
    use iced::widget::slider::{Appearance, StyleSheet};

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
pub mod svg {
    use super::Theme;
    use iced::theme::Svg;
    use iced::widget::svg::{Appearance, StyleSheet};

    impl StyleSheet for Theme {
        type Style = Svg;

        fn appearance(&self, style: &Self::Style) -> Appearance {
            iced::Theme::CatppuccinMacchiato.appearance(style)
        }
    }
}



/// Module that implements the [tab_bar](iced_aw::tab_bar::TabBar) [StyleSheet] for the
/// custom [Theme].
pub mod tab_bar {
    use super::Theme;
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
pub mod badge {
    use super::Theme;
    use iced_aw::style::badge::{Appearance, StyleSheet};

    impl StyleSheet for Theme {
        type Style = iced_aw::style::BadgeStyles;

        fn active(&self, style: &Self::Style) -> Appearance {
            iced::Theme::CatppuccinMacchiato.active(&style)
        }
    }
}

*/
