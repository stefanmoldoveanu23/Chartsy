use std::sync::Arc;

use directories::ProjectDirs;
use iced::{
    advanced::widget::Text,
    alignment::{Horizontal, Vertical},
    widget::{Button, Column, Container, Row, Scrollable, Space},
    Alignment, Element, Length, Renderer, Size,
};
use image::{load_from_memory_with_format, ImageFormat};
use json::JsonValue;
use mongodb::bson::{Bson, Document, Uuid, UuidRepresentation};
use tokio::io;

use crate::{
    database, debug_message,
    scene::{Globals, Message},
    scenes::{
        auth::AuthOptions,
        data::{
            auth::{AuthTabIds, User},
            drawing::SaveMode,
            main::{MainTabIds, ModalType},
        },
        drawing::DrawingOptions,
        main::MainMessage,
        scenes::Scenes,
    },
    utils::{
        cache::PixelImage,
        errors::Error,
        icons::{Icon, ICON},
        theme::{self, Theme},
    },
    widgets::{card::Card, closeable::Closeable, Centered, Tabs},
};

/// Returns the ids of the drawings stored locally.
pub async fn get_drawings_offline() -> Result<Vec<(Uuid, String)>, Error> {
    let proj_dirs = ProjectDirs::from("", "CharMe", "Chartsy")
        .ok_or(debug_message!("Unable to find project directory.").into())?;

    let dir_path = proj_dirs.data_local_dir();
    tokio::fs::create_dir_all(dir_path)
        .await
        .map_err(|err| debug_message!("{}", err).into())?;

    let file_path = dir_path.join("drawings.json");
    let input = match tokio::fs::read_to_string(file_path.clone()).await {
        Ok(input) => input,
        Err(err) => {
            if err.kind() == io::ErrorKind::NotFound {
                tokio::fs::write(file_path, json::stringify(JsonValue::Array(vec![])))
                    .await
                    .map_err(|err| debug_message!("{}", err).into())?;
            }

            return Ok(vec![]);
        }
    };
    let mut list = vec![];

    let json = json::parse(&*input).map_err(|err| debug_message!("{}", err).into())?;
    if let JsonValue::Array(drawings) = json {
        for drawing in drawings {
            if let JsonValue::Object(drawing) = drawing {
                let name = if let Some(JsonValue::Short(name)) = drawing.get("name") {
                    name.to_string()
                } else if let Some(JsonValue::String(name)) = drawing.get("name") {
                    name.clone()
                } else {
                    String::from("New drawing")
                };

                if let Some(JsonValue::String(id)) = drawing.get("id") {
                    if let Ok(id) = Uuid::parse_str(id) {
                        list.push((id, name));
                    }
                }
            }
        }
    }

    Ok(list)
}

/// Returns the ids of the drawings stored in a database that belong to the currently
/// authenticated user.
pub fn get_drawings_online(drawings: &Vec<Document>) -> Vec<(Uuid, String)> {
    let mut list = vec![];
    for document in drawings {
        if let Some(Bson::Binary(bin)) = document.get("id") {
            if let Ok(uuid) = bin.to_uuid_with_representation(UuidRepresentation::Standard) {
                list.push((
                    uuid,
                    if let Ok(name) = document.get_str("name") {
                        String::from(name)
                    } else {
                        String::from("New drawing")
                    },
                ));
            }
        }
    }

    list
}

pub async fn load_preview_offline(id: Uuid) -> Result<Arc<PixelImage>, Error> {
    let proj_dirs = ProjectDirs::from("", "CharMe", "Chartsy")
        .ok_or(debug_message!("Could not open local project directory.").into())?;

    let dir_path = proj_dirs.data_local_dir();
    let file_path = dir_path.join(id.to_string()).join("data.webp");

    match tokio::fs::read(file_path).await {
        Ok(data) => load_from_memory_with_format(data.as_slice(), ImageFormat::WebP)
            .map(|data| Arc::new(data.into()))
            .map_err(|err| debug_message!("{}", err).into()),
        Err(err) => Err(debug_message!("{}", err).into()),
    }
}

