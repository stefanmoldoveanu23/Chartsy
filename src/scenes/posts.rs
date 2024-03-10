use std::any::Any;
use std::io;
use std::ops::DerefMut;
use dropbox_sdk::default_client::{NoauthDefaultClient, UserAuthDefaultClient};
use dropbox_sdk::files;
use dropbox_sdk::files::DownloadArg;
use iced::advanced::svg::Handle;
use iced::{Alignment, Element, Length, Renderer};
use iced::widget::{Column, Row, Scrollable, Svg, Text};
use iced_runtime::Command;
use mongodb::bson::{Bson, doc, Uuid, UuidRepresentation};
use crate::widgets::closeable::Closeable;
use crate::widgets::modal_stack::ModalStack;
use crate::widgets::post_summary::PostSummary;
use crate::config::{DROPBOX_ID, DROPBOX_REFRESH_TOKEN};
use crate::errors::debug::DebugError;
use crate::errors::error::Error;
use crate::mongo::{MongoRequest, MongoRequestType, MongoResponse};
use crate::scene::{Action, Globals, Message, Scene, SceneOptions};
use crate::theme::Theme;

/// The [messages](Action) that can be triggered on the [Posts] scene.
#[derive(Clone)]
enum PostsAction {
    /// Triggers when some posts are loaded to be displayed.
    LoadedDrawings(Vec<Handle>),
    /// Triggers when a [modal](ModalType) is toggled.
    ToggleModal(ModalType),
    /// Triggers when an error occured.
    ErrorHandler(Error),
}

impl Action for PostsAction
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn get_name(&self) -> String {
        match self {
            PostsAction::LoadedDrawings(_) => String::from("Loaded drawings"),
            PostsAction::ToggleModal(_) => String::from("Toggle modal"),
            PostsAction::ErrorHandler(_) => String::from("Error handler"),
        }
    }

    fn boxed_clone(&self) -> Box<dyn Action + 'static> {
        Box::new((*self).clone())
    }
}

impl Into<Box<dyn Action + 'static>> for Box<PostsAction>
{
    fn into(self) -> Box<dyn Action + 'static> {
        Box::new(*self)
    }
}

/// The types a modal can have on the [Posts] scene.
#[derive(Clone, Eq)]
enum ModalType {
    /// Modal for displaying an image in the center of the screen.
    ShowingImage(Handle),
    /// Modal for opening a post.
    ShowingPost(Handle),
}

impl ModalType {
    /// Checks if its value is [ShowingImage](ModalType::ShowingImage).
    fn is_showing_image(&self) -> bool {
        match self {
            ModalType::ShowingImage(_) => true,
            _ => false,
        }
    }

    /// Checks if its value is [ShowingPost](ModalType::ShowingPost).
    fn is_showing_post(&self) -> bool {
        match self {
            ModalType::ShowingPost(_) => true,
            _ => false
        }
    }
}

impl PartialEq for ModalType {
    fn eq(&self, other: &Self) -> bool {
        match self {
            ModalType::ShowingImage(_) => {
                other.is_showing_image()
            }
            ModalType::ShowingPost(_) => {
                other.is_showing_post()
            }
        }
    }
}

/// A scene that displays posts.
#[derive(Clone)]
pub struct Posts {
    /// The stack of modals.
    modals: ModalStack<ModalType>,
    /// The list of available posts.
    drawings: Option<Vec<Handle>>,
}

/// The [Posts] scene does not have any optional initialization values.
#[derive(Debug, Clone, Copy)]
pub struct PostsOptions {}

impl SceneOptions<Posts> for PostsOptions {
    fn apply_options(&self, _scene: &mut Posts) { }

    fn boxed_clone(&self) -> Box<dyn SceneOptions<Posts>> {
        Box::new((*self).clone())
    }
}

