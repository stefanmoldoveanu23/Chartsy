use std::any::Any;
use std::io;
use std::ops::DerefMut;
use dropbox_sdk::default_client::{NoauthDefaultClient, UserAuthDefaultClient};
use dropbox_sdk::files;
use dropbox_sdk::files::DownloadArg;
use dropbox_sdk::oauth2::Authorization;
use iced::advanced::image::Handle;
use iced::{Alignment, Element, Length, Renderer, Command};
use iced::widget::{Column, Row, Scrollable, Image, Text, TextInput, Button};
use mongodb::bson::{doc, Uuid};
use crate::widgets::closeable::Closeable;
use crate::widgets::modal_stack::ModalStack;
use crate::widgets::post_summary::PostSummary;
use crate::{config, mongo};
use crate::errors::error::Error;
use crate::scene::{Action, Globals, Message, Scene, SceneOptions};
use crate::serde::Serialize;
use crate::theme::Theme;
use crate::widgets::rating::Rating;

use crate::scenes::data::posts::*;

/// The [messages](Action) that can be triggered on the [Posts] scene.
#[derive(Clone)]
enum PostsAction {
    /// Triggers when some posts are loaded to be displayed.
    LoadedPosts(Vec<Post>),

    /// Triggers when the given amount of images from the posts have been loaded.
    LoadedImage{ image: Vec<u8>, index: usize },

    /// Loads a batch of images.
    LoadBatch,

    /// Handles messages related to comments.
    CommentMessage(CommentMessage),

    /// Triggers when a [modal](ModalType) is toggled.
    ToggleModal(ModalType),

    /// Sets the rating of the given post.
    RatePost{ post_index: usize, rating: usize },

    /// Triggers when an error occurred.
    ErrorHandler(Error),
}

