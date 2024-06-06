use crate::debug_message;
use crate::scene::{Globals, Message, Scene, SceneMessage};
use crate::scenes::data::auth::Role;
use crate::scenes::data::auth::User;
use crate::scenes::data::drawing::Tag;
use crate::scenes::services;
use crate::utils::errors::Error;
use crate::utils::icons::{Icon, ICON};
use crate::utils::theme::{self, Theme};
use crate::widgets::{
    Card, Close, Closeable, ComboBox, Grid, ModalStack, PostSummary, Rating, Tabs,
};
use crate::{config, database, LOADING_IMAGE};
use iced::advanced::image::Handle;
use iced::alignment::Horizontal;
use iced::widget::tooltip::Position;
use iced::widget::{
    Button, Column, Container, Image, Row, Scrollable, Space, Text, TextInput, Tooltip,
};
use iced::{Alignment, Command, Element, Length, Renderer};
use image::{load_from_memory_with_format, ExtendedColorType, ImageFormat};
use lettre::message::{Attachment, MultiPart, SinglePart};
use moka::future::Cache;
use mongodb::bson::Uuid;
use mongodb::Database;
use std::any::Any;
use std::collections::{HashMap, HashSet};
use std::io::Cursor;
use std::sync::Arc;

use crate::scenes::data::posts::*;

use super::scenes::Scenes;

/// The [messages](SceneMessage) that can be triggered on the [Posts] scene.
#[derive(Clone)]
pub enum PostsMessage {
    /// Loads posts for the active tab.
    LoadPosts,

    /// Triggers when some posts are loaded to be displayed.
    LoadedPosts(Vec<Post>, PostTabs),

    /// Triggers when the given amount of images from the posts have been loaded.
    LoadedImage { image: Arc<PixelImage>, id: Uuid },

    /// Loads a batch of images.
    LoadBatch(PostTabs),

    /// Handles messages related to comments.
    CommentMessage(CommentMessage),

    /// Triggers when a [modal](ModalType) is toggled.
    ToggleModal(ModalType),

    /// Sets the rating of the given post.
    RatePost { post_index: usize, rating: usize },

    /// Triggered when all tags have been loaded.
    LoadedTags(Vec<Tag>),

    /// Updates the filter tag input.
    UpdateFilterInput(String),

    /// Adds a new tag to the filters.
    AddTag(Tag),

    /// Removes a tag from the filters.
    RemoveTag(Tag),

    /// Opens a users' profile.
    OpenProfile(User),

    /// Update user tag input.
    UpdateUserTagInput(String),

    /// Get user by tag.
    GetUserByTag,

    /// Deletes a post.
    DeletePost(Uuid),

    /// Updates the post report input.
    UpdateReportInput(String),

    /// Submits a post report.
    SubmitReport(usize),

    /// Selects a tab.
    SelectTab(PostTabs),

    /// Triggers when an error occurred.
    ErrorHandler(Error),
}

impl Into<Message> for PostsMessage {
    fn into(self) -> Message {
        Message::DoAction(Box::new(self))
    }
}

impl SceneMessage for PostsMessage {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn get_name(&self) -> String {
        match self {
            Self::LoadPosts => String::from("Load posts"),
            Self::LoadedPosts(_, _) => String::from("Loaded posts"),
            Self::LoadedImage { .. } => String::from("Loaded image"),
            Self::LoadBatch(_) => String::from("Load batch"),
            Self::CommentMessage(_) => String::from("Loaded comments"),
            Self::ToggleModal(_) => String::from("Toggle modal"),
            Self::RatePost { .. } => String::from("Rate post"),
            Self::LoadedTags(_) => String::from("Loaded tags"),
            Self::UpdateFilterInput(_) => String::from("Update filter input"),
            Self::AddTag(_) => String::from("Add tag"),
            Self::RemoveTag(_) => String::from("Remove tag"),
            Self::OpenProfile(_) => String::from("Open profile"),
            Self::UpdateUserTagInput(_) => String::from("Update user tag input"),
            Self::GetUserByTag => String::from("Get user by tag"),
            Self::DeletePost(_) => String::from("Delete a post"),
            Self::UpdateReportInput(_) => String::from("Update report input"),
            Self::SubmitReport(_) => String::from("Submit report"),
            Self::SelectTab(_) => String::from("Select tab"),
            Self::ErrorHandler(_) => String::from("Error handler"),
        }
    }

    fn boxed_clone(&self) -> Box<dyn SceneMessage + 'static> {
        Box::new((*self).clone())
    }
}

impl Into<Box<dyn SceneMessage + 'static>> for Box<PostsMessage> {
    fn into(self) -> Box<dyn SceneMessage + 'static> {
        Box::new(*self)
    }
}

/// A scene that displays posts.
#[derive(Clone)]
pub struct Posts {
    /// The stack of modals.
    modals: ModalStack<ModalType>,

