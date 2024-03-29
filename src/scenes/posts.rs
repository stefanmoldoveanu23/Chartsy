use std::any::Any;
use std::{fs, io};
use std::ops::DerefMut;
use dropbox_sdk::default_client::{NoauthDefaultClient, UserAuthDefaultClient};
use dropbox_sdk::files;
use dropbox_sdk::files::DownloadArg;
use dropbox_sdk::oauth2::Authorization;
use iced::advanced::image::Handle;
use iced::{Alignment, Element, Length, Renderer, Command};
use iced::widget::{Column, Row, Scrollable, Image, Text};
use mongodb::bson::{Bson, doc, Document, Uuid, UuidRepresentation};
use mongodb::Cursor;
use mongodb::options::{AggregateOptions, UpdateOptions};
use crate::widgets::closeable::Closeable;
use crate::widgets::modal_stack::ModalStack;
use crate::widgets::post_summary::PostSummary;
use crate::config;
use crate::errors::debug::DebugError;
use crate::errors::error::Error;
use crate::mongo::{MongoRequest, MongoRequestType};
use crate::scene::{Action, Globals, Message, Scene, SceneOptions};
use crate::scenes::auth::User;
use crate::serde::Deserialize;
use crate::theme::Theme;
use crate::widgets::rating::Rating;

/// The data for a loaded post.
#[derive(Clone)]
struct Post {
    /// The id of the post.
    id: Uuid,

    /// The data of the image.
    image: Vec<u8>,

    /// The description of the [Post].
    description: String,

    /// The tags of the [Post].
    tags: Vec<String>,

    /// The [User] this [Post] belongs to.
    user: User,

    /// The id of the drawing.
    drawing_id: Uuid,

    /// The rating of the post.
    rating: usize,
}

impl Default for Post {
    fn default() -> Self {
        Post {
            id: Uuid::default(),
            image: fs::read("./src/images/loading.png").unwrap(),
            description: "".into(),
            tags: vec![],
            user: User::default(),
            drawing_id: Uuid::default(),
            rating: 0
        }
    }
}

impl Deserialize<Document> for Post {
    fn deserialize(document: Document) -> Self where Self: Sized {
        let mut post :Post= Default::default();

        if let Some(Bson::Document(post_data)) = document.get("post") {
            if let Some(Bson::String(description)) = post_data.get("description") {
                post.description = description.clone();
            }
            if let Some(Bson::Array(tags)) = post_data.get("tags") {
                for tag in tags {
                    if let Bson::String(tag) = tag {
                        post.tags.push(tag.clone());
                    }
                }
            }
            if let Some(Bson::Binary(bin)) = post_data.get("drawing_id") {
                post.drawing_id = bin.to_uuid_with_representation(UuidRepresentation::Standard).unwrap();
            }

            if let Some(Bson::Binary(bin)) = post_data.get("id") {
                post.id = bin.to_uuid_with_representation(UuidRepresentation::Standard).unwrap();
            }
        }
        if let Some(Bson::Document(user)) = document.get("user") {
            post.user = Deserialize::deserialize(user.clone());
        }
        if let Some(Bson::Document(rating)) = document.get("rating") {
            if let Some(Bson::Int32(rating)) = rating.get("rating") {
                post.rating = *rating as usize;
            }
        }
        if let Some(Bson::Double(score)) = document.get("score") {
            println!("{}", score);
        }

        post
    }
}

/// The [messages](Action) that can be triggered on the [Posts] scene.
#[derive(Clone)]
enum PostsAction {
    /// Triggers when some posts are loaded to be displayed.
    LoadedPosts(Vec<Post>),

    /// Triggers when the given amount of images from the posts have been loaded.
    LoadedImage{ image: Vec<u8>, index: usize, limit: usize, auth: Authorization },

    /// Loads a batch of images
    LoadBatch,
    
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

/// The types a modal can have on the [Posts] scene.
#[derive(Clone)]
enum ModalType {
    /// Modal for displaying an image in the center of the screen.
    ShowingImage(Vec<u8>),
    
    /// Modal for opening a post.
    ShowingPost(usize),
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

impl Eq for ModalType { }

/// A scene that displays posts.
#[derive(Clone)]
pub struct Posts {
    /// The stack of modals.
    modals: ModalStack<ModalType>,
    
    /// The list of available posts.
    posts: Vec<Post>,

    /// The amount of loaded images
    loaded: usize,

    /// The amount of posts to be shown
    batched: usize,

