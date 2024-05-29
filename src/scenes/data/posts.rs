use crate::scene::Message;
use crate::scenes::data::auth::User;
use crate::scenes::posts::PostsMessage;
use crate::utils::serde::{Deserialize, Serialize};
use iced::widget::image::Handle;
use image::{DynamicImage, RgbaImage};
use mongodb::bson::{doc, Bson, Document, Uuid, UuidRepresentation};
use std::collections::HashMap;
use std::sync::Arc;

/// An image represented by pixel data.
#[derive(Debug, Clone)]
pub struct PixelImage {
    /// The width of the [PixelImage].
    width: u32,

    /// The height of the [PixelImage].
    height: u32,

    /// The pixel data.
    data: Vec<u8>,
}

impl PixelImage {
    /// Initialize a [PixelImage].
    pub fn new(width: u32, height: u32, data: Vec<u8>) -> Self {
        PixelImage {
            width,
            height,
            data,
        }
    }

    pub fn get_width(&self) -> u32 {
        self.width
    }

    pub fn get_height(&self) -> u32 {
        self.height
    }

    pub fn get_data(&self) -> &Vec<u8> {
        &self.data
    }
}

impl From<DynamicImage> for PixelImage {
    fn from(value: DynamicImage) -> Self {
        Self::new(value.width(), value.height(), value.to_rgba8().to_vec())
    }
}

impl Into<DynamicImage> for PixelImage {
    fn into(self) -> DynamicImage {
        DynamicImage::ImageRgba8(RgbaImage::from_raw(self.width, self.height, self.data).unwrap())
    }
}

/// A comment on a post.
#[derive(Clone)]
pub struct Comment {
    /// The id of the [Comment].
    id: Uuid,

    /// The [User] who sent the [Comment].
    user: User,

    /// The content of the [Comment].
    content: String,

    /// The id of the [Comment] this is a reply to.
    reply_to: Option<Uuid>,

    /// The input of a reply the user is currently writing.
    reply_input: String,

    /// The position of the [Comment] this is a reply to.
    parent: Option<(usize, usize)>,

    /// The index of the line with the replies to this comment in the comments vector of the [Post].
    /// Is None if this comments replies are not loaded from the database.
    replies: Option<usize>,

    /// The index of the reply that is currently opened(absolute).
    open_reply: Option<usize>,
}

impl Comment {
    pub fn new_comment(id: Uuid, user: User, content: impl Into<String>) -> Self {
        Comment {
            id,
            user,
            content: content.into(),
            ..Default::default()
        }
    }

    pub fn new_reply(
        id: Uuid,
        user: User,
        content: impl Into<String>,
        reply_to: impl Into<Option<Uuid>>,
        parent: impl Into<Option<(usize, usize)>>,
    ) -> Self {
        Comment {
            id,
            user,
            content: content.into(),
            reply_to: reply_to.into(),
            parent: parent.into(),
            ..Default::default()
        }
    }

    pub fn get_id(&self) -> &Uuid {
        &self.id
    }

    pub fn get_user(&self) -> &User {
        &self.user
    }

    pub fn get_content(&self) -> &String {
        &self.content
    }

    pub fn get_parent(&self) -> &Option<(usize, usize)> {
        &self.parent
    }

    pub fn get_reply_input(&self) -> &String {
        &self.reply_input
    }

    pub fn get_replies(&self) -> &Option<usize> {
        &self.replies
    }

    pub fn get_open_reply(&self) -> &Option<usize> {
        &self.open_reply
    }

    pub fn replies_not_loaded(&self) -> bool {
        self.replies.is_none()
    }

    pub fn set_parent(&mut self, parent: impl Into<Option<(usize, usize)>>) {
        self.parent = parent.into();
    }

    pub fn set_reply_input(&mut self, reply_input: impl Into<String>) {
        self.reply_input = reply_input.into();
    }