    /// Tab of recommended posts.
    recommended: PostList,

    /// Tab of filtered posts.
    filtered: PostList,

    /// List of chosen filter tags.
    tags: HashSet<Tag>,

    /// List of all filter tags.
    all_tags: HashSet<Tag>,

    /// Value of filter tag input.
    filter_input: String,

    /// Tab of user profile.
    profile: PostList,

    /// The user currently being looked up.
    user_profile: User,

    /// The user tag input.
    user_tag_input: String,

    /// All necessary images, stored in a hashmap for avoiding storing the same image twice.
    images: HashMap<Uuid, Arc<PixelImage>>,

    /// Currently active tab.
    active_tab: PostTabs,

    /// The user input of a report.
    report_input: String,

    /// User error.
    error: Option<Error>,
}

impl Posts {
    /// Get an image handle or resort to the default.
    fn get_handle(&self, image_id: Uuid) -> Handle {
        match self.images.get(&image_id) {
            Some(image) => {
                let data = image.get_data().clone();

                Handle::from_rgba(image.get_width(), image.get_height(), data)
            }
            None => Handle::from_bytes(LOADING_IMAGE),
        }
    }

    /// Get an image if it is not in cache.
    fn get_image(
        image_id: Uuid,
        image_path: String,
        cache: &Cache<Uuid, Arc<PixelImage>>,
    ) -> Command<Message> {
        let cache = cache.clone();

        Command::perform(
            async move {
                cache
                    .try_get_with(image_id, async move {
                        match database::base::download_file(image_path).await {
                            Ok(data) => {
                                match load_from_memory_with_format(
                                    data.as_slice(),
                                    ImageFormat::WebP,
                                ) {
                                    Ok(data) => Ok(Arc::new(data.into())),
                                    Err(err) => {
                                        return Err(debug_message!("{}", err).into());
                                    }
                                }
                            }
                            Err(err) => {
                                return Err(debug_message!("{}", err)).into();
                            }
                        }
                    })
                    .await
            },
            move |result| match result {
                Ok(data) => PostsMessage::LoadedImage {
                    image: data,
                    id: image_id,
                }
                .into(),
                Err(err) => Message::Error(err.as_ref().clone().into()),
            },
        )
    }

    /// Creates a command that returns a list of recommended posts.
    fn gen_recommended(db: Database, user_id: Uuid) -> Command<Message> {
        Command::perform(
            async move {
                let mut posts = match database::posts::get_recommendations(&db, user_id).await {
                    Ok(posts) => posts,
                    Err(err) => {
                        return Err(err);
                    }
                };

                let need = 100 - posts.len();
                let uuids: Vec<Uuid> = posts
                    .iter()
                    .map(|post: &Post| post.get_id().clone())
                    .collect();

                if posts.len() < 100 {
                    let mut posts_random =
                        match database::posts::get_random_posts(&db, need, user_id, uuids).await {
                            Ok(posts) => posts,
                            Err(err) => {
                                return Err(err);
                            }
                        };

                    posts.append(&mut posts_random);
                }

                Ok(posts)
            },
            |result| match result {
                Ok(posts) => PostsMessage::LoadedPosts(posts, PostTabs::Recommended).into(),
                Err(err) => Message::Error(err),
            },
        )
    }

    /// Creates a command that returns the list of posts that has all tags from the filter.
    fn gen_filtered(db: Database, user_id: Uuid, tags: Vec<String>) -> Command<Message> {
        Command::perform(
            async move { database::posts::get_filtered(&db, user_id, tags).await },
            |result| match result {
                Ok(posts) => PostsMessage::LoadedPosts(posts, PostTabs::Filtered).into(),
                Err(err) => Message::Error(err),
            },
        )
    }

    /// Creates a command that returns the list of posts on the given users profile.
    fn gen_profile(
        db: Database,
        user_id: Uuid,
        profile_picture_path: String,
        cache: &Cache<Uuid, Arc<PixelImage>>,
    ) -> Command<Message> {
        Command::batch(vec![
            Command::perform(
                async move { database::posts::get_user_posts(&db, user_id).await },
                |result| match result {
                    Ok(posts) => PostsMessage::LoadedPosts(posts, PostTabs::Profile).into(),
                    Err(err) => Message::Error(err),
                },
            ),
            Self::get_image(user_id, profile_picture_path, cache),
        ])
    }