impl Action for PostsAction
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn get_name(&self) -> String {
        match self {
            PostsAction::LoadedPosts(_) => String::from("Loaded posts"),
            PostsAction::LoadedImage{ .. } => String::from("Loaded image"),
            PostsAction::LoadBatch => String::from("Load batch"),
            PostsAction::CommentMessage(_) => String::from("Loaded comments"),
            PostsAction::ToggleModal(_) => String::from("Toggle modal"),
            PostsAction::RatePost { .. } => String::from("Rate post"),
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

/// A scene that displays posts.
#[derive(Clone)]
pub struct Posts {
    /// The stack of modals.
    modals: ModalStack<ModalType>,
    
    /// The list of available posts.
    posts: Vec<Post>,

    /// The amount of posts to be shown
    batched: usize,
}

impl Posts {
    fn update_comment(&mut self, comment_message: &CommentMessage, globals: &mut Globals) -> Command<Message>
    {
        match comment_message {
            CommentMessage::Open { post, position } => {
                let (line, index) = position;

                let comment = self.posts[*post].get_comments()[*line][*index].clone();
                if let Some((parent_line, parent_index)) = comment.get_parent() {
                    self.posts[*post].get_comments_mut()[*parent_line][*parent_index].set_open_reply(*index);
                } else {
                    self.posts[*post].set_open_comment(*index);
                }

                if comment.replies_not_loaded() {
                    self.update_comment(
                        &CommentMessage::Load {
                            post: *post,
                            parent: Some((*line, *index))
                        },
                        globals
                    )
                } else {
                    Command::none()
                }
            }
            CommentMessage::Close {post, position} => {
                let mut position = if position.0 != 0 {
                    self.posts[*post].get_comments()[position.0][position.1].get_parent().clone()
                } else {
                    self.posts[*post].set_open_comment(None);
                    Some(*position)
                };

                while let Some((line, index)) = position {
                    let reply_line = self.posts[*post].get_comments()[line][index].get_replies().clone();
                    let reply_index = self.posts[*post].get_comments()[line][index].get_open_reply().clone();
                    position = reply_line.zip(reply_index);

                    self.posts[*post].get_comments_mut()[line][index].set_open_reply(None);
                }

                Command::none()
            }
            CommentMessage::UpdateInput {post, position, input} => {
                if let Some((line, index)) = position {
                    self.posts[*post].get_comments_mut()[*line][*index].set_reply_input(input.clone());
                } else {
                    self.posts[*post].set_comment_input(input.clone());
                }

                Command::none()
            }
            CommentMessage::Add { post, parent } => {
                let db = globals.get_db().unwrap();

                let comment = if let Some((line, index)) = parent {
                    let parent = &self.posts[*post].get_comments()[*line][*index];
                    Comment::new_reply(
                        Uuid::new(),
                        globals.get_user().unwrap(),
                        parent.get_reply_input().clone(),
                        parent.get_id().clone(),
                        (*line, *index)
                    )
                } else {
                    Comment::new_comment(
                        Uuid::new(),
                        globals.get_user().unwrap(),
                        self.posts[*post].get_comment_input().clone(),
                    )
                };

                let mut document = comment.serialize();
                if let Some((line, index)) = parent {
                    self.posts[*post].get_comments_mut()[*line][*index].set_reply_input("");

                    let line = self.posts[*post].get_comments()[*line][*index].get_replies().unwrap();
                    self.posts[*post].get_comments_mut()[line].push(comment);
                } else {
                    self.posts[*post].set_comment_input("");
                    self.posts[*post].get_comments_mut()[0].push(comment);

                    document.insert("post_id", self.posts[*post].get_id().clone());
                }

                Command::perform(
                    async move {
                        mongo::posts::create_comment(&db, &document).await
                    },
                    |result| {
                        match result {
                            Ok(_) => Message::None,
                            Err(err) => Message::Error(err)
                        }
                    }
                )
            }
            CommentMessage::Load { post, parent } => {
                let db = globals.get_db().unwrap();
                let parent = parent.clone();
                let post = post.clone();

                let filter = if let Some((line, index)) = parent {
                    doc! {
                        "reply_to": self.posts[post].get_comments()[line][index].get_id().clone()
                    }
                } else {
                    doc! {
                        "post_id": self.posts[post].get_id().clone()
                    }
                };

                Command::perform(
                    async move {
                        mongo::posts::get_comments(&db, filter).await
                    },
                    move |result| {
                        match result {
                            Ok(comments) => {
                                Message::DoAction(Box::new(PostsAction::CommentMessage(
                                    CommentMessage::Loaded {
                                        post,
                                        parent,
                                        comments
                                    }
                                )))
                            }
                            Err(err) => Message::Error(err)
                        }
                    }
                )
            }
            CommentMessage::Loaded { post, parent, comments } => {
                self.posts[*post].get_comments_mut().push(comments.clone());
                let new_line = self.posts[*post].get_comments().len() - 1;

                for comment in &mut self.posts[*post].get_comments_mut()[new_line] {
                    comment.set_parent(*parent);
                }

                if let Some((line, index)) = parent {
                    self.posts[*post].get_comments_mut()[*line][*index].set_replies(new_line);
                }

                Command::none()
            }
        }
    }
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
            posts: vec![],
            batched: 0,
        };

        if let Some(options) = options {
            options.apply_options(&mut posts);
        }

        let db = globals.get_db().unwrap();
        let user_id = globals.get_user().unwrap().get_id().clone();
        (
            posts,
            Command::perform(
                async move {
                    let mut posts =
                        match mongo::posts::get_recommendations(&db, user_id).await {
                            Ok(posts) => posts,
                            Err(err) => {
                                return Err(err);
                            }
                        };

                    let need = 100 - posts.len();
                    let uuids :Vec<Uuid>= posts.iter().map(|post: &Post| post.get_id().clone()).collect();

                    if posts.len() < 100 {
                        let mut posts_random =
                            match mongo::posts::get_random_posts(
                                &db,
                                need,
                                user_id,
                                uuids
                            ).await {
                                Ok(posts) => posts,
                                Err(err) => {
                                    return Err(err);
                                }
                            };

                        posts.append(&mut posts_random);
                    }

                    Ok(posts)
                },
                |result| {
                    match result {
                        Ok(posts) => Message::DoAction(Box::new(PostsAction::LoadedPosts(posts))),
                        Err(err) => Message::Error(err)
                    }
                }
            )
        )
    }

    fn get_title(&self) -> String {
        String::from("Posts")
    }

    fn update(&mut self, globals: &mut Globals, message: Box<dyn Action>) -> Command<Message> {
        let message = message.as_any().downcast_ref::<PostsAction>().expect("Panic downcasting to PostsAction");

        match message {
            PostsAction::LoadedPosts(posts) => {
                self.posts = posts.clone();
                let length = self.posts.len();

                if length > 0 {
                    self.update(globals, Box::new(PostsAction::LoadBatch))
                } else {
                    Command::none()
                }
            }
            PostsAction::LoadedImage { image, index } => {
                let post = &mut self.posts[*index];
                post.set_image(image.clone());

                Command::none()
            }
            PostsAction::LoadBatch => {
                let start = self.batched;
                let total = self.posts.len();

                self.batched += 10.min(total - start);

                let posts_data = self.posts[start..self.batched].iter().enumerate().map(
                    |(index, post)| (
                        index,
                        post.get_id().clone(),
                        post.get_user().get_id().clone(),
                    )
                ).collect::<Vec<(usize, Uuid, Uuid)>>();

                Command::batch(
                    posts_data.into_iter().map(
                        |(index, post_id, user_id)| {
                            Command::perform(
                                async move {
                                    let mut auth = Authorization::from_refresh_token(
                                        config::dropbox_id().into(),
                                        config::dropbox_refresh_token().into()
                                    );

                                    let _token = auth
                                        .obtain_access_token(NoauthDefaultClient::default())
                                        .unwrap();

                                    let client = UserAuthDefaultClient::new(auth.clone());
                                    let mut data = vec![];

                                    match files::download(
                                        &client,
                                        &DownloadArg::new(format!("/{}/{}.webp", user_id, post_id)),
                                        None,
                                        None
                                    ) {
                                        Ok(Ok(result)) => {
                                            let mut read = result.body.unwrap();

                                            let _ = io::copy(read.deref_mut(), &mut data).unwrap();
                                        },
                                        _ => {}
                                    }

                                    data
                                },
                                move |data| Message::DoAction(
                                    Box::new(PostsAction::LoadedImage {
                                        image: data,
                                        index,
                                    })
                                )
                            )
                        }
                    )
                )
            }
            PostsAction::CommentMessage(message) => {
                self.update_comment(message, globals)
            }
            PostsAction::ToggleModal(modal) => {
                self.modals.toggle_modal(modal.clone());

                match modal {
                    ModalType::ShowingPost(post) => {
                        if self.posts[*post].get_comments().len() == 0 {
                            self.update_comment(
                                &CommentMessage::Load {
                                    post: *post,
                                    parent: None
                                },
                                globals
                            )
                        } else {
                            Command::none()
                        }
                    }
                    _ => Command::none()
                }
            }
            PostsAction::RatePost { post_index, rating } => {
                let post :Option<&mut Post>= self.posts.get_mut(*post_index);
                if let Some(post) = post {
                    let rating = rating.clone();
                    post.set_rating(rating);

                    let post_id = *post.get_id();
                    let user_id = globals.get_user().unwrap().get_id();
                    let db = globals.get_db().unwrap();

                    if rating > 0 {
                        Command::perform(
                            async move {
                                mongo::posts::update_rating(
                                    &db,
                                    post_id,
                                    user_id,
                                    rating as i32
                                ).await
                            },
                            |result| {
                                match result {
                                    Ok(_) => Message::None,
                                    Err(err) => Message::Error(err)
                                }
                            }
                        )
                    } else {
                        Command::perform(
                            async move {
                                mongo::posts::delete_rating(
                                    &db,
                                    post_id,
                                    user_id
                                ).await
                            },
                            |result| {
                                match result {
                                    Ok(_) => Message::None,
                                    Err(err) => Message::Error(err)
                                }
                            }
                        )
                    }
                } else {
                    Command::none()
                }
            }
            PostsAction::ErrorHandler(_) => { Command::none() }
        }
    }

    fn view(&self, _globals: &Globals) -> Element<'_, Message, Theme, Renderer> {
        let post_summaries :Element<Message, Theme, Renderer>= Scrollable::new(
            Column::with_children(
                self.posts.iter().zip(0..self.batched).map(
                    |(post, index)| {
                        PostSummary::<Message, Theme, Renderer>::new(
                            Column::with_children(vec![
                                Text::new(post.get_user().get_username()).size(20.0).into(),
                                Text::new(post.get_description().clone()).into()
                            ]),
                            Image::new(
                                Handle::from_memory(post.get_image().clone())
                            ).width(Length::Shrink)
                        )
                            .padding(40)
                            .on_click_image(Message::DoAction(Box::new(PostsAction::ToggleModal(
                                ModalType::ShowingImage(post.get_image().clone())
                            ))))
                            .on_click_data(Message::DoAction(Box::new(PostsAction::ToggleModal(
                                ModalType::ShowingPost(index)
                            ))))
                            .into()
                    }
                ).collect::<Vec<Element<Message, Theme, Renderer>>>()
            )
                .width(Length::Fill)
                .align_items(Alignment::Center)
                .spacing(50)
        )
            .on_scroll(|viewport| {
                if viewport.relative_offset().y == 1.0 && self.batched != self.posts.len() {
                    Message::DoAction(Box::new(PostsAction::LoadBatch))
                } else {
                    Message::None
                }
            })
            .width(Length::Fill)
            .into();

        let modal_generator = |modal_type: ModalType| {
            match modal_type {
                ModalType::ShowingImage(data) => {
                    Closeable::new(Image::new(
                        Handle::from_memory(data.clone())
                    ).width(Length::Shrink))
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .on_close(
                            Message::DoAction(Box::new(PostsAction::ToggleModal(
                                ModalType::ShowingImage(data.clone())
                            ))),
                            40.0
                        )
                        .style(crate::theme::closeable::Closeable::SpotLight)
                        .into()
                }
                ModalType::ShowingPost(post_index) => {
                    let post = self.posts.get(post_index).unwrap();

                    let mut comment_chain = Column::with_children(
                        vec![
                            Row::with_children(
                                vec![
                                    TextInput::new("Write comment here...", &*post.get_comment_input())
                                        .width(Length::Fill)
                                        .on_input(move |value| Message::DoAction(Box::new(
                                            PostsAction::CommentMessage(CommentMessage::UpdateInput {
                                                post: post_index,
                                                position: None,
                                                input: value,
                                            })
                                        )))
                                        .into(),
                                    Button::new("Add comment")
                                        .on_press(Message::DoAction(Box::new(
                                            PostsAction::CommentMessage(CommentMessage::Add {
                                                post: post_index,
                                                parent: None,
                                            })
                                        )))
                                        .into()
                                ]
                            )
                                .into()
                        ]
                    );

                    let mut position = if let Some(index) = post.get_open_comment() {
                        Ok((0usize, *index))
                    } else {
                        Err(0usize)
                    };

                    let mut done = false;
                    while !done {
                        comment_chain = comment_chain.push(
                            match position {
                                Ok((line, index)) => {
                                    position = if let Some(reply_index) = post.get_comments()[line][index].get_open_reply() {
                                        Ok((post.get_comments()[line][index].get_replies().unwrap(), *reply_index))
                                    } else {
                                        Err(post.get_comments()[line][index].get_replies().unwrap_or(post.get_comments().len()))
                                    };

                                    Into::<Element<Message, Theme, Renderer>>::into(
                                        Closeable::new(
                                            Column::with_children(vec![
                                                Text::new(post.get_comments()[line][index].get_user().get_username().clone())
                                                    .size(17.0)
                                                    .into(),
                                                Text::new(post.get_comments()[line][index].get_content().clone())
                                                    .into(),
                                                Row::with_children(vec![
                                                    TextInput::new(
                                                        "Write reply here...",
                                                        &*post.get_comments()[line][index].get_reply_input()
                                                    )
                                                        .on_input(move |value| Message::DoAction(Box::new(
                                                            PostsAction::CommentMessage(CommentMessage::UpdateInput {
                                                                post: post_index,
                                                                position: Some((line, index)),
                                                                input: value.clone(),
                                                            })
                                                        )))
                                                        .into(),
                                                    Button::new("Add reply")
                                                        .on_press(Message::DoAction(Box::new(
                                                            PostsAction::CommentMessage(CommentMessage::Add {
                                                                post: post_index,
                                                                parent: Some((line, index))
                                                            })
                                                        )))
                                                        .into()
                                                ])
                                                    .into()
                                            ])
                                        )
                                            .on_close(
                                                Message::DoAction(Box::new(PostsAction::CommentMessage(
                                                    CommentMessage::Close {
                                                        post: post_index,
                                                        position: (line, index),
                                                    }
                                                ))),
                                                20.0
                                            )
                                    )
                                }
                                Err(line) => {
                                    done = true;

                                    if line >= post.get_comments().len() {
                                        Text::new("Loading").into()
                                    } else {
                                        Column::with_children(
                                            post.get_comments()[line].iter().zip(0..post.get_comments()[line].len()).map(
                                                |(comment, index)| Button::new(Column::with_children(vec![
                                                    Text::new(comment.get_user().get_username().clone())
                                                        .size(17.0)
                                                        .into(),
                                                    Text::new(comment.get_content().clone())
                                                        .into()
                                                ]))
                                                    .style(crate::theme::button::Button::Transparent)
                                                    .on_press(Message::DoAction(Box::new(
                                                        PostsAction::CommentMessage(CommentMessage::Open {
                                                            post: post_index,
                                                            position: (line, index)
                                                        })
                                                    )))
                                                    .into()
                                            ).collect::<Vec<Element<Message, Theme, Renderer>>>()
                                        )
                                            .into()
                                    }
                                }
                            }
                        );
                    }

                    Row::with_children(
                        vec![
                            Closeable::new(Image::new(
                                Handle::from_memory(post.get_image().clone())
                            ).width(Length::Shrink))
                                .width(Length::FillPortion(3))
                                .height(Length::Fill)
                                .style(crate::theme::closeable::Closeable::SpotLight)
                                .on_click(Message::DoAction(Box::new(PostsAction::ToggleModal(ModalType::ShowingImage(post.get_image().clone())))))
                                .into(),
                            Closeable::new(
                                Column::with_children(vec![
                                    Text::new(post.get_user().get_username())
                                        .size(20.0)
                                        .into(),
                                    Text::new(post.get_description().clone())
                                        .into(),
                                    Rating::new()
                                        .on_rate(move |value| Message::DoAction(Box::new(
                                            PostsAction::RatePost {
                                                post_index: post_index.clone(),
                                                rating: value
                                            }
                                        )))
                                        .on_unrate(Message::DoAction(Box::new(
                                            PostsAction::RatePost {
                                                post_index,
                                                rating: 0
                                            }
                                        )))
                                        .value(*post.get_rating())
                                        .into(),
                                    comment_chain.into()
                                ])
                            )
                                .width(Length::FillPortion(1))
                                .height(Length::Fill)
                                .horizontal_alignment(Alignment::Start)
                                .vertical_alignment(Alignment::Start)
                                .padding([30.0, 0.0, 0.0, 10.0])
                                .style(crate::theme::closeable::Closeable::Default)
                                .on_close(
                                    Message::DoAction(Box::new(PostsAction::ToggleModal(ModalType::ShowingPost(post_index)))),
                                    40.0
                                )
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