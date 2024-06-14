use crate::debug_message;
use crate::scene::{Globals, Message, Scene, SceneMessage};
use crate::scenes::data::auth::User;
use crate::scenes::data::drawing::Tag;
use crate::scenes::services;
use crate::utils::errors::Error;
use crate::utils::icons::{Icon, ICON};
use crate::utils::theme::{self, Theme};
use crate::widgets::{Close, ComboBox, Grid, ModalStack, Tabs};
use crate::{config, database};
use iced::widget::text_editor::{Action, Content};
use iced::widget::{Button, Column, Container, Row, Text, TextInput};
use iced::{Alignment, Command, Element, Length, Renderer, Size};
use image::{ExtendedColorType, ImageFormat};
use lettre::message::{Attachment, MultiPart, SinglePart};
use mongodb::bson::Uuid;
use mongodb::Database;
use std::any::Any;
use std::collections::HashSet;
use std::io::Cursor;

use crate::scenes::data::posts::*;

use super::scenes::Scenes;

/// The [messages](SceneMessage) that can be triggered on the [Posts] scene.
#[derive(Clone)]
pub enum PostsMessage {
    /// Loads posts for the active tab.
    LoadPosts,

    /// Triggers when some posts are loaded to be displayed.
    LoadedPosts(Vec<Post>, PostTabs),

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
    UpdateReportInput(Action),

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

    /// Currently active tab.
    active_tab: PostTabs,

    /// The user input of a report.
    report_input: Content,

    /// User error.
    error: Option<Error>,
}

impl Posts {
    /// Loads all necessary images.
    pub fn load_images(&self, globals: &Globals) -> Command<Message> {
        let ids = self
            .recommended
            .get_loaded_posts()
            .into_iter()
            .chain(self.filtered.get_loaded_posts())
            .chain(self.profile.get_loaded_posts())
            .map(|(post, _)| (post.get_id(), post.get_user().get_id()));

        let post_images =
            globals
                .get_cache()
                .insert_if_not(ids, |(id, _)| id, services::posts::load_post);

        let profile_picure_ids = self
            .recommended
            .get_loaded_posts()
            .into_iter()
            .chain(self.filtered.get_loaded_posts())
            .map(|(post, _)| {
                post.get_user()
                    .has_profile_picture()
                    .then_some(post.get_user().get_id())
            })
            .chain(vec![self
                .user_profile
                .has_profile_picture()
                .then_some(self.user_profile.get_id())]);

        let profile_pictures = globals.get_cache().insert_if_not(
            profile_picure_ids,
            |option| option.unwrap_or(Uuid::from_bytes([0; 16])),
            services::posts::load_profile_picture,
        );

        Command::batch(vec![post_images, profile_pictures])
    }