    /// Applies the update corresponding the given message.
    fn update_comment(
        &mut self,
        comment_message: &CommentMessage,
        globals: &mut Globals,
    ) -> Command<Message> {
        match comment_message {
            CommentMessage::Open { post, position } => {
                let (line, index) = position;

                if self.get_active_tab_mut().open_comment(*post, *line, *index) {
                    self.update_comment(
                        &CommentMessage::Load {
                            post: *post,
                            parent: Some((*line, *index)),
                        },
                        globals,
                    )
                } else {
                    Command::none()
                }
            }
            CommentMessage::Close { post, position } => {
                let (line, index) = position;

                self.get_active_tab_mut()
                    .close_comment(*post, *line, *index);

                Command::none()
            }
            CommentMessage::UpdateInput {
                post,
                position,
                input,
            } => {
                self.get_active_tab_mut()
                    .update_input(*post, *position, input.clone());

                Command::none()
            }
            CommentMessage::Add { post, parent } => {
                let db = globals.get_db().unwrap();
                let user = globals.get_user().unwrap().clone();

                let document = if let Some((line, index)) = parent {
                    self.get_active_tab_mut()
                        .add_reply(user, *post, *line, *index)
                } else {
                    self.get_active_tab_mut().add_comment(user, *post)
                };

                Command::perform(
                    async move { database::posts::create_comment(&db, &document).await },
                    |result| match result {
                        Ok(_) => Message::None,
                        Err(err) => Message::Error(err),
                    },
                )
            }
            CommentMessage::Load { post, parent } => {
                let db = globals.get_db().unwrap();
                let parent = parent.clone();
                let post = post.clone();

                let active_tab = self.active_tab;
                let filter = self.get_tab_mut(active_tab).load_comments(post, parent);

                Command::perform(
                    async move { database::posts::get_comments(&db, filter).await },
                    move |result| match result {
                        Ok(comments) => CommentMessage::Loaded {
                            post,
                            parent,
                            comments,
                            tab: active_tab,
                        }
                        .into(),
                        Err(err) => Message::Error(err),
                    },
                )
            }
            CommentMessage::Loaded {
                post,
                parent,
                comments,
                tab,
            } => {
                self.get_tab_mut(*tab)
                    .loaded_comments(*post, *parent, comments.clone());

                Command::none()
            }
        }
    }

    /// Generates the visible list of posts.
    pub fn gen_post_list(
        &self,
        tab: PostTabs,
        globals: &Globals,
    ) -> Container<'_, Message, Theme, Renderer> {
        let user = globals.get_user().unwrap();
        let user_id = user.get_id();
        let user_role = user.get_role();