    pub fn set_replies(&mut self, replies: impl Into<Option<usize>>) {
        self.replies = replies.into();
    }

    pub fn set_open_reply(&mut self, open_reply: impl Into<Option<usize>>) {
        self.open_reply = open_reply.into()
    }
}

impl Default for Comment {
    fn default() -> Self {
        Comment {
            id: Uuid::default(),
            user: User::default(),
            content: Default::default(),
            reply_to: None,
            reply_input: Default::default(),
            parent: None,
            replies: None,
            open_reply: None,
        }
    }
}

impl Serialize<Document> for Comment {
    fn serialize(&self) -> Document {
        if let Some(id) = self.reply_to {
            doc! {
                "id": self.id,
                "user_id": self.user.get_id(),
                "content": self.content.clone(),
                "reply_to": id
            }
        } else {
            doc! {
                "id": self.id,
                "user_id": self.user.get_id(),
                "content": self.content.clone(),
            }
        }
    }
}

impl Deserialize<Document> for Comment {
    fn deserialize(document: &Document) -> Self
    where
        Self: Sized,
    {
        let mut comment = Comment::default();

        if let Some(Bson::Binary(bin)) = document.get("id") {
            comment.id = bin
                .to_uuid_with_representation(UuidRepresentation::Standard)
                .unwrap();
        }
        if let Some(Bson::Document(user)) = document.get("user") {
            comment.user = Deserialize::deserialize(user);
        }
        if let Some(Bson::String(content)) = document.get("content") {
            comment.content = content.clone();
        }

        comment
    }
}

#[derive(Clone)]
pub enum CommentMessage {
    /// Opens a [Comment].
    Open {
        post: usize,
        position: (usize, usize),
    },

    /// Closes a [Comment] and all the replies to it that are opened.
    Close {
        post: usize,
        position: (usize, usize),
    },

    /// Updates the content of a [Comment].
    UpdateInput {
        post: usize,
        position: Option<(usize, usize)>,
        input: String,
    },

    /// Adds a reply to a [Comment].
    Add {
        post: usize,
        parent: Option<(usize, usize)>,
    },

    /// Loads the replies for a [Comment].
    Load {
        post: usize,
        parent: Option<(usize, usize)>,
    },

    /// Loads comments that are replies to another comment.
    Loaded {
        post: usize,
        parent: Option<(usize, usize)>,
        comments: Vec<Comment>,
        tab: PostTabs,
    },
}

impl Into<Message> for CommentMessage {
    fn into(self) -> Message {
        PostsMessage::CommentMessage(self).into()
    }
}

/// The data for a loaded post.
#[derive(Clone)]
pub struct Post {
    /// The id of the post.
    id: Uuid,

    /// The description of the [Post].
    description: String,

    /// The tags of the [Post].
    tags: Vec<String>,

    /// The [User] this [Post] belongs to.
    user: User,

    /// The rating of the post.
    rating: usize,

    /// The input of the comment the user is currently writing.
    comment_input: String,

    /// The comments of the post.
    /// None if they haven't been loaded yet.
    comments: Vec<Vec<Comment>>,

    /// The index of the comment that is currently opened.
    open_comment: Option<usize>,
}

impl Post {
    pub fn get_id(&self) -> Uuid {
        self.id
    }

    pub fn get_user(&self) -> &User {
        &self.user
    }

    pub fn get_description(&self) -> &String {
        &self.description
    }

    pub fn get_comments(&self) -> &Vec<Vec<Comment>> {
        &self.comments
    }

    pub fn get_comments_mut(&mut self) -> &mut Vec<Vec<Comment>> {
        &mut self.comments
    }

    pub fn get_comment_input(&self) -> &String {
        &self.comment_input
    }

    pub fn get_open_comment(&self) -> &Option<usize> {
        &self.open_comment
    }

    pub fn get_rating(&self) -> &usize {
        &self.rating
    }

