use std::fs;
use mongodb::bson::{Bson, doc, Document, Uuid, UuidRepresentation};
use crate::scenes::data::auth::User;
use crate::serde::{Deserialize, Serialize};

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
    open_reply: Option<usize>
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
        parent: impl Into<Option<(usize, usize)>>
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
            open_reply: None
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
    fn deserialize(document: &Document) -> Self where Self: Sized {
        let mut comment = Comment::default();

        if let Some(Bson::Binary(bin)) = document.get("id") {
            comment.id = bin.to_uuid_with_representation(UuidRepresentation::Standard).unwrap();
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
    Open{ post: usize, position: (usize, usize) },

    /// Closes a [Comment] and all the replies to it that are opened.
    Close{ post: usize, position: (usize, usize) },

    /// Updates the content of a [Comment].
    UpdateInput{ post: usize, position: Option<(usize, usize)>, input: String },

    /// Adds a reply to a [Comment].
    Add { post: usize, parent: Option<(usize, usize)> },

    /// Loads the replies for a [Comment].
    Load{ post: usize, parent: Option<(usize, usize)> },

    /// Loads comments that are replies to another comment.
    Loaded{ post: usize, parent: Option<(usize, usize)>, comments: Vec<Comment> },
}

/// The data for a loaded post.
#[derive(Clone)]
pub struct Post {
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

    /// The rating of the post.
    rating: usize,

    /// The input of the comment the user is currently writing.
    comment_input: String,

    /// The comments of the post.
    /// None if they haven't been loaded yet.
    comments: Vec<Vec<Comment>>,

    /// The index of the comment that is currently opened.
    open_comment: Option<usize>
}

impl Post {
    pub fn get_id(&self) -> &Uuid {
        &self.id
    }

    pub fn get_user(&self) -> &User {
        &self.user
    }

    pub fn get_image(&self) -> &Vec<u8> {
        &self.image
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

    pub fn set_image(&mut self, image: Vec<u8>) {
        self.image = image;
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
            image: fs::read("./src/images/loading.png").unwrap(),
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
    fn deserialize(document: &Document) -> Self where Self: Sized {
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

            if let Some(Bson::Binary(bin)) = post_data.get("id") {
                post.id = bin.to_uuid_with_representation(UuidRepresentation::Standard).unwrap();
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

/// The types a modal can have on the [Posts] scene.
#[derive(Clone)]
pub enum ModalType {
    /// Modal for displaying an image in the center of the screen.
    ShowingImage(Vec<u8>),

    /// Modal for opening a post.
    ShowingPost(usize),

    /// Modal for reporting a post.
    ShowingReport(usize)
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
            ModalType::ShowingImage(_) => {
                other.is_showing_image()
            }
            ModalType::ShowingPost(_) => {
                other.is_showing_post()
            }
            ModalType::ShowingReport(_) => {
                other.is_showing_report()
            }
        }
    }
}

impl Eq for ModalType { }