        Container::new(
            Scrollable::new(
                Column::with_children(
                    self.get_tab(tab)
                        .get_loaded_posts()
                        .into_iter()
                        .map(|(post, index)| {
                            PostSummary::<Message, Theme, Renderer>::new(
                                Row::with_children(vec![
                                    Tooltip::new(
                                        Button::new(
                                            Image::new(self.get_handle(post.get_user().get_id()))
                                                .width(50.0)
                                                .height(50.0),
                                        )
                                        .on_press(
                                            PostsMessage::OpenProfile(post.get_user().clone())
                                                .into(),
                                        )
                                        .style(iced::widget::button::text),
                                        Text::new(format!(
                                            "{}'s profile",
                                            post.get_user().get_user_tag()
                                        )),
                                        Position::FollowCursor,
                                    )
                                    .into(),
                                    Column::with_children(vec![
                                        Tooltip::new(
                                            Button::new(
                                                Text::new(format!(
                                                    "@{}",
                                                    post.get_user().get_user_tag()
                                                ))
                                                .size(15.0)
                                                .style(theme::text::gray),
                                            )
                                            .style(iced::widget::button::text)
                                            .on_press(
                                                PostsMessage::OpenProfile(post.get_user().clone())
                                                    .into(),
                                            ),
                                            Text::new(format!(
                                                "{}'s profile",
                                                post.get_user().get_user_tag()
                                            )),
                                            Position::FollowCursor,
                                        )
                                        .into(),
                                        Text::new(post.get_user().get_username()).size(20.0).into(),
                                        Text::new(post.get_description().clone()).into(),
                                    ])
                                    .into(),
                                    Space::with_width(Length::Fill).into(),
                                    Column::with_children(vec![
                                        Tooltip::new(
                                            Button::new(
                                                Text::new(Icon::Report.to_string())
                                                    .font(ICON)
                                                    .style(theme::text::danger)
                                                    .size(30.0),
                                            )
                                            .on_press(
                                                PostsMessage::ToggleModal(
                                                    ModalType::ShowingReport(index),
                                                )
                                                .into(),
                                            )
                                            .padding(0.0)
                                            .style(iced::widget::button::text),
                                            Text::new("Report post"),
                                            Position::FollowCursor,
                                        )
                                        .into(),
                                        if *user_role == Role::Admin
                                            || user_id == post.get_user().get_id()
                                        {
                                            Tooltip::new(
                                                Button::new(
                                                    Text::new(Icon::Trash.to_string())
                                                        .font(ICON)
                                                        .style(theme::text::danger)
                                                        .size(30),
                                                )
                                                .on_press(
                                                    PostsMessage::DeletePost(post.get_id()).into(),
                                                )
                                                .padding(0.0)
                                                .style(iced::widget::button::text),
                                                Text::new("Delete post"),
                                                Position::FollowCursor,
                                            )
                                            .into()
                                        } else {
                                            Space::with_height(Length::Shrink).into()
                                        },
                                    ])
                                    .into(),
                                ])
                                .spacing(10.0),
                                Image::new(self.get_handle(post.get_id())),
                            )
                            .padding(40)
                            .on_click_image(Into::<Message>::into(PostsMessage::ToggleModal(
                                ModalType::ShowingImage(self.get_handle(post.get_id())),
                            )))
                            .on_click_data(Into::<Message>::into(PostsMessage::ToggleModal(
                                ModalType::ShowingPost(index),
                            )))
                            .into()
                        })
                        .collect::<Vec<Element<Message, Theme, Renderer>>>(),
                )
                .width(Length::Fill)
                .align_items(Alignment::Center)
                .spacing(50),
            )
            .on_scroll(move |viewport| {
                if viewport.relative_offset().y == 1.0 && !self.get_tab(tab).done_loading() {
                    Some(PostsMessage::LoadBatch(tab).into())
                } else {
                    None
                }
            })
            .width(Length::Fill),
        )
        .padding([20.0, 0.0, 0.0, 0.0])
    }

    /// Generate the modal that shows an image.
    pub fn gen_show_image<'a>(
        image: Handle,
        _globals: &Globals,
    ) -> Element<'a, Message, Theme, Renderer> {
        Closeable::new(Image::new(image.clone()).width(Length::Shrink))
            .width(Length::Fill)
            .height(Length::Fill)
            .on_close(
                Into::<Message>::into(PostsMessage::ToggleModal(ModalType::ShowingImage(image))),
                40.0,
            )
            .style(theme::closeable::Closeable::SpotLight)
            .into()
    }

    /// Generate the modal that shows the post.
    pub fn gen_show_post<'a>(
        post_index: usize,
        image: Handle,
        post: &'a Post,
        _globals: &Globals,
    ) -> Element<'a, Message, Theme, Renderer> {
        let mut comment_chain = Column::with_children(vec![Row::with_children(vec![
            TextInput::new("Write comment here...", &*post.get_comment_input())
                .width(Length::Fill)
                .on_input(move |value| {
                    CommentMessage::UpdateInput {
                        post: post_index,
                        position: None,
                        input: value,
                    }
                    .into()
                })
                .into(),
            Button::new("Add comment")
                .on_press(
                    CommentMessage::Add {
                        post: post_index,
                        parent: None,
                    }
                    .into(),
                )
                .into(),
        ])
        .into()]);

        let mut position = if let Some(index) = post.get_open_comment() {
            Ok((0usize, *index))
        } else {
            Err(0usize)
        };

        let mut done = false;
        while !done {
            comment_chain = comment_chain.push(match position {
                Ok((line, index)) => {
                    position = if let Some(reply_index) =
                        post.get_comments()[line][index].get_open_reply()
                    {
                        Ok((
                            post.get_comments()[line][index].get_replies().unwrap(),
                            *reply_index,
                        ))
                    } else {
                        Err(post.get_comments()[line][index]
                            .get_replies()
                            .unwrap_or(post.get_comments().len()))
                    };

                    Into::<Element<Message, Theme, Renderer>>::into(
                        Closeable::new(Column::with_children(vec![
                            Text::new(
                                post.get_comments()[line][index]
                                    .get_user()
                                    .get_username()
                                    .clone(),
                            )
                            .size(17.0)
                            .into(),
                            Text::new(post.get_comments()[line][index].get_content().clone())
                                .into(),
                            Row::with_children(vec![
                                TextInput::new(
                                    "Write reply here...",
                                    &*post.get_comments()[line][index].get_reply_input(),
                                )
                                .on_input(move |value| {
                                    CommentMessage::UpdateInput {
                                        post: post_index,
                                        position: Some((line, index)),
                                        input: value.clone(),
                                    }
                                    .into()
                                })
                                .into(),
                                Button::new("Add reply")
                                    .on_press(
                                        CommentMessage::Add {
                                            post: post_index,
                                            parent: Some((line, index)),
                                        }
                                        .into(),
                                    )
                                    .into(),
                            ])
                            .into(),
                        ]))
                        .on_close(
                            Into::<Message>::into(CommentMessage::Close {
                                post: post_index,
                                position: (line, index),
                            }),
                            20.0,
                        ),
                    )
                }
                Err(line) => {
                    done = true;

                    if line >= post.get_comments().len() {
                        Text::new("Loading").into()
                    } else {
                        Column::with_children(
                            post.get_comments()[line]
                                .iter()
                                .zip(0..post.get_comments()[line].len())
                                .map(|(comment, index)| {
                                    Button::new(Column::with_children(vec![
                                        Text::new(comment.get_user().get_username().clone())
                                            .size(17.0)
                                            .into(),
                                        Text::new(comment.get_content().clone()).into(),
                                    ]))
                                    .style(iced::widget::button::text)
                                    .on_press(
                                        CommentMessage::Open {
                                            post: post_index,
                                            position: (line, index),
                                        }
                                        .into(),
                                    )
                                    .into()
                                })
                                .collect::<Vec<Element<Message, Theme, Renderer>>>(),
                        )
                        .into()
                    }
                }
            });
        }

        Row::with_children(vec![
            Closeable::new(Image::new(image.clone()).width(Length::Shrink))
                .width(Length::FillPortion(3))
                .height(Length::Fill)
                .style(theme::closeable::Closeable::SpotLight)
                .on_click(Into::<Message>::into(PostsMessage::ToggleModal(
                    ModalType::ShowingImage(image),
                )))
                .into(),
            Closeable::new(Column::with_children(vec![
                Text::new(post.get_user().get_username()).size(20.0).into(),
                Text::new(post.get_description().clone()).into(),
                Rating::new()
                    .on_rate(move |value| {
                        PostsMessage::RatePost {
                            post_index: post_index.clone(),
                            rating: value,
                        }
                        .into()
                    })
                    .on_unrate(Into::<Message>::into(PostsMessage::RatePost {
                        post_index,
                        rating: 0,
                    }))
                    .value(*post.get_rating())
                    .into(),
                comment_chain.into(),
            ]))
            .width(Length::FillPortion(1))
            .height(Length::Fill)
            .horizontal_alignment(Alignment::Start)
            .vertical_alignment(Alignment::Start)
            .padding([30.0, 0.0, 0.0, 10.0])
            .style(theme::closeable::Closeable::Default)
            .on_close(
                Into::<Message>::into(PostsMessage::ToggleModal(ModalType::ShowingPost(
                    post_index,
                ))),
                40.0,
            )
            .into(),
        ])
        .into()
    }

    /// Generates the modal for sending a report.
    pub fn gen_show_report(
        &self,
        post_index: usize,
        _globals: &Globals,
    ) -> Element<Message, Theme, Renderer> {
        Closeable::new(
            Card::new(
                Text::new("Report post").size(20.0),
                Column::with_children(vec![
                    TextInput::new(
                        "Give a summary of the issue...",
                        &*self.report_input.clone(),
                    )
                    .on_input(|value| PostsMessage::UpdateReportInput(value.clone()).into())
                    .into(),
                    Container::new(
                        Button::new("Submit")
                            .on_press(PostsMessage::SubmitReport(post_index).into()),
                    )
                    .width(Length::Fill)
                    .align_x(Horizontal::Center)
                    .into(),
                ])
                .padding(20.0)
                .spacing(30.0),
            )
            .width(300.0),
        )
        .on_close(
            Into::<Message>::into(PostsMessage::ToggleModal(ModalType::ShowingReport(
                post_index,
            ))),
            25.0,
        )
        .into()
    }

    /// Returns the required tab.
    fn get_tab(&self, tab: PostTabs) -> &PostList {
        match tab {
            PostTabs::Recommended => &self.recommended,
            PostTabs::Filtered => &self.filtered,
            PostTabs::Profile => &self.profile,
        }
    }

    /// Returns the required tab as mutable.
    fn get_tab_mut(&mut self, tab: PostTabs) -> &mut PostList {
        match tab {
            PostTabs::Recommended => &mut self.recommended,
            PostTabs::Filtered => &mut self.filtered,
            PostTabs::Profile => &mut self.profile,
        }
    }

    /// Returns the active tab.
    fn get_active_tab(&self) -> &PostList {
        self.get_tab(self.active_tab.clone())
    }

    /// Returns the active tab as mutable.
    fn get_active_tab_mut(&mut self) -> &mut PostList {
        self.get_tab_mut(self.active_tab.clone())
    }
}

