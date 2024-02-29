use iced::{Element, Renderer};
use iced_aw::modal;
use crate::scene::Message;
use crate::theme::Theme;

/// A structure that can stack overlays on top of each other
/// Useful for scenes with multiple overlays
#[derive(Clone)]
pub struct ModalStack<ModalTypes: Clone + Eq + PartialEq>
{
    stack: Vec<ModalTypes>
}

impl<ModalTypes> ModalStack<ModalTypes>
where
    ModalTypes: Clone+Eq+PartialEq
{
    pub fn new() -> ModalStack<ModalTypes>
    {
        ModalStack {
            stack: vec![]
        }
    }

    /// Attempts to toggle the given modal in the stack.
    /// If the modal is at the top of the stack, the function pops it.
    /// Otherwise, the function pushes it.
    pub fn toggle_modal(&mut self, modal: ModalTypes)
    {
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
    pub fn get_modal<'a, F>(&self, into_element: F) -> Option<Element<'a, Message, Renderer<Theme>>>
    where F: Fn(ModalTypes) -> Element<'a, Message, Renderer<Theme>>
    {
        if self.stack.len() == 0 {
            None
        } else {
            let mut modals = self.stack.clone();

            let initial = modals.pop().unwrap();
            Some(modals.iter().rfold(
                into_element(initial),
                |element, underlay| modal::<'a, Message, Renderer<Theme>>(
                    into_element(underlay.clone()),
                    Some(element)
                ).into()
            ))
        }
    }
}