    pub fn get_image(&self, images: &HashMap<Uuid, Arc<Vec<u8>>>) -> Option<Arc<Vec<u8>>> {
        images.get(&self.id).map(Clone::clone)
    }

    pub fn set_rating(&mut self, rating: impl Into<usize>) {
        self.rating = rating.into();
    }

    pub fn set_comment_input(&mut self, comment_input: impl Into<String>) {
        self.comment_input = comment_input.into();
    }

    pub fn set_open_comment(&mut self, open_comment: impl Into<Option<usize>>) {
        self.open_comment = open_comment.into();
    }
}

impl Default for Post {
    fn default() -> Self {
        Post {
            id: Uuid::default(),
            description: "".into(),
            tags: vec![],
            user: User::default(),
            rating: 0,
            comment_input: Default::default(),
            comments: vec![],
            open_comment: None,
        }
    }
}

impl Deserialize<Document> for Post {
    fn deserialize(document: &Document) -> Self
    where
        Self: Sized,
    {
        let mut post: Post = Default::default();

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

            if let Some(Bson::Binary(bin)) = post_data.get("id") {
                post.id = bin
                    .to_uuid_with_representation(UuidRepresentation::Standard)
                    .unwrap();
            }
        }
        if let Some(Bson::Document(user)) = document.get("user") {
            post.user = User::deserialize(user);
        }
        if let Some(Bson::Document(rating)) = document.get("rating") {
            if let Some(Bson::Int32(rating)) = rating.get("rating") {
                post.rating = *rating as usize;
            }
        }

        post
    }
}

/// A list of posts to be displayed.
#[derive(Clone)]
pub struct PostList {
    /// Ids of posts in this list.
    posts: Vec<Post>,

    /// Number of loaded posts.
    loaded: usize,
}

impl PostList {
    pub fn new(posts: Vec<Post>) -> Self {
        PostList { posts, loaded: 0 }
    }

    /// Load the next batch of images.
    pub fn load_batch(&mut self) -> &[Post] {
        let start = self.loaded;
        let total = self.posts.len();

        self.loaded += 10.min(total - start);

        &self.posts[start..self.loaded]
    }

    /// Change the rating given to a post by the authenticated user.
    pub fn rate_post(&mut self, index: usize, rating: usize) -> (Uuid, Option<usize>) {
        let post = &mut self.posts[index];

        let rating = rating.clone();
        post.set_rating(rating);

        (post.get_id(), if rating == 0 { None } else { Some(rating) })
    }

    /// Opens the given comment. If the replies haven't been loaded yet, returns true.
    pub fn open_comment(&mut self, post_index: usize, line: usize, index: usize) -> bool {
        let post = &mut self.posts[post_index];
        if let Some((parent_line, parent_index)) = post.comments[line][index].parent {
            post.comments[parent_line][parent_index].open_reply = Some(index);
        } else {
            post.set_open_comment(index);
        }

        post.comments[line][index].replies_not_loaded()
    }

    /// Closes the given comment.
    pub fn close_comment(&mut self, post_index: usize, line: usize, index: usize) {
        let post = &mut self.posts[post_index];

        let mut position = if line != 0 {
            post.comments[line][index].parent.clone()
        } else {
            post.open_comment = None;
            Some((line, index))
        };

        while let Some((line, index)) = position {
            let reply_line = post.comments[line][index].replies.clone();
            let reply_index = post.comments[line][index].open_reply.clone();
            position = reply_line.zip(reply_index);

            post.comments[line][index].open_reply = None;
        }
    }

    /// Updates the reply input field of the given comment.
    pub fn update_input(
        &mut self,
        post_index: usize,
        position: Option<(usize, usize)>,
        input: String,
    ) {
        let post = &mut self.posts[post_index];

        if let Some((line, index)) = position {
            post.comments[line][index].reply_input = input;
        } else {
            post.comment_input = input;
        }
    }