/// The [Posts] scene does not have any optional initialization values.
#[derive(Debug, Clone, Copy)]
pub struct PostsOptions {}

impl Scene for Posts {
    type Message = PostsMessage;
    type Options = PostsOptions;

    fn new(options: Option<Self::Options>, globals: &mut Globals) -> (Self, Command<Message>)
    where
        Self: Sized,
    {
        let mut posts = Posts {
            modals: ModalStack::new(),
            recommended: PostList::new(vec![]),
            filtered: PostList::new(vec![]),
            tags: HashSet::new(),
            all_tags: HashSet::new(),
            filter_input: String::from(""),
            profile: PostList::new(vec![]),
            user_profile: globals.get_user().unwrap().clone(),
            user_tag_input: String::from(""),
            images: HashMap::new(),
            active_tab: PostTabs::Recommended,
            report_input: String::from(""),
            error: None,
        };

        if let Some(options) = options {
            posts.apply_options(options);
        }

        let db = globals.get_db().unwrap();
        let db_clone = db.clone();
        let user_id = globals.get_user().unwrap().get_id().clone();

        (
            posts,
            Command::batch(vec![
                Self::gen_recommended(db.clone(), user_id),
                Command::perform(
                    async move { database::drawing::get_tags(&db_clone).await },
                    |tags| match tags {
                        Ok(tags) => PostsMessage::LoadedTags(tags).into(),
                        Err(err) => Message::Error(err),
                    },
                ),
                Self::gen_profile(
                    db,
                    user_id,
                    if globals.get_user().unwrap().has_profile_picture() {
                        format!("/{}/profile_picture.webp", user_id)
                    } else {
                        String::from("/default_profile_picture.webp")
                    },
                    globals.get_cache_async(),
                ),
            ]),
        )
    }

