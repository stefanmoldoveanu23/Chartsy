use std::any::Any;
use iced::advanced::image::Handle;
use iced::{Alignment, Element, Length, Renderer, Command};
use iced::alignment::Horizontal;
use iced::widget::{Column, Row, Scrollable, Image, Text, TextInput, Button, Space, Tooltip, Container};
use iced::widget::tooltip::Position;
use lettre::message::{Attachment, MultiPart, SinglePart};
use mongodb::bson::Uuid;
use crate::widgets::closeable::Closeable;
use crate::widgets::modal_stack::ModalStack;
use crate::widgets::post_summary::PostSummary;
use crate::{config, database};
use crate::errors::debug::DebugError;
use crate::errors::error::Error;
use crate::icons::{ICON, Icon};
use crate::scene::{Action, Globals, Message, Scene, SceneOptions};
use crate::theme::Theme;
use crate::widgets::rating::Rating;

use crate::scenes::data::posts::*;
use crate::widgets::card::Card;

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

    /// Updates the post report input.
    UpdateReportInput(String),

    /// Submits a post report.
    SubmitReport(usize),

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
            PostsAction::UpdateReportInput(_) => String::from("Update report input"),
            PostsAction::SubmitReport(_) => String::from("Submit report"),
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
    
    recommended: PostList,

    /// The user input of a report.
    report_input: String,
}

