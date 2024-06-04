use iced::theme::palette::{Background, Danger, Primary, Secondary, Success};
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

pub fn extended_palette_generator(palette: Palette) -> palette::Extended {
    palette::Extended {
        background: Background::new(palette.background, palette.text),
        primary: Primary::generate(palette.primary, palette.background, palette.background),
        secondary: Secondary::generate(SECONDARY, palette.background),
        success: Success::generate(palette.success, palette.background, palette.text),
        danger: Danger::generate(palette.danger, palette.background, palette.text),
        is_dark: true,
    }
}

pub mod text {
    use iced::{widget::text::Style, Color};

    use super::Theme;

    pub fn danger(theme: &Theme) -> Style {
        Style {
            color: Some(theme.palette().danger),
        }
    }

    pub fn gray(_theme: &Theme) -> Style {
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

    pub fn primary_tab(theme: &Theme, status: Status) -> Style {
        let mut primary_tab = iced::widget::button::primary(theme, status);
        primary_tab.border = Border {
            radius: 0.0.into(),
            ..primary_tab.border
        };

        primary_tab
    }

    pub fn secondary_tab(theme: &Theme, status: Status) -> Style {
        let mut secondary_tab = iced::widget::button::secondary(theme, status);
        secondary_tab.border = Border {
            radius: 0.0.into(),
            ..secondary_tab.border
        };

        secondary_tab
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