    /// Tells whether images are being loaded
    loading: bool,
}

impl Posts {
    async fn get_posts_from_cursor(posts: &mut Cursor<Document>) -> Vec<Post>
    {
        let mut posts_vec = vec![];
        loop {
            let exists = posts.advance().await;
            let post = match exists {
                Ok(true) => {
                    match Document::try_from(posts.current()) {
                        Ok(document) => document,
                        _ => { break; }
                    }
                }
                _ => { break; }
            };

            let post :Post= Deserialize::deserialize(post);

            if post.drawing_id != Uuid::default() && post.user.get_id() != Uuid::default() {
                posts_vec.push(post);
            }
        }

        posts_vec
    }

    async fn get_cursor_size(cursor: &mut Cursor<Document>) -> usize
    {
        let mut size = 0;

        loop {
            let exists = cursor.advance().await;
            let post = match exists {
                Ok(true) => {
                    match Document::try_from(cursor.current()) {
                        Ok(document) => document,
                        _ => { break; }
                    }
                }
                _ => { break; }
            };

            println!("{}", post);
            size += 1;
        }

        size
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
            loaded: 0,
            batched: 0,
            loading: false,
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
                    let mut posts = match db.collection::<Result<Document, mongodb::error::Error>>("ratings").aggregate(
                        Vec::from([
                            /// Groups all ratings by the post they were made on
                            doc! {
                                "$group": {
                                    "_id": "$post_id",
                                    "ratings": {
                                        "$push": "$$ROOT"
                                    }
                                }
                            },
                            /// Filters out all posts that the currently authenticated user hasn't rated
                            doc! {
                                "$match": {
                                    "ratings": {
                                        "$elemMatch": { "user_id": user_id }
                                    }
                                }
                            },
                            /// Unwind all groups
                            doc! {
                                "$unwind": "$ratings"
                            },
                            /// Join with the corresponding post
                            doc! {
                                "$lookup": {
                                    "from": "posts",
                                    "localField": "_id",
                                    "foreignField": "id",
                                    "as": "post"
                                }
                            },
                            /// Unwind by post
                            doc! {
                                "$unwind": "$post"
                            },
                            /// Unwind by tags
                            doc! {
                                "$unwind": "$post.tags"
                            },
                            /// Restructure to keep the essential data
                            doc! {
                                "$project": {
                                    "_id": 0,
                                    "user_id": "$ratings.user_id",
                                    "tag": "$post.tags",
                                    "value": {
                                        "$subtract": [
                                            { "$divide":
                                                [
                                                    { "$subtract": ["$ratings.rating", 1.0] },
                                                    2.0
                                                ]
                                            },
                                            1.0
                                        ]
                                    },
                                }
                            },
                            /// Average score by user and tag
                            doc! {
                                "$group": {
                                    "_id": { "user_id": "$user_id", "tag": "$tag" },
                                    "score": { "$avg": "$value" }
                                }
                            },
                            /// Group by user, computing the magnitudes
                            doc! {
                                "$group": {
                                    "_id": "$_id.tag",
                                    "user_score": {
                                        "$max": {
                                            "$cond": {
                                                "if": { "$eq": ["$_id.user_id", user_id] },
                                                "then": "$score",
                                                "else": null
                                            }
                                        }
                                    },
                                    "groups": {
                                        "$push": {
                                            "user_id": "$_id.user_id",
                                            "score": "$score"
                                        }
                                    }
                                }
                            },
                            /// Unwind; this way, each tag will have access to the authenticated users score for the same tag
                            doc! {
                                "$unwind": "$groups"
                            },
                            /// Create the dot multiplication
                            doc! {
                                "$project": {
                                    "_id": 0,
                                    "user_id": "$groups.user_id",
                                    "tag": "$_id",
                                    "score": "$groups.score",
                                    "dot": {
                                        "$multiply": ["$groups.score", "$user_score"]
                                    }
                                }
                            },
                            /// Group by user and compute magnitudes and dot product
                            doc! {
                                "$group": {
                                    "_id": "$user_id",
                                    "magnitude_square": {
                                        "$sum": {
                                            "$pow": ["$score", 2]
                                        }
                                    },
                                    "dot": {
                                        "$sum": "$dot"
                                    }
                                }
                            },
                            /// Group to isolate authenticated user
                            doc! {
                                "$group": {
                                    "_id": null,
                                    "auth_user_magnitude": {
                                        "$max": {
                                            "$cond": {
                                                "if": { "$eq": [ "$_id", user_id ] },
                                                "then": { "$sqrt": "$magnitude_square" },
                                                "else": null
                                            }
                                        }
                                    },
                                    "users": {
                                        "$push": {
                                            "$cond": {
                                                "if": { "$eq": [ "$_id", user_id ] },
                                                "then": "$$REMOVE",
                                                "else": {
                                                    "_id": "$_id",
                                                    "magnitude": { "$sqrt": "$magnitude_square" },
                                                    "dot": "$dot"
                                                }
                                            }
                                        }
                                    }
                                }
                            },
                            /// Unwind to get each user again, except the authenticated one
                            doc! {
                                "$unwind": "$users"
                            },
                            /// Project to compute each user's similarity score
                            doc! {
                                "$project": {
                                    "_id": "$users._id",
                                    "score": {
                                        "$divide": [
                                            "$users.dot",
                                            {
                                                "$max": [
                                                    {
                                                        "$multiply": [
                                                            "$users.magnitude",
                                                            "$auth_user_magnitude"
                                                        ]
                                                    },
                                                    0.00001
                                                ]
                                            }
                                        ]
                                    }
                                }
                            },
                            /// Join with users to get full data
                            doc! {
                                "$lookup": {
                                    "from": "users",
                                    "localField": "_id",
                                    "foreignField": "id",
                                    "as": "user"
                                }
                            },
                            /// Unwind user data
                            doc! {
                                "$unwind": "$user"
                            },
                            /// Join with posts
                            doc! {
                                "$lookup": {
                                    "from": "posts",
                                    "localField": "_id",
                                    "foreignField": "user_id",
                                    "as": "post",
                                }
                            },
                            /// Unwind the posts
                            doc! {
                                "$unwind": "$post"
                            },
                            /// Add a field of random value
                            doc! {
                                "$addFields": {
                                    "randomValue": { "$rand": {} }
                                }
                            },
                            /// Remove those with score less than the random value
                            doc! {
                                "$match": {
                                    "$expr": {
                                        "$lte": ["$randomValue", "$score"]
                                    }
                                }
                            },
                            /// Sample 100 of those remaining
                            doc! {
                                "$sample": {
                                    "size": 100
                                }
                            },
                            /// Join with ratings
                            doc! {
                                "$lookup": {
                                    "from": "ratings",
                                    "localField": "post.id",
                                    "foreignField": "post_id",
                                    "pipeline": vec![
                                        doc! {
                                            "$match": {
                                                "$expr": {
                                                    "$eq": ["$user_id", user_id]
                                                }
                                            }
                                        }
                                    ],
                                    "as": "rating"
                                }
                            },
                            /// Unwind the rating
                            doc! {
                                "$unwind": {
                                    "path": "$rating",
                                    "preserveNullAndEmptyArrays": true
                                }
                            },
                        ]),
                        AggregateOptions::builder().allow_disk_use(true).build()
                    ).await {
                        Ok(cursor) => cursor,
                        Err(err) => {
                            return Err(Message::Error(Error::DebugError(DebugError::new(err.to_string()))));
                        }
                    };

                    let mut posts_vec = Self::get_posts_from_cursor(&mut posts).await;
                    let need = 100 - posts_vec.len();
                    let uuids :Vec<Uuid>= posts_vec.iter().map(|post| post.id).collect();

                    if posts_vec.len() < 100 {
                        let mut posts = match db.collection::<Result<Document, mongodb::error::Error>>("posts").aggregate(
                            vec![
                                doc! {
                                    "$match": {
                                        "id": {
                                            "$nin": uuids
                                        }
                                    }
                                },
                                doc! {
                                    "$sample": {
                                        "size": need as i32
                                    }
                                },
                                doc! {
                                    "$lookup": {
                                        "from": "users",
                                        "localField": "user_id",
                                        "foreignField": "id",
                                        "as": "user"
                                    }
                                },
                                doc! {
                                    "$unwind": "$user"
                                },
                                doc! {
                                    "$lookup": {
                                        "from": "posts",
                                        "localField": "id",
                                        "foreignField": "id",
                                        "as": "post"
                                    }
                                },
                                doc! {
                                    "$unwind": "$post"
                                },
                                doc! {
                                    "$lookup": {
                                        "from": "ratings",
                                        "localField": "id",
                                        "foreignField": "post_id",
                                        "pipeline": vec![
                                            doc! {
                                                "$match": {
                                                    "$expr": {
                                                        "$eq": [ "$user_id", user_id ]
                                                    }
                                                }
                                            }
                                        ],
                                        "as": "rating"
                                    }
                                },
                                doc! {
                                    "$unwind": {
                                        "path": "$rating",
                                        "preserveNullAndEmptyArrays": true
                                    }
                                }
                            ],
                            AggregateOptions::builder().allow_disk_use(true).build()
                        ).await {
                            Ok(cursor) => cursor,
                            Err(err) => {
                                return Err(Message::Error(Error::DebugError(DebugError::new(err.to_string()))));
                            }
                        };

                        let mut second_post_set = Self::get_posts_from_cursor(&mut posts).await;
                        posts_vec.append(&mut second_post_set);
                    }

                    Ok(posts_vec)
                },
                |posts| {
                    Message::DoAction(Box::new(PostsAction::LoadedPosts(posts.unwrap())))
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
            PostsAction::LoadedImage { image, index, limit, auth } => {
                let post = self.posts.get_mut(*index).unwrap();
                post.image = image.clone();

                let index = index.clone() + 1;
                let limit = limit.clone();

                if index == limit {
                    self.loaded = limit;
                    self.loading = false;
                    Command::none()
                } else {
                    let auth = auth.clone();

                    let post_user_id = self.posts[index].user.get_id().clone();
                    let post_id = self.posts[index].id.clone();
                    let client = UserAuthDefaultClient::new(auth.clone());

                    Command::perform(
                        async move {
                            let mut data = vec![];

                            match files::download(
                                &client,
                                &DownloadArg::new(format!("/{}/{}.webp", post_user_id, post_id)),
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
                        move |data| Message::DoAction(Box::new(PostsAction::LoadedImage {
                            image: data,
                            index,
                            limit,
                            auth
                        }))
                    )
                }
            }
            PostsAction::LoadBatch => {
                if self.loading {
                    Command::none()
                } else {
                    if self.loaded == self.posts.len() {
                        Command::none()
                    } else {
                        self.loading = true;
                        let start = self.loaded;
                        let total = self.posts.len();
                        self.batched += 10.min(total - start);

                        let post_user_id = self.posts[start].user.get_id().clone();
                        let post_id = self.posts[start].id.clone();

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
                                    &DownloadArg::new(format!("/{}/{}.webp", post_user_id, post_id)),
                                    None,
                                    None
                                ) {
                                    Ok(Ok(result)) => {
                                        let mut read = result.body.unwrap();

                                        let _ = io::copy(read.deref_mut(), &mut data).unwrap();
                                    },
                                    _ => {}
                                }

                                (data, auth)
                            },
                            move |(data, auth)| Message::DoAction(Box::new(PostsAction::LoadedImage {
                                image: data,
                                index: start,
                                limit: (start + 10).min(total),
                                auth
                            }))
                        )
                    }
                }
            }
            PostsAction::ToggleModal(modal) => {
                self.modals.toggle_modal(modal.clone());
                Command::none()
            }
            PostsAction::RatePost { post_index, rating } => {
                let post :Option<&mut Post>= self.posts.get_mut(*post_index);
                if let Some(post) = post {
                    let rating = rating.clone();
                    post.rating = rating;

                    let post_id = post.id;
                    let user_id = globals.get_user().unwrap().get_id();
                    let db = globals.get_db().unwrap();

                    if rating > 0 {
                        Command::perform(
                            async move {
                                MongoRequest::send_requests(
                                    db,
                                    vec![
                                        MongoRequest::new(
                                            "ratings".into(),
                                            MongoRequestType::Update {
                                                filter: doc!{
                                                    "post_id": post_id,
                                                    "user_id": user_id
                                                },
                                                update: doc!{
                                                    "$set": {
                                                        "rating": rating as i32
                                                    }
                                                },
                                                options: Some(UpdateOptions::builder()
                                                    .upsert(Some(true))
                                                    .build()
                                                )
                                            }
                                        )
                                    ]
                                ).await
                            },
                            |result| {
                                match result {
                                    Ok(_) => Message::None,
                                    Err(message) => message
                                }
                            }
                        )
                    } else {
                        Command::perform(
                            async move {
                                MongoRequest::send_requests(
                                    db,
                                    vec![
                                        MongoRequest::new(
                                            "ratings".into(),
                                            MongoRequestType::Delete {
                                                filter: doc! {
                                                    "user_id": user_id,
                                                    "post_id": post_id
                                                },
                                                options: None
                                            }
                                        )
                                    ]
                                ).await
                            },
                            |result| {
                                match result {
                                    Ok(_) => Message::None,
                                    Err(message) => message
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
                                Text::new(post.user.get_username()).size(20.0).into(),
                                Text::new(post.description.clone()).into()
                            ]),
                            Image::new(
                                Handle::from_memory(post.image.clone())
                            ).width(Length::Shrink)
                        )
                            .padding(40)
                            .on_click_image(Message::DoAction(Box::new(PostsAction::ToggleModal(
                                ModalType::ShowingImage(post.image.clone())
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
                if viewport.relative_offset().y == 1.0 {
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

                    Row::with_children(
                        vec![
                            Closeable::new(Image::new(
                                Handle::from_memory(post.image.clone())
                            ).width(Length::Shrink))
                                .width(Length::FillPortion(3))
                                .height(Length::Fill)
                                .style(crate::theme::closeable::Closeable::SpotLight)
                                .on_click(Message::DoAction(Box::new(PostsAction::ToggleModal(ModalType::ShowingImage(post.image.clone())))))
                                .into(),
                            Closeable::new(
                                Column::with_children(vec![
                                    Text::new(post.user.get_username())
                                        .size(20.0)
                                        .into(),
                                    Text::new(post.description.clone())
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
                                        .value(post.rating)
                                        .into()
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