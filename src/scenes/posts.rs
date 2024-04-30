use std::any::Any;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use iced::advanced::image::Handle;
use iced::{Alignment, Element, Length, Renderer, Command};
use iced::alignment::Horizontal;
use iced::widget::{Column, Row, Scrollable, Image, Text, TextInput, Button, Space, Tooltip, Container};
use iced::widget::tooltip::Position;
use iced_aw::{TabLabel, Tabs};
use lettre::message::{Attachment, MultiPart, SinglePart};
use moka::future::Cache;
use mongodb::bson::Uuid;
use mongodb::Database;
use crate::widgets::closeable::Closeable;
use crate::widgets::modal_stack::ModalStack;
use crate::widgets::post_summary::PostSummary;
use crate::{config, database, LOADING_IMAGE};
use crate::errors::debug::{debug_message, DebugError};
use crate::errors::error::Error;
use crate::icons::{ICON, Icon};
use crate::scene::{Action, Globals, Message, Scene, SceneOptions};
use crate::scenes::data::auth::User;
use crate::scenes::data::drawing::Tag;
use crate::theme::Theme;
use crate::widgets::rating::Rating;

use crate::scenes::data::posts::*;
use crate::widgets::card::Card;
use crate::widgets::close::Close;
use crate::widgets::combo_box::ComboBox;
use crate::widgets::grid::Grid;

/// The [messages](Action) that can be triggered on the [Posts] scene.
#[derive(Clone)]
enum PostsAction {
    /// Loads posts for the active tab.
    LoadPosts,

    /// Triggers when some posts are loaded to be displayed.
    LoadedPosts(Vec<Post>, PostTabs),

    /// Triggers when the given amount of images from the posts have been loaded.
    LoadedImage{ image: Arc<Vec<u8>>, id: Uuid},

    /// Loads a batch of images.
    LoadBatch(PostTabs),

    /// Handles messages related to comments.
    CommentMessage(CommentMessage),

    /// Triggers when a [modal](ModalType) is toggled.
    ToggleModal(ModalType),

    /// Sets the rating of the given post.
    RatePost{ post_index: usize, rating: usize },

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

    /// Updates the post report input.
    UpdateReportInput(String),

    /// Submits a post report.
    SubmitReport(usize),

    /// Selects a tab.
    SelectTab(PostTabs),

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
            PostsAction::LoadPosts => String::from("Load posts"),
            PostsAction::LoadedPosts(_, _) => String::from("Loaded posts"),
            PostsAction::LoadedImage{ .. } => String::from("Loaded image"),
            PostsAction::LoadBatch(_) => String::from("Load batch"),
            PostsAction::CommentMessage(_) => String::from("Loaded comments"),
            PostsAction::ToggleModal(_) => String::from("Toggle modal"),
            PostsAction::RatePost { .. } => String::from("Rate post"),
            PostsAction::LoadedTags(_) => String::from("Loaded tags"),
            PostsAction::UpdateFilterInput(_) => String::from("Update filter input"),
            PostsAction::AddTag(_) => String::from("Add tag"),
            PostsAction::RemoveTag(_) => String::from("Remove tag"),
            PostsAction::OpenProfile(_) => String::from("Open profile"),
            PostsAction::UpdateUserTagInput(_) => String::from("Update user tag input"),
            PostsAction::GetUserByTag => String::from("Get user by tag"),
            PostsAction::UpdateReportInput(_) => String::from("Update report input"),
            PostsAction::SubmitReport(_) => String::from("Submit report"),
            PostsAction::SelectTab(_) => String::from("Select tab"),
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
    images: HashMap<Uuid, Arc<Vec<u8>>>,

    /// Currently active tab.
    active_tab: PostTabs,

    /// The user input of a report.
    report_input: String,

    /// User error.
    error: Option<Error>
}

impl Posts {
    /// Get an image if it is not in cache.
    fn get_image(image_id: Uuid, image_path: String, cache: &Cache<Uuid, Arc<Vec<u8>>>) -> Command<Message>
    {
        let cache = cache.clone();

        Command::perform(
            async move {
                cache.try_get_with(
                    image_id,
                    async move {
                        database::base::download_file(image_path).await.map(
                            |data| Arc::new(data)
                        )
                    }
                ).await
            },
            move |result| {
                match result {
                    Ok(data) => Message::DoAction(
                        Box::new(PostsAction::LoadedImage {
                            image: data,
                            id: image_id,
                        })
                    ),
                    Err(err) => Message::Error(err.as_ref().clone())
                }
            }
        )
    }