    fn get_title(&self) -> String {
        String::from("Posts")
    }

    fn apply_options(&mut self, _options: Self::Options) {}

    fn update(&mut self, globals: &mut Globals, message: &Self::Message) -> Command<Message> {
        match message {
            PostsMessage::LoadPosts => {
                let db = globals.get_db().unwrap();
                let user_id = globals.get_user().unwrap().get_id();

                match self.active_tab {
                    PostTabs::Recommended => Self::gen_recommended(db, user_id),
                    PostTabs::Filtered => Self::gen_filtered(
                        db,
                        user_id,
                        self.tags.iter().map(|tag| tag.get_name().clone()).collect(),
                    ),
                    PostTabs::Profile => Self::gen_profile(
                        db,
                        user_id,
                        if globals.get_user().unwrap().has_profile_picture() {
                            format!("/{}/profile_picture.webp", user_id)
                        } else {
                            String::from("/default_profile_picture.webp")
                        },
                        globals.get_cache_async(),
                    ),
                }
            }
            PostsMessage::LoadedPosts(posts, tab) => {
                let tab = tab.clone();
                *self.get_tab_mut(tab.clone()) = PostList::new(posts.clone());
                let length = posts.len();

                if length > 0 {
                    self.update(globals, &PostsMessage::LoadBatch(tab))
                } else {
                    Command::none()
                }
            }
            PostsMessage::LoadedImage { image, id } => {
                if self.images.contains_key(&id) {
                    return Command::none();
                }

                let id = *id;
                let image = image.clone();
                self.images.insert(id, image.clone());

                let cache = globals.get_cache_async().clone();

                Command::perform(
                    async move {
                        if !cache.contains_key(&id) {
                            cache.insert(id, image.clone()).await
                        }
                    },
                    |()| Message::None,
                )
            }
            PostsMessage::LoadBatch(tab) => {
                let tab = tab.clone();
                let posts = self.get_tab_mut(tab).load_batch();
                let mut user_ids = HashSet::<Uuid>::new();

                let mut commands = vec![];

                for post in posts {
                    let post_id = post.get_id();
                    let user_id = post.get_user().get_id();

                    commands.push(Self::get_image(
                        post_id,
                        format!("/{}/{}.webp", user_id, post_id),
                        globals.get_cache_async(),
                    ));

                    if !user_ids.contains(&user_id) {
                        let has_profile_picture = post.get_user().has_profile_picture();
                        user_ids.insert(user_id);

                        commands.push(Self::get_image(
                            user_id,
                            if has_profile_picture {
                                format!("/{}/profile_picture.webp", user_id)
                            } else {
                                String::from("/default_profile_picture.webp")
                            },
                            globals.get_cache_async(),
                        ));
                    }
                }

                Command::batch(commands)
            }
            PostsMessage::CommentMessage(message) => self.update_comment(&message, globals),
            PostsMessage::ToggleModal(modal) => {
                self.modals.toggle_modal(modal.clone());

                match modal {
                    ModalType::ShowingPost(post) => {
                        if !self.recommended.has_loaded_comments(*post) {
                            self.update_comment(
                                &CommentMessage::Load {
                                    post: *post,
                                    parent: None,
                                },
                                globals,
                            )
                        } else {
                            Command::none()
                        }
                    }
                    ModalType::ShowingReport(_) => {
                        self.report_input = String::from("");
                        Command::none()
                    }
                    _ => Command::none(),
                }
            }
            PostsMessage::RatePost { post_index, rating } => {
                let user_id = globals.get_user().unwrap().get_id();
                let db = globals.get_db().unwrap();

                let (post_id, rating) = self.get_active_tab_mut().rate_post(*post_index, *rating);

                if let Some(rating) = rating {
                    Command::perform(
                        async move {
                            database::posts::update_rating(&db, post_id, user_id, rating as i32)
                                .await
                        },
                        |result| match result {
                            Ok(_) => Message::None,
                            Err(err) => Message::Error(err),
                        },
                    )
                } else {
                    Command::perform(
                        async move { database::posts::delete_rating(&db, post_id, user_id).await },
                        |result| match result {
                            Ok(_) => Message::None,
                            Err(err) => Message::Error(err),
                        },
                    )
                }
            }
            PostsMessage::LoadedTags(tags) => {
                self.all_tags = HashSet::from_iter(tags.iter().map(|tag| tag.clone()));

                Command::none()
            }
            PostsMessage::UpdateFilterInput(filter_input) => {
                self.filter_input = filter_input.clone();

                Command::none()
            }
            PostsMessage::AddTag(tag) => {
                self.tags.insert(tag.clone());
                self.filter_input = String::from("");

                Command::none()
            }
            PostsMessage::RemoveTag(tag) => {
                self.tags.remove(&tag);

                Command::none()
            }
            PostsMessage::OpenProfile(user) => {
                self.error = None;
                self.user_profile = user.clone();
                self.active_tab = PostTabs::Profile;

                Posts::gen_profile(
                    globals.get_db().unwrap(),
                    user.get_id(),
                    if user.has_profile_picture() {
                        format!("/{}/profile_picture.webp", user.get_id())
                    } else {
                        String::from("/default_profile_picture.webp")
                    },
                    globals.get_cache_async(),
                )
            }
            PostsMessage::UpdateUserTagInput(user_tag_input) => {
                self.user_tag_input = user_tag_input.clone();

                Command::none()
            }
            PostsMessage::GetUserByTag => {
                let db = globals.get_db().unwrap();
                let user_tag = self.user_tag_input.clone();

                Command::perform(
                    async move { database::posts::get_user_by_tag(&db, user_tag).await },
                    |result| match result {
                        Ok(user) => PostsMessage::OpenProfile(user).into(),
                        Err(err) => Message::Error(err),
                    },
                )
            }
            PostsMessage::DeletePost(id) => {
                let id = *id;
                self.recommended.remove_post(id);
                self.filtered.remove_post(id);
                self.profile.remove_post(id);
                let globals = globals.clone();

                Command::perform(
                    async move { services::posts::delete_post(id, &globals).await },
                    |result| match result {
                        Ok(_) => Message::None,
                        Err(err) => Message::Error(err),
                    },
                )
            }
            PostsMessage::UpdateReportInput(report_input) => {
                self.report_input = report_input.clone();

                Command::none()
            }
            PostsMessage::SubmitReport(post_index) => {
                let post_index = post_index.clone();
                let report_description = self.report_input.clone();
                let post = self.get_active_tab().get_post(post_index).unwrap();
                let image: PixelImage = self.images.get(&post.get_id()).unwrap().as_ref().clone();

                let mut data = vec![];
                let mut cursor = Cursor::new(&mut data);
                match image::write_buffer_with_format(
                    &mut cursor,
                    image.get_data().as_slice(),
                    image.get_width(),
                    image.get_height(),
                    ExtendedColorType::Rgba8,
                    ImageFormat::WebP,
                ) {
                    Ok(_) => {}
                    Err(err) => {
                        return Command::perform(async {}, move |()| {
                            Message::Error(debug_message!("{}", err).into())
                        });
                    }
                }

                let message = lettre::Message::builder()
                    .from(
                        format!("Chartsy <{}>", config::email_address())
                            .parse()
                            .unwrap(),
                    )
                    .to(
                        format!("Stefan Moldoveanu <{}>", config::admin_email_address())
                            .parse()
                            .unwrap(),
                    )
                    .subject("Anonymous user has submitted a report")
                    .multipart(
                        MultiPart::mixed()
                            .multipart(
                                MultiPart::related()
                                    .singlepart(SinglePart::html(String::from(format!(
                                        "<p>A user has submitted a report regarding a post:</p>\
                                            <p>\"{}\"</p>\
                                            <p>Data regarding the post:</p>\
                                            <p>Username: \"{}\"</p>\
                                            <p>Post description: \"{}\"</p>\
                                            <p>Image:</p>\
                                            <div><img src=cid:post_image></div>",
                                        report_description,
                                        post.get_user().get_username().clone(),
                                        post.get_description().clone()
                                    ))))
                                    .singlepart(
                                        Attachment::new_inline(String::from("post_image"))
                                            .body(data.clone(), "image/*".parse().unwrap()),
                                    ),
                            )
                            .singlepart(
                                Attachment::new(String::from("post_image.webp"))
                                    .body(data.clone(), "image/*".parse().unwrap()),
                            ),
                    )
                    .unwrap();

                Command::batch(vec![
                    Command::perform(async {}, move |()| Message::SendSmtpMail(message)),
                    Command::perform(async {}, move |()| {
                        PostsMessage::ToggleModal(ModalType::ShowingReport(post_index)).into()
                    }),
                ])
            }
            PostsMessage::SelectTab(tab_id) => {
                self.active_tab = *tab_id;

                Command::none()
            }
            PostsMessage::ErrorHandler(error) => {
                self.error = Some(error.clone());

                Command::none()
            }
        }
    }