impl Posts {
    fn update_comment(&mut self, comment_message: &CommentMessage, globals: &mut Globals) -> Command<Message>
    {
        match comment_message {
            CommentMessage::Open { post, position } => {
                let (line, index) = position;

                if self.recommended.open_comment(*post, *line, *index) {
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
                let (line, index) = position;

                self.recommended.close_comment(*post, *line, *index);

                Command::none()
            }
            CommentMessage::UpdateInput {post, position, input} => {
                self.recommended.update_input(*post, *position, input.clone());

                Command::none()
            }
            CommentMessage::Add { post, parent } => {
                let db = globals.get_db().unwrap();
                let user = globals.get_user().unwrap();

                let document = if let Some((line, index)) = parent {
                    self.recommended.add_reply(user, *post, *line, *index)
                } else {
                    self.recommended.add_comment(user, *post)
                };

                Command::perform(
                    async move {
                        database::posts::create_comment(&db, &document).await
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

                let filter = self.recommended.load_comments(post, parent);

                Command::perform(
                    async move {
                        database::posts::get_comments(&db, filter).await
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
                self.recommended.loaded_comments(*post, *parent, comments.clone());

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
            recommended: PostList::new(vec![]),
            report_input: String::from(""),
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
                        match database::posts::get_recommendations(&db, user_id).await {
                            Ok(posts) => posts,
                            Err(err) => {
                                return Err(err);
                            }
                        };

                    let need = 100 - posts.len();
                    let uuids :Vec<Uuid>= posts.iter().map(|post: &Post| post.get_id().clone()).collect();

                    if posts.len() < 100 {
                        let mut posts_random =
                            match database::posts::get_random_posts(
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
        let as_option: Option<&PostsAction> = message
            .as_any()
            .downcast_ref::<PostsAction>();
        let message = if let Some(message) = as_option {
            message
        } else {
            return Command::perform(async {}, move |()| Message::Error(
                Error::DebugError(DebugError::new(
                    format!("Message doesn't belong to posts scene: {}.", message.get_name())
                ))
            ))
        };

        match message {
            PostsAction::LoadedPosts(posts) => {
                self.recommended = PostList::new(posts.clone());
                let length = posts.len();

                if length > 0 {
                    self.update(globals, Box::new(PostsAction::LoadBatch))
                } else {
                    Command::none()
                }
            }
            PostsAction::LoadedImage { image, index } => {
                self.recommended.set_image(*index, image.clone());

                Command::none()
            }
            PostsAction::LoadBatch => {
                let posts_data = self.recommended.load_batch();

                Command::batch(
                    posts_data.into_iter().map(
                        |(index, post_id, user_id)| {
                            Command::perform(
                                async move {
                                    database::base::download_file(
                                        format!("/{}/{}.webp", user_id, post_id)
                                    ).await
                                },
                                move |data| {
                                    match data {
                                        Ok(data) => Message::DoAction(
                                            Box::new(PostsAction::LoadedImage {
                                                image: data,
                                                index,
                                            })
                                        ),
                                        Err(err) => Message::Error(err)
                                    }
                                }
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
                        if !self.recommended.has_loaded_comments(*post) {
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
                let user_id = globals.get_user().unwrap().get_id();
                let db = globals.get_db().unwrap();

                let (post_id, rating) = self.recommended.rate_post(*post_index, *rating);

                if let Some(rating) = rating {
                    Command::perform(
                        async move {
                            database::posts::update_rating(
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
                            database::posts::delete_rating(
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
            }
            PostsAction::UpdateReportInput(report_input) => {
                self.report_input = report_input.clone();

                Command::none()
            }
            PostsAction::SubmitReport(post_index) => {
                let post_index = post_index.clone();
                let post = self.recommended.get_post(post_index).unwrap();
                let report_description = self.report_input.clone();

                let message = lettre::Message::builder()
                    .from(format!("Chartsy <{}>", config::email_address()).parse().unwrap())
                    .to(format!(
                        "Stefan Moldoveanu <{}>",
                        config::admin_email_address()
                    ).parse().unwrap())
                    .subject("Anonymous user has submitted a report")
                    .multipart(
                        MultiPart::mixed()
                            .multipart(
                                MultiPart::related()
                                    .singlepart(
                                        SinglePart::html(String::from(
                                            format!("<p>A user has submitted a report regarding a post:</p>\
                                            <p>\"{}\"</p>\
                                            <p>Data regarding the post:</p>\
                                            <p>Username: \"{}\"</p>\
                                            <p>Post description: \"{}\"</p>\
                                            <p>Image:</p>\
                                            <div><img src=cid:post_image></div>",
                                                report_description,
                                                post.get_user().get_username().clone(),
                                                post.get_description().clone()
                                        )))
                                    )
                                    .singlepart(
                                        Attachment::new_inline(String::from("post_image"))
                                            .body(
                                                post.get_image().clone(),
                                                "image/*".parse().unwrap()
                                            )
                                    )
                            )
                            .singlepart(
                                Attachment::new(String::from("post_image.webp"))
                                    .body(
                                        post.get_image().clone(),
                                        "image/*".parse().unwrap()
                                    )
                            )
                    )
                    .unwrap();

                Command::batch(vec![
                    Command::perform(
                        async { },
                        move |()| Message::SendSmtpMail(message)
                    ),
                    Command::perform(
                        async { },
                        move |()| Message::DoAction(Box::new(PostsAction::ToggleModal(
                            ModalType::ShowingReport(post_index)
                        )))
                    )
                ])
            }
            PostsAction::ErrorHandler(_) => { Command::none() }
        }
    }

    fn view(&self, _globals: &Globals) -> Element<'_, Message, Theme, Renderer> {
        let post_summaries :Element<Message, Theme, Renderer>= Scrollable::new(
            Column::with_children(
                self.recommended.get_loaded_posts().into_iter().map(
                    |(post, index)| {
                        PostSummary::<Message, Theme, Renderer>::new(
                            Row::with_children(vec![
                                Column::with_children(vec![
                                    Text::new(post.get_user().get_username()).size(20.0).into(),
                                    Text::new(post.get_description().clone()).into()
                                ])
                                    .into(),
                                Space::with_width(Length::Fill).into(),
                                Column::with_children(vec![
                                    Tooltip::new(
                                        Button::new(Text::new(
                                            Icon::Report.to_string()
                                        ).font(ICON).style(crate::theme::text::Text::Error).size(30.0))
                                            .on_press(Message::DoAction(Box::new(
                                                PostsAction::ToggleModal(ModalType::ShowingReport(
                                                    index
                                                ))
                                            )))
                                            .padding(0.0)
                                            .style(crate::theme::button::Button::Transparent),
                                        Text::new("Report post"),
                                        Position::FollowCursor
                                    )
                                        .into(),
                                ])
                                    .into()
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
                if viewport.relative_offset().y == 1.0 && self.recommended.done_loading() {
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
                    let post = self.recommended.get_post(post_index).unwrap();

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
                ModalType::ShowingReport(post_index) => {
                    Closeable::new(
                        Card::new(
                            Text::new("Report post").size(20.0),
                            Column::with_children(vec![
                                TextInput::new(
                                    "Give a summary of the issue...",
                                    &*self.report_input.clone()
                                )
                                    .on_input(|value| Message::DoAction(Box::new(
                                        PostsAction::UpdateReportInput(value.clone())
                                    )))
                                    .into(),
                                Container::new(
                                    Button::new("Submit")
                                        .on_press(Message::DoAction(Box::new(
                                            PostsAction::SubmitReport(post_index)
                                        )))
                                )
                                    .width(Length::Fill)
                                    .align_x(Horizontal::Center)
                                    .into()
                            ])
                                .padding(20.0)
                                .spacing(30.0)
                        )
                            .width(300.0)
                    )
                        .on_close(
                            Message::DoAction(Box::new(PostsAction::ToggleModal(
                                ModalType::ShowingReport(post_index)
                            ))),
                            25.0
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

    fn clear(&self, _globals: &mut Globals) { }
}