pub async fn load_preview_online(ids: (Uuid, Uuid)) -> Result<Arc<PixelImage>, Error> {
    match database::base::download_file(format!("/{}/{}.webp", ids.1, ids.0)).await {
        Ok(data) => load_from_memory_with_format(data.as_slice(), ImageFormat::WebP)
            .map(|data| Arc::new(data.into()))
            .map_err(|err| debug_message!("{}", err).into()),
        Err(err) => Err(debug_message!("{}", err).into()),
    }
}

pub async fn delete_token_file() -> Result<(), Error> {
    let proj_dirs = ProjectDirs::from("", "CharMe", "Chartsy")
        .ok_or(debug_message!("Unable to find project directory.").into())?;
    let dir_path = proj_dirs.data_local_dir();
    let file_path = dir_path.join("token");

    tokio::fs::remove_file(file_path)
        .await
        .map_err(|err| debug_message!("{}", err).into())
}

pub fn auth_logged_in<'a>(user: &User) -> Element<'a, Message, Theme, Renderer> {
    let welcome_message = Text::new(format!("Welcome, {}!", user.get_username()))
        .vertical_alignment(Vertical::Bottom);
    let settings_button = Button::new("Settings")
        .padding(8)
        .on_press(Message::ChangeScene(Scenes::Settings(None)));
    let logout_button = Button::new("Log Out")
        .padding(8)
        .on_press(MainMessage::LogOut.into());

    Row::with_children(vec![
        Space::with_width(Length::Fill).into(),
        Row::with_children(vec![
            welcome_message.into(),
            settings_button.into(),
            logout_button.into(),
        ])
        .align_items(Alignment::Center)
        .width(Length::Shrink)
        .spacing(20)
        .into(),
    ])
    .into()
}

pub fn auth_logged_out<'a>() -> Element<'a, Message, Theme, Renderer> {
    let register_button = Button::new("Register")
        .padding(8)
        .on_press(Message::ChangeScene(Scenes::Auth(Some(AuthOptions::new(
            AuthTabIds::Register,
        )))));
    let login_button = Button::new("Log In")
        .padding(8)
        .on_press(Message::ChangeScene(Scenes::Auth(Some(AuthOptions::new(
            AuthTabIds::LogIn,
        )))));

    Row::with_children(vec![
        Space::with_width(Length::Fill).into(),
        Row::with_children(vec![register_button.into(), login_button.into()])
            .width(Length::Shrink)
            .spacing(20)
            .into(),
    ])
    .into()
}

pub fn main_column<'a>(user_logged_in: bool) -> Element<'a, Message, Theme, Renderer> {
    let start_drawing_button = Button::new(
        Text::new("Start new Drawing")
            .width(Length::Fill)
            .horizontal_alignment(Horizontal::Center),
    )
    .padding(8)
    .on_press(MainMessage::ToggleModal(ModalType::SelectingSaveMode).into());

    let continue_drawing_button = Button::new(
        Text::new("Continue drawing")
            .width(Length::Fill)
            .horizontal_alignment(Horizontal::Center),
    )
    .padding(8)
    .on_press(MainMessage::ToggleModal(ModalType::ShowingDrawings).into());

    let browse_posts_button = Button::new(
        Text::new("Browse posts")
            .width(Length::Fill)
            .horizontal_alignment(Horizontal::Center),
    )
    .padding(8)
    .on_press(Message::ChangeScene(Scenes::Posts(None)));

    let quit_button = Button::new(
        Text::new("Quit")
            .width(Length::Fill)
            .horizontal_alignment(Horizontal::Center),
    )
    .padding(8)
    .on_press(Message::Quit);

    Column::with_children(if user_logged_in {
        vec![
            start_drawing_button.width(Length::Fill).into(),
            continue_drawing_button.width(Length::Fill).into(),
            browse_posts_button.width(Length::Fill).into(),
            quit_button.width(Length::Fill).into(),
        ]
    } else {
        vec![
            start_drawing_button.width(Length::Fill).into(),
            continue_drawing_button.width(Length::Fill).into(),
            quit_button.width(Length::Fill).into(),
        ]
    })
    .spacing(20)
    .height(Length::FillPortion(3))
    .width(Length::Fixed(200.0))
    .align_items(Alignment::Center)
    .into()
}