    fn view(&self, globals: &Globals) -> Element<'_, Message, Theme, Renderer> {
        let recommended_tab = self.gen_post_list(PostTabs::Recommended, globals).into();

        let filtered_tab = Column::with_children(vec![
            Column::with_children(vec![
                Row::with_children(vec![
                    ComboBox::new(
                        self.all_tags.clone(),
                        "Add filter...",
                        &*self.filter_input,
                        |tag| PostsMessage::AddTag(tag).into(),
                    )
                    .on_input(|input| PostsMessage::UpdateFilterInput(input).into())
                    .into(),
                    Button::new("Submit")
                        .on_press(PostsMessage::LoadPosts.into())
                        .into(),
                ])
                .spacing(10.0)
                .into(),
                Grid::new(self.tags.iter().map(|tag| {
                    Container::new(
                        Row::with_children(vec![
                            Text::new(tag.get_name().clone()).into(),
                            Close::new(Into::<Message>::into(PostsMessage::RemoveTag(tag.clone())))
                                .size(15.0)
                                .into(),
                        ])
                        .spacing(5.0)
                        .align_items(Alignment::Center),
                    )
                    .padding(10.0)
                    .style(theme::container::badge)
                }))
                .into(),
            ])
            .padding([0.0, 300.0, 0.0, 300.0])
            .spacing(10.0)
            .into(),
            self.gen_post_list(PostTabs::Filtered, globals).into(),
        ])
        .spacing(20.0)
        .padding([20.0, 0.0, 0.0, 0.0])
        .into();