    /// Creates a command that returns a list of recommended posts.
    fn gen_recommended(db: Database, user_id: Uuid) -> Command<Message>
    {
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
                    Ok(posts) => Message::DoAction(Box::new(
                        PostsAction::LoadedPosts(posts, PostTabs::Recommended)
                    )),
                    Err(err) => Message::Error(err)
                }
            }
        )
    }

    /// Creates a command that returns the list of posts that has all tags from the filter.
    fn gen_filtered(db: Database, user_id: Uuid, tags: Vec<String>) -> Command<Message>
    {
        Command::perform(
            async move {
                database::posts::get_filtered(&db, user_id, tags).await
            },
            |result| {
                match result {
                    Ok(posts) => Message::DoAction(Box::new(
                        PostsAction::LoadedPosts(posts, PostTabs::Filtered)
                    )),
                    Err(err) => Message::Error(err)
                }
            }
        )
    }

    /// Creates a command that returns the list of posts on the given users profile.
    fn gen_profile(
        db: Database,
        user_id: Uuid,
        profile_picture_path: String,
        cache: &Cache<Uuid, Arc<Vec<u8>>>
    ) -> Command<Message> {
        Command::batch(vec![
            Command::perform(
                async move {
                    database::posts::get_user_posts(&db, user_id).await
                },
                |result| {
                    match result {
                        Ok(posts) => Message::DoAction(Box::new(
                            PostsAction::LoadedPosts(posts, PostTabs::Profile)
                        )),
                        Err(err) => Message::Error(err)
                    }
                }
            ),
            Self::get_image(user_id, profile_picture_path, cache)
        ])
    }

    /// Applies the update corresponding the given message.
    fn update_comment(&mut self, comment_message: &CommentMessage, globals: &mut Globals) -> Command<Message>
    {
        match comment_message {
            CommentMessage::Open { post, position } => {
                let (line, index) = position;

                if self.get_active_tab_mut().open_comment(*post, *line, *index) {
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

                self.get_active_tab_mut().close_comment(*post, *line, *index);

                Command::none()
            }
            CommentMessage::UpdateInput {post, position, input} => {
                self.get_active_tab_mut().update_input(*post, *position, input.clone());

                Command::none()
            }
            CommentMessage::Add { post, parent } => {
                let db = globals.get_db().unwrap();
                let user = globals.get_user().unwrap().clone();

                let document = if let Some((line, index)) = parent {
                    self.get_active_tab_mut().add_reply(user, *post, *line, *index)
                } else {
                    self.get_active_tab_mut().add_comment(user, *post)
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

                let active_tab = self.active_tab;
                let filter = self.get_tab_mut(active_tab).load_comments(post, parent);

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
                                        comments,
                                        tab: active_tab
                                    }
                                )))
                            }
                            Err(err) => Message::Error(err)
                        }
                    }
                )
            }
            CommentMessage::Loaded { post, parent, comments, tab } => {
                self.get_tab_mut(*tab).loaded_comments(*post, *parent, comments.clone());

                Command::none()
            }
        }
    }

    /// Generates the visible list of posts.
    pub fn gen_post_list(&self, tab: PostTabs, _globals: &Globals) -> Container<'_, Message, Theme, Renderer>
    {
        Container::new(
            Scrollable::new(
                Column::with_children(
                    self.get_tab(tab).get_loaded_posts().into_iter().map(
                        |(post, index)| {
                            PostSummary::<Message, Theme, Renderer>::new(
                                Row::with_children(vec![
                                    Tooltip::new(
                                        Button::new(
                                            Image::new(Handle::from_memory(
                                                self.images.get(&post.get_user().get_id())
                                                    .map(|image| image.as_ref().clone())
                                                    .unwrap_or(LOADING_IMAGE.into())
                                            ))
                                                .width(50.0)
                                                .height(50.0)
                                        )
                                            .on_press(Message::DoAction(Box::new(
                                                PostsAction::OpenProfile(post.get_user().clone())
                                            )))
                                            .style(crate::theme::button::Button::Transparent),
                                        Text::new(format!("{}'s profile", post.get_user().get_user_tag())),
                                        Position::FollowCursor
                                    )
                                        .into(),
                                    Column::with_children(vec![
                                        Tooltip::new(
                                            Button::new(
                                                Text::new(format!("@{}", post.get_user().get_user_tag()))
                                                    .size(15.0)
                                                    .style(crate::theme::text::Text::Gray)
                                            )
                                                .style(crate::theme::button::Button::Transparent)
                                                .on_press(Message::DoAction(Box::new(
                                                    PostsAction::OpenProfile(post.get_user().clone())
                                                ))),
                                            Text::new(format!("{}'s profile", post.get_user().get_user_tag())),
                                            Position::FollowCursor
                                        )
                                            .into(),
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
                                ])
                                    .spacing(10.0),
                                Image::new(Handle::from_memory(
                                    post.get_image(&self.images)
                                        .map(|image| image.as_ref().clone())
                                        .unwrap_or(LOADING_IMAGE.into())
                                )).width(Length::Shrink)
                            )
                                .padding(40)
                                .on_click_image(Message::DoAction(Box::new(PostsAction::ToggleModal(
                                    ModalType::ShowingImage(Handle::from_memory(
                                        post.get_image(&self.images)
                                            .map(|image| image.as_ref().clone())
                                            .unwrap_or(LOADING_IMAGE.into())
                                    ))
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
                .on_scroll(move |viewport| {
                    if viewport.relative_offset().y == 1.0 && self.get_tab(tab).done_loading() {
                        Message::DoAction(Box::new(PostsAction::LoadBatch(tab)))
                    } else {
                        Message::None
                    }
                })
                .width(Length::Fill)
        )
            .padding([20.0, 0.0, 0.0, 0.0])
    }

    /// Generate the modal that shows an image.
    pub fn gen_show_image<'a>(image: Handle, _globals: &Globals) -> Element<'a, Message, Theme, Renderer>
    {
        Closeable::new(Image::new(
            image.clone()
        ).width(Length::Shrink))
            .width(Length::Fill)
            .height(Length::Fill)
            .on_close(
                Message::DoAction(Box::new(PostsAction::ToggleModal(
                    ModalType::ShowingImage(image)
                ))),
                40.0
            )
            .style(crate::theme::closeable::Closeable::SpotLight)
            .into()
    }

    /// Generate the modal that shows the post.
    pub fn gen_show_post<'a>(post_index: usize, image: Handle, post: &'a Post, _globals: &Globals)
        -> Element<'a, Message, Theme, Renderer> {
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
                Closeable::new(
                    Image::new(image.clone())
                        .width(Length::Shrink)
                )
                    .width(Length::FillPortion(3))
                    .height(Length::Fill)
                    .style(crate::theme::closeable::Closeable::SpotLight)
                    .on_click(Message::DoAction(Box::new(PostsAction::ToggleModal(
                        ModalType::ShowingImage(image)
                    ))))
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

    /// Generates the modal for sending a report.
    pub fn gen_show_report(&self, post_index: usize, _globals: &Globals) -> Element<Message, Theme, Renderer>
    {
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

    /// Returns the required tab.
    fn get_tab(&self, tab: PostTabs) -> &PostList {
        match tab {
            PostTabs::Recommended => &self.recommended,
            PostTabs::Filtered => &self.filtered,
            PostTabs::Profile => &self.profile
        }
    }

    /// Returns the required tab as mutable.
    fn get_tab_mut(&mut self, tab: PostTabs) -> &mut PostList {
        match tab {
            PostTabs::Recommended => &mut self.recommended,
            PostTabs::Filtered => &mut self.filtered,
            PostTabs::Profile => &mut self.profile
        }
    }

    /// Returns the active tab.
    fn get_active_tab(&self) -> &PostList { self.get_tab(self.active_tab.clone()) }

    /// Returns the active tab as mutable.
    fn get_active_tab_mut(&mut self) -> &mut PostList { self.get_tab_mut(self.active_tab.clone()) }
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
            error: None
        };

        if let Some(options) = options {
            options.apply_options(&mut posts);
        }

        let db = globals.get_db().unwrap();
        let db_clone = db.clone();
        let user_id = globals.get_user().unwrap().get_id().clone();

        (
            posts,
            Command::batch(vec![
                Self::gen_recommended(db.clone(), user_id),
                Command::perform(
                    async move {
                        database::drawing::get_tags(&db_clone).await
                    },
                    |tags| {
                        match tags {
                            Ok(tags) => Message::DoAction(Box::new(
                                PostsAction::LoadedTags(tags)
                            )),
                            Err(err) => Message::Error(err)
                        }
                    }
                ),
                Self::gen_profile(
                    db,
                    user_id,
                    if globals.get_user().unwrap().has_profile_picture() {
                        format!("/{}/profile_picture.webp", user_id)
                    } else {
                        String::from("/default_profile_picture.webp")
                    },
                    globals.get_cache()
                )
            ])
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
                    debug_message!(format!("Message doesn't belong to posts scene: {}.", message.get_name()))
                ))
            ))
        };

        match message {
            PostsAction::LoadPosts => {
                let db = globals.get_db().unwrap();
                let user_id = globals.get_user().unwrap().get_id();

                match self.active_tab {
                    PostTabs::Recommended => Self::gen_recommended(db, user_id),
                    PostTabs::Filtered => Self::gen_filtered(
                        db,
                        user_id,
                        self.tags.iter().map(|tag| tag.get_name().clone()).collect()
                    ),
                    PostTabs::Profile => Self::gen_profile(
                        db,
                        user_id,
                        if globals.get_user().unwrap().has_profile_picture() {
                            format!("/{}/profile_picture.webp", user_id)
                        } else {
                            String::from("/default_profile_picture.webp")
                        },
                        globals.get_cache()
                    )
                }
            }
            PostsAction::LoadedPosts(posts, tab) => {
                let tab = tab.clone();
                *self.get_tab_mut(tab.clone()) = PostList::new(posts.clone());
                let length = posts.len();

                if length > 0 {
                    self.update(globals, Box::new(PostsAction::LoadBatch(tab)))
                } else {
                    Command::none()
                }
            }
            PostsAction::LoadedImage { image, id } => {
                if self.images.contains_key(id) {
                    return Command::none();
                }

                let id = *id;
                let image = image.clone();
                self.images.insert(id, image.clone());

                let cache = globals.get_cache().clone();

                Command::perform(
                    async move {
                        if !cache.contains_key(&id) {
                            cache.insert(id, image.clone()).await
                        }
                    },
                    |()| Message::None
                )
            }
            PostsAction::LoadBatch(tab) => {
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
                        globals.get_cache()
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
                            globals.get_cache()
                        ));
                    }
                }

                Command::batch(commands)
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
                    ModalType::ShowingReport(_) => {
                        self.report_input = String::from("");
                        Command::none()
                    }
                    _ => Command::none()
                }
            }
            PostsAction::RatePost { post_index, rating } => {
                let user_id = globals.get_user().unwrap().get_id();
                let db = globals.get_db().unwrap();

                let (post_id, rating) =
                    self.get_active_tab_mut().rate_post(*post_index, *rating);

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
            PostsAction::LoadedTags(tags) => {
                self.all_tags = HashSet::from_iter(tags.iter().map(|tag| tag.clone()));

                Command::none()
            }
            PostsAction::UpdateFilterInput(filter_input) => {
                self.filter_input = filter_input.clone();

                Command::none()
            }
            PostsAction::AddTag(tag) => {
                self.tags.insert(tag.clone());
                self.filter_input = String::from("");

                Command::none()
            }
            PostsAction::RemoveTag(tag) => {
                self.tags.remove(tag);

                Command::none()
            }
            PostsAction::OpenProfile(user) => {
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
                    globals.get_cache()
                )
            }
            PostsAction::UpdateUserTagInput(user_tag_input) => {
                self.user_tag_input = user_tag_input.clone();

                Command::none()
            }
            PostsAction::GetUserByTag => {
                let db = globals.get_db().unwrap();
                let user_tag = self.user_tag_input.clone();

                Command::perform(
                    async move {
                        database::posts::get_user_by_tag(&db, user_tag).await
                    },
                    |result| {
                        match result {
                            Ok(user) => Message::DoAction(Box::new(
                                PostsAction::OpenProfile(user)
                            )),
                            Err(err) => Message::Error(err)
                        }
                    }
                )
            }
            PostsAction::UpdateReportInput(report_input) => {
                self.report_input = report_input.clone();

                Command::none()
            }
            PostsAction::SubmitReport(post_index) => {
                let post_index = post_index.clone();
                let report_description = self.report_input.clone();
                let post = self.get_active_tab().get_post(post_index).unwrap();
                let image = self.images.get(&post.get_id()).unwrap().as_ref().clone();

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
                                                image.clone(),
                                                "image/*".parse().unwrap()
                                            )
                                    )
                            )
                            .singlepart(
                                Attachment::new(String::from("post_image.webp"))
                                    .body(
                                        image,
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
            PostsAction::SelectTab(tab_id) => {
                self.active_tab = *tab_id;

                Command::none()
            }
            PostsAction::ErrorHandler(error) => {
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
                        |tag| Message::DoAction(Box::new(PostsAction::AddTag(tag)))
                    )
                        .on_input(|input| Message::DoAction(Box::new(
                            PostsAction::UpdateFilterInput(input))
                        ))
                        .into(),
                    Button::new("Submit")
                        .on_press(Message::DoAction(Box::new(PostsAction::LoadPosts)))
                        .into()
                ])
                    .spacing(10.0)
                    .into(),
                Grid::new(self.tags.iter().map(
                    |tag| Container::new(
                        Row::with_children(vec![
                            Text::new(tag.get_name().clone()).into(),
                            Close::new(
                                Message::DoAction(Box::new(PostsAction::RemoveTag(tag.clone())))
                            )
                                .size(15.0)
                                .into()
                        ])
                            .spacing(5.0)
                            .align_items(Alignment::Center)
                    )
                        .padding(10.0)
                        .style(crate::theme::container::Container::Badge(crate::theme::pallete::TEXT))
                ))
                    .into()

            ])
                .padding([0.0, 300.0, 0.0, 300.0])
                .spacing(10.0)
                .into(),
            self.gen_post_list(PostTabs::Filtered, globals)
                .into()
        ])
            .spacing(20.0)
            .padding([20.0, 0.0, 0.0, 0.0])
            .into();

        let user_tag_input = Row::with_children(vec![
            TextInput::new("Input user tag...", &*self.user_tag_input)
                .on_input(|input| Message::DoAction(Box::new(
                    PostsAction::UpdateUserTagInput(input)
                )))
                .on_paste(|input| Message::DoAction(Box::new(
                    PostsAction::UpdateUserTagInput(input)
                )))
                .into(),
            Button::new("Search")
                .on_press(Message::DoAction(Box::new(PostsAction::GetUserByTag)))
                .into()
        ])
            .spacing(10.0)
            .padding([20.0, 400.0, 0.0, 400.0])
            .into();

        let profile_tab = if let Some(error) = &self.error {
            Column::with_children(vec![
                user_tag_input,
                Text::new(error.to_string())
                    .size(50.0)
                    .style(crate::theme::text::Text::Error)
                    .into()
            ])
        } else {
            Column::with_children(vec![
                user_tag_input,
                Button::new(
                    Image::new(Handle::from_memory(
                        self.images.get(&self.user_profile.get_id()).map(
                            |image| image.as_ref().clone()
                        ).unwrap_or(LOADING_IMAGE.to_vec())
                    ))
                        .height(Length::Fill)
                )
                    .style(crate::theme::button::Button::Transparent)
                    .width(Length::Shrink)
                    .height(Length::FillPortion(1))
                    .on_press(Message::DoAction(Box::new(PostsAction::ToggleModal(
                        ModalType::ShowingImage(
                            Handle::from_memory(
                                self.images.get(&self.user_profile.get_id()).map(
                                    |image| image.as_ref().clone()
                                ).unwrap_or(LOADING_IMAGE.to_vec())
                            )
                        )
                    ))))
                    .into(),
                Text::new(self.user_profile.get_username())
                    .size(30.0)
                    .into(),
                self.gen_post_list(PostTabs::Profile, globals)
                    .height(Length::FillPortion(3))
                    .into()
            ])
        }
            .spacing(10.0)
            .align_items(Alignment::Center)
            .into();

        let underlay = Tabs::new_with_tabs(
            vec![
                (
                    PostTabs::Recommended,
                    TabLabel::Text(String::from("Recommended")),
                    recommended_tab
                ),
                (
                    PostTabs::Filtered,
                    TabLabel::Text(String::from("Filtered")),
                    filtered_tab
                ),
                (
                    PostTabs::Profile,
                    TabLabel::Text(String::from("Profile")),
                    profile_tab
                )
            ],
            |tab_id| Message::DoAction(Box::new(PostsAction::SelectTab(tab_id)))
        )
            .set_active_tab(&self.active_tab);

        let modal_generator = |modal_type: ModalType| {
            match modal_type {
                ModalType::ShowingImage(data) => {
                    Self::gen_show_image(data.clone(), globals)
                }
                ModalType::ShowingPost(post_index) => {
                    let post = self.get_active_tab().get_post(post_index).unwrap();

                    Self::gen_show_post(
                        post_index,
                        Handle::from_memory(
                            self.images.get(&post.get_id())
                                .map(|data| data.as_ref().clone())
                                .unwrap_or(LOADING_IMAGE.to_vec())
                        ),
                        post,
                        globals
                    )
                }
                ModalType::ShowingReport(post_index) => {
                    self.gen_show_report(post_index, globals)
                }
            }
        };

        self.modals.get_modal(underlay, modal_generator)
    }

    fn get_error_handler(&self, error: Error) -> Box<dyn Action> {
        Box::new(PostsAction::ErrorHandler(error))
    }

    fn clear(&self, _globals: &mut Globals) { }
}