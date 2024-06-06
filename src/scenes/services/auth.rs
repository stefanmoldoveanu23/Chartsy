use iced::{
    advanced::widget::Text,
    widget::{Button, Column, TextInput},
    Element, Length, Renderer,
};

use crate::{
    scene::{Globals, Message},
    scenes::{
        auth::AuthMessage,
        data::auth::{AuthTabIds, LogInField, LogInForm, RegisterField, RegisterForm},
    },
    utils::{
        errors::AuthError,
        theme::{self, Theme},
    },
    widgets::{Centered, Tabs},
};

pub fn register_tab<'a>(
    register_form: &RegisterForm,
    register_code: &Option<String>,
    code_error: &Option<AuthError>,
    globals: &Globals,
) -> Element<'a, Message, Theme, Renderer> {
    let register_error_text = Text::new(if let Some(error) = register_form.get_error().clone() {
        error.to_string()
    } else {
        String::from("")
    })
    .size(14.0)
    .style(theme::text::danger);

    let code_error_text = Text::new(if let Some(error) = code_error.clone() {
        error.to_string()
    } else {
        String::from("")
    })
    .size(14.0)
    .style(theme::text::danger);

    if let Some(code) = &register_code {
        Column::with_children([
            Text::new("A code has been sent to your email address:").into(),
            code_error_text.into(),
            TextInput::new("Input register code...", code)
                .on_input(|value| {
                    AuthMessage::RegisterTextFieldUpdate(RegisterField::Code(value)).into()
                })
                .into(),
            Button::new("Reset code")
                .on_press(AuthMessage::ResetRegisterCode.into())
                .into(),
            Button::new("Validate")
                .on_press(AuthMessage::ValidateEmail.into())
                .into(),
        ])
    } else {
        Column::with_children([
            register_error_text.into(),
            Text::new("Email:").into(),
            TextInput::new("Input email...", &*register_form.get_email())
                .on_input(|value| {
                    AuthMessage::RegisterTextFieldUpdate(RegisterField::Email(value)).into()
                })
                .into(),
            Text::new("Username:").into(),
            TextInput::new("Input username...", &*register_form.get_username())
                .on_input(|value| {
                    AuthMessage::RegisterTextFieldUpdate(RegisterField::Username(value)).into()
                })
                .into(),
            Text::new("Password:").into(),
            TextInput::new("Input password...", &*register_form.get_password())
                .on_input(|value| {
                    AuthMessage::RegisterTextFieldUpdate(RegisterField::Password(value)).into()
                })
                .secure(true)
                .into(),
            if globals.get_db().is_some() {
                Button::new("Register")
                    .on_press(AuthMessage::SendRegister(false).into())
                    .into()
            } else {
                Button::new("Register").into()
            },
        ])
    }
    .spacing(10.0)
    .into()
}

pub fn log_in_tab<'a>(
    log_in_form: &LogInForm,
    globals: &Globals,
) -> Element<'a, Message, Theme, Renderer> {
    let log_in_error_text = Text::new(if let Some(error) = log_in_form.get_error().clone() {
        error.to_string()
    } else {
        String::from("")
    })
    .size(14.0)
    .style(theme::text::danger);

    Column::with_children([
        log_in_error_text.into(),
        Text::new("Email:").into(),
        TextInput::new("Input email...", &*log_in_form.get_email())
            .on_input(|value| AuthMessage::LogInTextFieldUpdate(LogInField::Email(value)).into())
            .into(),
        Text::new("Password:").into(),
        TextInput::new("Input password...", &*log_in_form.get_password())
            .on_input(|value| AuthMessage::LogInTextFieldUpdate(LogInField::Password(value)).into())
            .secure(true)
            .into(),
        if globals.get_db().is_some() {
            Button::new("Log In")
                .on_press(AuthMessage::SendLogIn.into())
                .into()
        } else {
            Button::new("Log In").into()
        },
    ])
    .spacing(10.0)
    .into()
}

pub fn tabs<'a>(
    register_tab: Element<'a, Message, Theme, Renderer>,
    log_in_tab: Element<'a, Message, Theme, Renderer>,
    active_tab: AuthTabIds,
) -> Element<'a, Message, Theme, Renderer> {
    Centered::new(
        Tabs::new_with_tabs(
            vec![
                (AuthTabIds::Register, String::from("Register"), register_tab),
                (AuthTabIds::LogIn, String::from("Login"), log_in_tab),
            ],
            |tab_id| AuthMessage::TabSelection(tab_id).into(),
        )
        .selected(active_tab)
        .width(Length::Fill)
        .height(Length::Fill),
    )
    .height(0.75)
    .into()
}