        let user_tag_input = Row::with_children(vec![
            TextInput::new("Input user tag...", &*self.user_tag_input)
                .on_input(|input| PostsMessage::UpdateUserTagInput(input).into())
                .on_paste(|input| PostsMessage::UpdateUserTagInput(input).into())
                .into(),
            Button::new("Search")
                .on_press(PostsMessage::GetUserByTag.into())
                .into(),
        ])
        .spacing(10.0)
        .padding([20.0, 400.0, 0.0, 400.0])
        .into();

        let profile_tab = if let Some(error) = &self.error {
            Column::with_children(vec![
                user_tag_input,
                Text::new(error.to_string())
                    .size(50.0)
                    .style(theme::text::danger)
                    .into(),
            ])
        } else {
            Column::with_children(vec![
                user_tag_input,
                Button::new(
                    Image::new(self.get_handle(self.user_profile.get_id())).height(Length::Fill),
                )
                .style(iced::widget::button::text)
                .width(Length::Shrink)
                .height(Length::FillPortion(1))
                .on_press(
                    PostsMessage::ToggleModal(ModalType::ShowingImage(
                        self.get_handle(self.user_profile.get_id()),
                    ))
                    .into(),
                )
                .into(),
                Text::new(self.user_profile.get_username())
                    .size(30.0)
                    .into(),
                self.gen_post_list(PostTabs::Profile, globals)
                    .height(Length::FillPortion(3))
                    .into(),
            ])
        }
        .spacing(10.0)
        .align_items(Alignment::Center)
        .into();

        let underlay = Column::with_children(vec![
            Row::with_children(vec![
                Button::new(Text::new(Icon::Leave.to_string()).size(30.0).font(ICON))
                    .on_press(Message::ChangeScene(Scenes::Main(None)))
                    .style(iced::widget::button::text)
                    .padding(10.0)
                    .into(),
                Text::new(self.get_title()).size(30.0).into(),
            ])
            .align_items(Alignment::Center)
            .into(),
            Tabs::new_with_tabs(
                vec![
                    (
                        PostTabs::Recommended,
                        String::from("Recommended"),
                        recommended_tab,
                    ),
                    (PostTabs::Filtered, String::from("Filtered"), filtered_tab),
                    (PostTabs::Profile, String::from("Profile"), profile_tab),
                ],
                |tab_id| PostsMessage::SelectTab(tab_id).into(),
            )
            .selected(self.active_tab)
            .width(Length::Fill)
            .height(Length::Fill)
            .into(),
        ]);

        let modal_generator = |modal_type: ModalType| match modal_type {
            ModalType::ShowingImage(data) => Self::gen_show_image(data.clone(), globals),
            ModalType::ShowingPost(post_index) => {
                let post = self.get_active_tab().get_post(post_index).unwrap();

                Self::gen_show_post(post_index, self.get_handle(post.get_id()), post, globals)
            }
            ModalType::ShowingReport(post_index) => self.gen_show_report(post_index, globals),
        };

        self.modals.get_modal(underlay, modal_generator)
    }

    fn handle_error(&mut self, globals: &mut Globals, error: &Error) -> Command<Message> {
        self.update(globals, &PostsMessage::ErrorHandler(error.clone()))
    }

    fn clear(&self, _globals: &mut Globals) {}
}
