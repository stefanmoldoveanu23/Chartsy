use std::fmt::{Display, Formatter, Write};
use iced::Font;
use iced::font::{Family, Weight, Stretch, Style};

pub const ICON_BYTES :&[u8]= include_bytes!("../images/SymbolsNerdFontMono-Regular.ttf");
pub const ICON :Font= Font {
    family: Family::Name("Symbols Nerd Font Mono"),
    weight: Weight::Normal,
    stretch: Stretch::Normal,
    style: Style::Normal,
};

pub enum Icon {
    X,
    Edit,
    Visible,
    Hidden,
    Add,
    Leave,
    Report,
    Loading
}

pub enum ToolIcon {
    Line,
    Rectangle,
    Triangle,
    Polygon,
    Circle,
    Ellipse,
    Pencil,
    FountainPen,
    Airbrush,
    Eraser
}

impl Display for Icon {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_char(
            match self {
                Icon::X => '\u{F015A}',
                Icon::Edit => '\u{F044}',
                Icon::Visible => '\u{F06E}',
                Icon::Hidden => '\u{F070}',
                Icon::Add => '\u{F0FE}',
                Icon::Leave => '\u{F0A8}',
                Icon::Report => '\u{F0CE7}',
                Icon::Loading => '\u{F1978}'
            }
        )
    }
}

impl Display for ToolIcon {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_char(
            match self {
                ToolIcon::Line => '\u{F055E}',
                ToolIcon::Rectangle => '\u{F05C6}',
                ToolIcon::Triangle => '\u{F0563}',
                ToolIcon::Polygon => '\u{F0560}',
                ToolIcon::Circle => '\u{F0556}',
                ToolIcon::Ellipse => '\u{F0893}',
                ToolIcon::Pencil => '\u{F03EB}',
                ToolIcon::FountainPen => '\u{F0D12}',
                ToolIcon::Airbrush => '\u{F0665}',
                ToolIcon::Eraser => '\u{F01FE}'
            }
        )
    }
}
