/// The modals that can be displayed on the [Main] [scene](Scene).
#[derive(Clone, Eq, PartialEq)]
pub enum ModalType {
    /// This modal displays the list of drawings a user can draw on.
    ShowingDrawings,

    /// This modal allows a user to create a new drawing.
    SelectingSaveMode,
}

/// The tabs for the drawing list overlay.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Default)]
pub enum MainTabIds {
    #[default]
    Offline,
    Online,
}
