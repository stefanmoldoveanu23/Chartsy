use crate::scene::Message;
use iced::widget::{Container, Stack};
use iced::Theme;
use iced::{Element, Length, Renderer};

/// A structure that can stack overlays on top of each other
/// Useful for scenes with multiple overlays
#[derive(Clone)]
pub struct ModalStack<ModalTypes: Clone + Eq + PartialEq> {
    /// The stack of modals.
    stack: Vec<ModalTypes>,
}

impl<ModalTypes> ModalStack<ModalTypes>
where
    ModalTypes: Clone + Eq + PartialEq,
{
    /// Initializes a [modal stack](ModalStack).
    pub fn new() -> ModalStack<ModalTypes> {
        ModalStack { stack: vec![] }
    }

    /// Attempts to toggle the given modal in the stack.
    /// If the modal is at the top of the stack, the function pops it.
    /// Otherwise, the function pushes it.
    pub fn toggle_modal(&mut self, modal: ModalTypes) {
        if let Some(last) = self.stack.last() {
            if modal == last.clone() {
                self.stack.pop();
            } else {
                self.stack.push(modal);
            }
        } else {
            self.stack.push(modal);
        }
    }

    /// Returns an element with the modals overlaid on top of each other.
    pub fn get_modal<'a, F>(
        &self,
        underlay: impl Into<Element<'a, Message, Theme, Renderer>>,
        into_element: F,
    ) -> Element<'a, Message, Theme, Renderer>
    where
        F: Fn(ModalTypes) -> Element<'a, Message, Theme, Renderer>,
    {
        let modals = self.stack.clone();

        Stack::with_children(
            modals
                .iter()
                .map(|modal| {
                    Element::from(
                        Container::new(into_element(modal.clone()))
                            .width(Length::Fill)
                            .height(Length::Fill),
                    )
                })
                .fold(vec![underlay.into()], |mut stack, overlay| {
                    stack.push(overlay);
                    stack
                }),
        )
        .into()
    }
}