    /// Creates a command that returns a list of recommended posts.
    fn gen_recommended(db: Database, user_id: Uuid) -> Command<Message> {
        Command::perform(
            services::posts::generate_recommended(db, user_id),
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
    fn gen_profile(db: Database, user_id: Uuid) -> Command<Message> {
        Command::perform(
            async move { database::posts::get_user_posts(&db, user_id).await },
            |result| match result {
                Ok(posts) => PostsMessage::LoadedPosts(posts, PostTabs::Profile).into(),
                Err(err) => Message::Error(err),
            },
        )
    }

    fn open_comment(
        &mut self,
        post: &usize,
        position: &(usize, usize),
        globals: &mut Globals,
    ) -> Command<Message> {
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

    fn close_comment(&mut self, post: &usize, position: &(usize, usize)) -> Command<Message> {
        let (line, index) = position;

        self.get_active_tab_mut()
            .close_comment(*post, *line, *index);

        Command::none()
    }

    fn add_comment(
        &mut self,
        post: &usize,
        parent: &Option<(usize, usize)>,
        globals: &Globals,
    ) -> Command<Message> {
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

    fn load_comments(
        &mut self,
        post: &usize,
        parent: &Option<(usize, usize)>,
        globals: &Globals,
    ) -> Command<Message> {
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

    /// Applies the update corresponding the given message.
    fn update_comment(
        &mut self,
        comment_message: &CommentMessage,
        globals: &mut Globals,
    ) -> Command<Message> {
        match comment_message {
            CommentMessage::Open { post, position } => self.open_comment(post, position, globals),
            CommentMessage::Close { post, position } => self.close_comment(post, position),
            CommentMessage::UpdateInput {
                post,
                position,
                input,
            } => {
                self.get_active_tab_mut()
                    .update_input(*post, *position, input.clone());

                Command::none()
            }
            CommentMessage::Add { post, parent } => self.add_comment(post, parent, globals),
            CommentMessage::Load { post, parent } => self.load_comments(post, parent, globals),
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
        size: Size<Length>,
    ) -> Element<'_, Message, Theme, Renderer> {
        services::posts::generate_post_list(
            tab,
            self.get_tab(tab),
            globals.get_user().unwrap(),
            globals.get_cache(),
        )
        .width(size.width)
        .height(size.height)
        .into()
    }

    /// Generate the modal that shows an image.
    pub fn gen_show_image<'a>(
        id: Uuid,
        globals: &Globals,
    ) -> Element<'a, Message, Theme, Renderer> {
        services::posts::generate_show_image(id, globals.get_cache())
    }

    /// Generate the modal that shows the post.
    pub fn gen_show_post<'a>(
        post_index: usize,
        post: &'a Post,
        globals: &Globals,
    ) -> Element<'a, Message, Theme, Renderer> {
        services::posts::generate_show_post(post, post_index, &globals.get_cache())
    }

    /// Generates the modal for sending a report.
    pub fn gen_show_report(
        &self,
        post_index: usize,
        _globals: &Globals,
    ) -> Element<Message, Theme, Renderer> {
        services::posts::generate_show_report(post_index, &self.report_input)
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

    /// Loads the posts for the given tab.
    fn load_posts(&mut self, tab: PostTabs, globals: &mut Globals) -> Command<Message> {
        let db = globals.get_db().unwrap();
        let user_id = globals.get_user().unwrap().get_id();

        match tab {
            PostTabs::Recommended => Self::gen_recommended(db, user_id),
            PostTabs::Filtered => Self::gen_filtered(
                db,
                user_id,
                self.tags.iter().map(|tag| tag.get_name().clone()).collect(),
            ),
            PostTabs::Profile => Self::gen_profile(db, user_id),
        }
    }

    /// Receives a list of posts for a tab.
    fn loaded_posts(
        &mut self,
        posts: &Vec<Post>,
        tab: &PostTabs,
        globals: &mut Globals,
    ) -> Command<Message> {
        let tab = tab.clone();
        *self.get_tab_mut(tab.clone()) = PostList::new(posts.clone());
        let length = posts.len();

        if length > 0 {
            self.update(globals, &PostsMessage::LoadBatch(tab))
        } else {
            Command::none()
        }
    }

    /// Toggles the given modal.
    fn toggle_modal(&mut self, modal: &ModalType, globals: &mut Globals) -> Command<Message> {
        self.modals.toggle_modal(modal.clone());

        match modal {
            ModalType::ShowingPost(post) => {
                if !self.get_active_tab().has_loaded_comments(*post) {
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
                self.report_input = Content::new();
                Command::none()
            }
            _ => Command::none(),
        }
    }

    /// Changes rating given to a post.
    fn rate_post(
        &mut self,
        post_index: usize,
        rating: usize,
        globals: &mut Globals,
    ) -> Command<Message> {
        let user_id = globals.get_user().unwrap().get_id();
        let db = globals.get_db().unwrap();

        let (post_id, rating) = self.get_active_tab_mut().rate_post(post_index, rating);

        if let Some(rating) = rating {
            Command::perform(
                async move {
                    database::posts::update_rating(&db, post_id, user_id, rating as i32).await
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

    /// Submits a report.
    fn submit_report(&mut self, post_index: usize, globals: &mut Globals) -> Command<Message> {
        let post_index = post_index.clone();
        let report_description = self.report_input.text();
        let post = self.get_active_tab().get_post(post_index).unwrap();
        let image = match globals.get_cache().get(post.get_id()) {
            Some(image) => image,
            None => {
                return Command::perform(async {}, |()| {
                    Message::Error(debug_message!("Post image not loaded yet.").into())
                });
            }
        };

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
                                report_description.clone(),
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
            active_tab: PostTabs::Recommended,
            report_input: Content::new(),
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
                Self::gen_profile(db, user_id),
            ]),
        )
    }

    fn get_title(&self) -> String {
        String::from("Posts")
    }

    fn apply_options(&mut self, _options: Self::Options) {}

    fn update(&mut self, globals: &mut Globals, message: &Self::Message) -> Command<Message> {
        match message {
            PostsMessage::LoadPosts => self.load_posts(self.active_tab, globals),
            PostsMessage::LoadedPosts(posts, tab) => self.loaded_posts(posts, tab, globals),
            PostsMessage::LoadBatch(tab) => {
                let tab = tab.clone();
                self.get_tab_mut(tab).load_batch();

                Command::none()
            }
            PostsMessage::CommentMessage(message) => self.update_comment(&message, globals),
            PostsMessage::ToggleModal(modal) => self.toggle_modal(modal, globals),
            PostsMessage::RatePost { post_index, rating } => {
                self.rate_post(*post_index, *rating, globals)
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

                self.modals.clear();

                Posts::gen_profile(globals.get_db().unwrap(), user.get_id())
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
            PostsMessage::UpdateReportInput(action) => {
                self.report_input.perform(action.clone());

                Command::none()
            }
            PostsMessage::SubmitReport(post_index) => self.submit_report(*post_index, globals),
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
        let recommended_tab = self
            .gen_post_list(
                PostTabs::Recommended,
                globals,
                Size::new(Length::Shrink, Length::Shrink),
            )
            .into();

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
                            Text::new(tag.get_name().clone())
                                .style(theme::text::dark)
                                .into(),
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
            self.gen_post_list(
                PostTabs::Filtered,
                globals,
                Size::new(Length::Shrink, Length::Shrink),
            )
            .into(),
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
                Button::new(globals.get_cache().get_element(
                    if self.user_profile.has_profile_picture() {
                        self.user_profile.get_id()
                    } else {
                        Uuid::from_bytes([0; 16])
                    },
                    Size::new(Length::Shrink, Length::Fill),
                    Size::new(Length::Fixed(400.0), Length::Fixed(300.0)),
                    None,
                ))
                .style(iced::widget::button::text)
                .width(Length::Shrink)
                .height(Length::FillPortion(1))
                .on_press(
                    PostsMessage::ToggleModal(ModalType::ShowingImage(self.user_profile.get_id()))
                        .into(),
                )
                .padding(0.0)
                .into(),
                Text::new(self.user_profile.get_username())
                    .size(30.0)
                    .into(),
                self.gen_post_list(
                    PostTabs::Profile,
                    globals,
                    Size::new(Length::Fill, Length::FillPortion(3)),
                ),
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

                Self::gen_show_post(post_index, post, globals)
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