impl Scene for Posts {
    fn new(
        options: Option<Box<dyn SceneOptions<Self>>>,
        globals: &mut Globals
    ) -> (Self, Command<Message>)
    where
        Self: Sized,
    {
        let mut posts = Posts {
            modals: ModalStack::new(),
            drawings: None,
        };

        if let Some(options) = options {
            options.apply_options(&mut posts);
        }

        let db = globals.get_db().unwrap();
        (
            posts,
            Command::perform(
                async move {
                    let posts = match MongoRequest::send_requests(
                        db,
                        vec![
                            MongoRequest::new(
                                "posts".into(),
                                MongoRequestType::Get(
                                    doc! { }
                                )
                            )
                        ]
                    ).await {
                        Ok(results) => {
                            if let Some(MongoResponse::Get(documents)) = results.get(0) {
                                documents.clone()
                            } else {
                                return Err(Message::Error(Error::DebugError(DebugError::new("Mongo response type error when getting posts".into()))))
                            }
                        },
                        Err(message) => {
                            return Err(message);
                        }
                    };

                    let mut auth = dropbox_sdk::oauth2::Authorization::from_refresh_token(
                        DROPBOX_ID.into(),
                        DROPBOX_REFRESH_TOKEN.into()
                    );

                    let _token = auth
                        .obtain_access_token(NoauthDefaultClient::default())
                        .unwrap();
                    let client = UserAuthDefaultClient::new(auth);

                    let posts :Vec<Handle>= posts.iter().filter_map(
                        |post| {
                            let mut user_id = Uuid::default();
                            let mut drawing_id = Uuid::default();

                            if let Some(Bson::Binary(bin)) = post.get("user_id") {
                                user_id = bin.to_uuid_with_representation(UuidRepresentation::Standard).unwrap();
                            }
                            if let Some(Bson::Binary(bin)) = post.get("drawing_id") {
                                drawing_id = bin.to_uuid_with_representation(UuidRepresentation::Standard).unwrap();
                            }

                            if user_id != Uuid::default() && drawing_id != Uuid::default() {
                                match files::download(
                                    &client,
                                    &DownloadArg::new(format!("/{}/{}.svg", user_id, drawing_id)),
                                    None,
                                    None
                                ) {
                                    Ok(Ok(result)) => {
                                        let mut read = result.body.unwrap();
                                        let mut data = vec![];

                                        let _ = io::copy(read.deref_mut(), &mut data).unwrap();

                                        Some(Handle::from_memory(data))
                                    },
                                    _ => None
                                }
                            } else {
                                None
                            }
                        }
                    ).collect();

                    Ok(posts)
                },
                |handles| {
                    Message::DoAction(Box::new(PostsAction::LoadedDrawings(handles.unwrap())))
                }
            )
        )
    }

    fn get_title(&self) -> String {
        String::from("Posts")
    }

    fn update(&mut self, _globals: &mut Globals, message: Box<dyn Action>) -> Command<Message> {
        let message = message.as_any().downcast_ref::<PostsAction>().expect("Panic downcasting to PostsAction");

        match message {
            PostsAction::LoadedDrawings(handles) => {
                self.drawings = Some(handles.clone());
            }
            PostsAction::ToggleModal(modal) => {
                self.modals.toggle_modal(modal.clone());
            }
            PostsAction::ErrorHandler(_) => { }
        }

        Command::none()
    }

    fn view(&self, _globals: &Globals) -> Element<'_, Message, Theme, Renderer> {
        let post_summaries :Element<Message, Theme, Renderer>= Scrollable::new(
            Column::with_children(
                self.drawings.clone().map_or(
                    vec![], |handles| handles.iter().map(|handle| {
                        PostSummary::<Message, Theme, Renderer>::new(Svg::<Theme>::new(handle.clone()).width(Length::Shrink))
                            .padding(40)
                            .on_click_image(Message::DoAction(Box::new(PostsAction::ToggleModal(ModalType::ShowingImage(handle.clone())))))
                            .on_click_data(Message::DoAction(Box::new(PostsAction::ToggleModal(ModalType::ShowingPost(handle.clone())))))
                            .into()
                    }).collect()
                )
            )
                .width(Length::Fill)
                .align_items(Alignment::Center)
                .spacing(50)
        )
            .width(Length::Fill)
            .into();

        let modal_generator = |modal_type: ModalType| {
            match modal_type {
                ModalType::ShowingImage(handle) => {
                    Closeable::new(Svg::new(handle.clone()).width(Length::Shrink))
                        .on_close(Message::DoAction(Box::new(PostsAction::ToggleModal(ModalType::ShowingImage(handle.clone())))))
                        .style(crate::theme::closeable::Closeable::SpotLight)
                        .into()
                }
                ModalType::ShowingPost(handle) => {
                    Row::with_children(
                        vec![
                            Closeable::new(Svg::new(handle.clone()).width(Length::Shrink))
                                .width(Length::FillPortion(3))
                                .height(Length::Fill)
                                .style(crate::theme::closeable::Closeable::SpotLight)
                                .on_click(Message::DoAction(Box::new(PostsAction::ToggleModal(ModalType::ShowingImage(handle.clone())))))
                                .into(),
                            Closeable::new(Text::new("Hello"))
                                .width(Length::FillPortion(1))
                                .height(Length::Fill)
                                .style(crate::theme::closeable::Closeable::Default)
                                .on_close(Message::DoAction(Box::new(PostsAction::ToggleModal(ModalType::ShowingPost(handle.clone())))))
                                .into()
                        ]
                    )
                        .into()
                }
            }
        };

        self.modals.get_modal(post_summaries.into(), modal_generator)
    }

    fn get_error_handler(&self, error: Error) -> Box<dyn Action> {
        Box::new(PostsAction::ErrorHandler(error))
    }

    fn clear(&self) { }
}