    /// Adds a reply to the given comment. Returns the reply data serialized.
    pub fn add_reply(
        &mut self,
        user: User,
        post_index: usize,
        line: usize,
        index: usize,
    ) -> Document {
        let post = &mut self.posts[post_index];
        let parent = &post.comments[line][index];

        let comment = Comment::new_reply(
            Uuid::new(),
            user,
            parent.get_reply_input().clone(),
            parent.get_id().clone(),
            (line, index),
        );

        let document = comment.serialize();

        post.comments[line][index].reply_input = String::from("");

        let line = post.comments[line][index].replies.unwrap();
        post.comments[line].push(comment);

        document
    }

    /// Adds a comment to the given post. Returns the comment data serialized.
    pub fn add_comment(&mut self, user: User, post_index: usize) -> Document {
        let post = &mut self.posts[post_index];

        let comment = Comment::new_comment(Uuid::new(), user, post.comment_input.clone());

        let mut document = comment.serialize();

        post.comment_input = String::from("");
        post.comments[0].push(comment);

        document.insert("post_id", post.id);
        document
    }

    /// Returns the load comments request mongo document.
    pub fn load_comments(&mut self, post_index: usize, parent: Option<(usize, usize)>) -> Document {
        if let Some((line, index)) = parent {
            doc! {
                "reply_to": self.posts[post_index].comments[line][index].id
            }
        } else {
            doc! {
                "post_id": self.posts[post_index].id
            }
        }
    }

    /// Adds a new set of comments that were loaded.
    pub fn loaded_comments(
        &mut self,
        post_index: usize,
        parent: Option<(usize, usize)>,
        comments: Vec<Comment>,
    ) {
        let post = &mut self.posts[post_index];
        post.comments.push(comments);
        let new_line = post.comments.len() - 1;

        for comment in &mut post.comments[new_line] {
            comment.parent = parent;
        }

        if let Some((line, index)) = parent {
            post.comments[line][index].replies = Some(new_line);
        }
    }

    /// Returns true if the given post has already loaded the main comments.
    pub fn has_loaded_comments(&self, post_index: usize) -> bool {
        self.posts[post_index].comments.len() > 0
    }

    /// Returns the post at the given index.
    pub fn get_post(&self, index: usize) -> Option<&Post> {
        self.posts.get(index)
    }

    /// Returns a list of the loaded posts.
    pub fn get_loaded_posts(&self) -> impl IntoIterator<Item = (&Post, usize)> {
        self.posts[..self.loaded]
            .iter()
            .enumerate()
            .map(|val| (val.1, val.0))
    }

    /// Tells whether the images have all been loaded.
    pub fn done_loading(&self) -> bool {
        self.loaded == self.posts.len()
    }
}

/// The types a modal can have on the [Posts] scene.
#[derive(Clone)]
pub enum ModalType {
    /// Modal for displaying an image in the center of the screen.
    ShowingImage(Handle),

    /// Modal for opening a post.
    ShowingPost(usize),

    /// Modal for reporting a post.
    ShowingReport(usize),
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
            _ => false,
        }
    }

    /// Checks if its value is [ShowingReport](ModalType::ShowingReport).
    fn is_showing_report(&self) -> bool {
        match self {
            ModalType::ShowingReport(_) => true,
            _ => false,
        }
    }
}

impl PartialEq for ModalType {
    fn eq(&self, other: &Self) -> bool {
        match self {
            ModalType::ShowingImage(_) => other.is_showing_image(),
            ModalType::ShowingPost(_) => other.is_showing_post(),
            ModalType::ShowingReport(_) => other.is_showing_report(),
        }
    }
}

impl Eq for ModalType {}

/// The tabs the posts page is split into.
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum PostTabs {
    /// Posts generated from comparing the users ratings to other users.
    Recommended,

    /// Posts generated from tag selection.
    Filtered,

    /// Posts generated by profile lookup.
    Profile,
}
