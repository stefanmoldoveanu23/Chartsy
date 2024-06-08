use std::ops::Deref;

use iced::{
    widget::{image::Handle, Button, Column, Image, Row, Space, Text, TextInput},
    Alignment, Element, Length, Renderer,
};
use image::load_from_memory;
use mongodb::{
    bson::{doc, Uuid},
    Database,
};
use rfd::AsyncFileDialog;

use crate::{
    database, debug_message,
    scene::Message,
    scenes::{data::auth::User, settings::SettingsMessage},
    utils::{
        errors::{AuthError, Error},
        theme::{self, Theme},
    },
    widgets::WaitPanel,
};

pub async fn get_profile_picture(user: &User) -> Result<Vec<u8>, Error> {
    database::base::download_file(if user.has_profile_picture() {
        format!("/{}/profile_picture.webp", user.get_id())
    } else {
        String::from("/default_profile_picture.webp")
    })
    .await
}

pub async fn select_image() -> Result<Vec<u8>, Error> {
    let file = AsyncFileDialog::new()
        .add_filter("image", &["png", "jpg", "jpeg", "webp", "tiff", "bmp"])
        .set_directory("~")
        .pick_file()
        .await;

    match file {
        Some(file) => {
            if tokio::fs::metadata(file.path())
                .await
                .map_err(|err| debug_message!("{}", err).into())?
                .len()
                > 5000000
            {
                Err(Error::AuthError(AuthError::ProfilePictureTooLarge))
            } else {
                Ok(file.read().await)
            }
        }
        None => Err(debug_message!("Error getting file path.").into()),
    }
}

pub async fn set_user_image(
    data: Vec<u8>,
    user_id: Uuid,
    db: Option<&Database>,
) -> Result<(), Error> {
    let data = data.clone();

    let data = match tokio::task::spawn_blocking(move || {
        let dyn_image =
            load_from_memory(data.as_slice()).map_err(|err| debug_message!("{}", err).into())?;

        match webp::Encoder::from_image(&dyn_image) {
            Ok(encoder) => Ok(encoder.encode(20.0).deref().to_vec()),
            Err(err) => Err(debug_message!("{}", err).into()),
        }
    })
    .await
    {
        Ok(Ok(data)) => Ok(data),
        Ok(Err(err)) => Err(err),
        Err(err) => Err(debug_message!("{}", err).into()),
    }?;

    match database::base::upload_file(format!("/{}/profile_picture.webp", user_id), data).await {
        Ok(_) => {}
        Err(err) => {
            return Err(err);
        }
    };

    if let Some(db) = db {
        database::settings::update_user(
            db,
            user_id,
            doc! {
                "profile_picture": true
            },
        )
        .await
    } else {
        Ok(())
    }
}

pub fn username_input<'a>(
    username: String,
    field_value: String,
) -> Element<'a, Message, Theme, Renderer> {
    Column::with_children(vec![
        Text::new("Username").size(20.0).into(),
        Row::with_children(vec![
            TextInput::new("Input username...", &*field_value.clone())
                .on_input(|value| SettingsMessage::UpdateUsernameField(value.clone()).into())
                .size(15.0)
                .into(),
            Space::with_width(Length::Fill).into(),
            if field_value == username {
                Button::new(Text::new("Update").size(15.0))
            } else {
                Button::new(Text::new("Update").size(15.0))
                    .on_press(SettingsMessage::UpdateUsername.into())
            }
            .into(),
        ])
        .spacing(5.0)
        .into(),
    ])
    .width(Length::Fill)
    .spacing(5.0)
    .into()
}

pub fn username_error<'a>() -> Element<'a, Message, Theme, Renderer> {
    Text::new(
        Error::AuthError(AuthError::RegisterBadCredentials {
            email: false,
            username: true,
            password: false,
        })
        .to_string(),
    )
    .style(theme::text::danger)
    .size(15.0)
    .into()
}

pub fn user_tag_input<'a>(
    user_tag: String,
    field_value: String,
) -> Element<'a, Message, Theme, Renderer> {
    Column::with_children(vec![
        Text::new("User Tag").size(20.0).into(),
        Row::with_children(vec![
            TextInput::new("Input user tag...", &*field_value.clone())
                .on_input(|value| SettingsMessage::UpdateUserTagField(value).into())
                .size(15.0)
                .into(),
            Space::with_width(Length::Fill).into(),
            if field_value == user_tag {
                Button::new(Text::new("Update").size(15.0))
            } else {
                Button::new(Text::new("Update").size(15.0))
                    .on_press(SettingsMessage::UpdateUserTag.into())
            }
            .into(),
        ])
        .spacing(5.0)
        .into(),
    ])
    .width(Length::Fill)
    .spacing(5.0)
    .into()
}

pub fn password_input<'a>(
    field_value: String,
    repeat_value: String,
) -> Element<'a, Message, Theme, Renderer> {
    Row::with_children(vec![
        Column::with_children(vec![
            Text::new("Password").size(20.0).into(),
            TextInput::new("Input password...", &*field_value.clone())
                .size(15.0)
                .on_input(|value| SettingsMessage::UpdatePasswordField(value.clone()).into())
                .secure(true)
                .into(),
            TextInput::new("Repeat password...", &*repeat_value.clone())
                .size(15.0)
                .on_input(|value| SettingsMessage::UpdatePasswordRepeatField(value).into())
                .secure(true)
                .into(),
        ])
        .spacing(5.0)
        .into(),
        Space::with_width(Length::Fill).into(),
        if field_value == repeat_value {
            Button::new(Text::new("Update").size(15.0))
                .on_press(SettingsMessage::UpdatePassword.into())
        } else {
            Button::new(Text::new("Update").size(15.0))
        }
        .into(),
    ])
    .align_items(Alignment::End)
    .width(Length::Fill)
    .spacing(5.0)
    .into()
}

pub fn password_error<'a>() -> Element<'a, Message, Theme, Renderer> {
    Text::new(
        Error::AuthError(AuthError::RegisterBadCredentials {
            email: false,
            username: false,
            password: true,
        })
        .to_string(),
    )
    .style(theme::text::danger)
    .size(15.0)
    .into()
}

pub fn profile_picture_input<'a>(
    image_handle: &Option<Handle>,
) -> Element<'a, Message, Theme, Renderer> {
    Row::with_children(vec![
        Text::new("Profile picture").size(20.0).into(),
        Space::with_width(Length::Fill).into(),
        Column::with_children(vec![
            if let Some(image_handle) = image_handle {
                Image::new(image_handle.clone())
                    .height(200.0)
                    .width(200.0)
                    .into()
            } else {
                WaitPanel::new("Loading...")
                    .width(200.0)
                    .height(200.0)
                    .into()
            },
            Button::new("Select image")
                .on_press(SettingsMessage::SelectImage.into())
                .into(),
        ])
        .align_items(Alignment::Center)
        .spacing(10.0)
        .into(),
    ])
    .align_items(Alignment::Center)
    .into()
}
