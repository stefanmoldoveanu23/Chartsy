pub mod card;
pub mod close;
pub mod closeable;
pub mod color_picker;
pub mod combo_box;
pub mod grid;
pub mod modal_stack;
pub mod post_summary;
pub mod rating;
pub mod tabs;
pub mod wait_panel;

pub type Card<'a, Message, Theme, Renderer> = card::Card<'a, Message, Theme, Renderer>;

pub type Close<Message> = close::Close<Message>;

pub type Closeable<'a, Message, Theme, Renderer> =
    closeable::Closeable<'a, Message, Theme, Renderer>;

pub type ColorPicker<Message> = color_picker::ColorPicker<Message>;

pub type ComboBox<'a, Tag, Message, Theme, Renderer> =
    combo_box::ComboBox<'a, Tag, Message, Theme, Renderer>;

pub type Grid<'a, Message, Theme, Renderer> = grid::Grid<'a, Message, Theme, Renderer>;

pub type ModalStack<ModalTypes> = modal_stack::ModalStack<ModalTypes>;

pub type PostSummary<'a, Message, Theme, Renderer> =
    post_summary::PostSummary<'a, Message, Theme, Renderer>;

pub type Rating<Message, F> = rating::Rating<Message, F>;

pub type Tabs<'a, Type, Message, Theme, Renderer> = tabs::Tabs<'a, Type, Message, Theme, Renderer>;

pub type WaitPanel = wait_panel::WaitPanel;