pub fn display_drawing<'a>(
    id: Uuid,
    image: Element<'a, Message, Theme, Renderer>,
    name: String,
    save_mode: SaveMode,
) -> Element<'a, Message, Theme, Renderer> {
    Button::new(
        Row::<Message, Theme, Renderer>::with_children(vec![
            Text::new(name.clone())
                .width(Length::FillPortion(1))
                .horizontal_alignment(Horizontal::Center)
                .into(),
            Space::with_width(Length::FillPortion(1)).into(),
            image,
            Button::new(
                Text::new(Icon::Trash.to_string())
                    .font(ICON)
                    .style(theme::text::danger),
            )
            .style(iced::widget::button::text)
            .on_press(MainMessage::DeleteDrawing(id, save_mode).into())
            .into(),
        ])
        .align_items(Alignment::Center),
    )
    .style(iced::widget::button::secondary)
    .on_press(Message::ChangeScene(Scenes::Drawing(Some(
        DrawingOptions::new(Some(id), Some(name), Some(save_mode)),
    ))))
    .width(Length::Fill)
    .padding(10.0)
    .into()
}

pub fn drawings_tab<'a>(
    drawings: &Option<Vec<(Uuid, String)>>,
    save_mode: SaveMode,
    globals: &Globals,
) -> Element<'a, Message, Theme, Renderer> {
    Container::new(Scrollable::new(
        Column::<Message, Theme, Renderer>::with_children(match drawings {
            Some(drawings) => drawings
                .clone()
                .iter()
                .map(|(uuid, name)| {
                    display_drawing(
                        *uuid,
                        globals.get_cache().get_element(
                            *uuid,
                            Size::new(Length::FillPortion(1), Length::Fixed(150.0)),
                            Size::new(Length::Fixed(200.0), Length::Fixed(150.0)),
                            None,
                        ),
                        name.clone(),
                        save_mode,
                    )
                })
                .collect(),
            None => vec![],
        })
        .spacing(20.0)
        .padding([15.0, 15.0, 0.0, 15.0]),
    ))
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

pub fn drawings_tabs<'a>(
    offline_tab: Element<'a, Message, Theme, Renderer>,
    online_tab: Element<'a, Message, Theme, Renderer>,
    active_tab: MainTabIds,
) -> Element<'a, Message, Theme, Renderer> {
    Tabs::new_with_tabs(
        vec![
            (MainTabIds::Offline, String::from("Offline"), offline_tab),
            (MainTabIds::Online, String::from("Online"), online_tab),
        ],
        |tab| MainMessage::SelectTab(tab).into(),
    )
    .selected(active_tab)
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

pub fn display_drawings<'a>(
    title: Element<'a, Message, Theme, Renderer>,
    tabs: Element<'a, Message, Theme, Renderer>,
) -> Element<'a, Message, Theme, Renderer> {
    Centered::new(
        Closeable::<Message, Theme, Renderer>::new(Card::new(title, tabs).content_padding(0.0))
            .height(Length::FillPortion(5))
            .width(Length::Fill)
            .style(theme::closeable::Closeable::Transparent)
            .on_close(
                Into::<Message>::into(MainMessage::ToggleModal(ModalType::ShowingDrawings)),
                32.0,
            )
            .close_padding(8.0),
    )
    .height(5.0 / 7.0)
    .into()
}

pub fn create_drawing<'a>(
    offline_button: Element<'a, Message, Theme, Renderer>,
    online_button: Element<'a, Message, Theme, Renderer>,
) -> Element<'a, Message, Theme, Renderer> {
    Closeable::<Message, Theme, Renderer>::new(
        Card::new(
            Text::new("Create new drawing"),
            Column::with_children(vec![
                Space::with_height(Length::Fill).into(),
                Row::with_children(vec![
                    offline_button,
                    Space::with_width(Length::FillPortion(2)).into(),
                    online_button,
                ])
                .into(),
            ])
            .height(Length::Fixed(150.0)),
        )
        .width(Length::Fixed(300.0)),
    )
    .style(theme::closeable::Closeable::Transparent)
    .on_close(
        Into::<Message>::into(MainMessage::ToggleModal(ModalType::SelectingSaveMode)),
        25.0,
    )
    .close_padding(7.0)
    .into()
}
