use std::fmt::{Display, Formatter, Write};
use iced::Font;
use iced::font::{Family, Weight, Stretch, Style};

pub const ICON_BYTES :&[u8]= include_bytes!("images/fontawesome.ttf");
pub const ICON :Font= Font {
    family: Family::Name("Font Awesome 5 Free"),
    weight: Weight::Black,
    stretch: Stretch::Normal,
    style: Style::Normal,
};

pub enum Icon {
    X,
    Edit,
    Visible,
    Hidden,
    Add
}

impl Display for Icon {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_char(
            match self {
                Icon::X => '\u{F057}',
                Icon::Edit => '\u{F044}',
                Icon::Visible => '\u{F06E}',
                Icon::Hidden => '\u{F070}',
                Icon::Add => '\u{F055}'
            }
        )
    }
}
