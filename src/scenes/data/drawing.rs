use crate::utils::serde::{Deserialize, Serialize};
use iced::widget::text_editor::{Action, Content};
use mongodb::bson::{doc, Document};
use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};

/// The types of the modals that can be opened.
#[derive(Clone, Eq)]
pub enum ModalTypes {
    /// A prompt where the user can write data for a post they are creating.
    PostPrompt,

    /// A screen that blocks user interaction.
    WaitScreen(String),
}

impl ModalTypes {
    pub fn is_post_prompt(&self) -> bool {
        match self {
            Self::PostPrompt => true,
            _ => false,
        }
    }

    pub fn is_wait_screen(&self) -> bool {
        match self {
            Self::WaitScreen(_) => true,
            _ => false,
        }
    }
}

impl PartialEq for ModalTypes {
    fn eq(&self, other: &Self) -> bool {
        match self {
            Self::PostPrompt => other.is_post_prompt(),
            Self::WaitScreen(_) => other.is_wait_screen(),
        }
    }
}

/// Data for a post tag.
#[derive(Default, Clone)]
pub struct Tag {
    /// The name of the tag.
    name: String,

    /// The number of posts the tag has been used in.
    uses: u32,
}

impl Tag {
    /// Reduces the name of a new tag to a base tag form.
    pub fn reduced(mut self) -> Self {
        self.name = self
            .name
            .to_ascii_lowercase()
            .split_whitespace()
            .collect::<Vec<&str>>()
            .join(" ");

        self
    }

    pub fn get_name(&self) -> &String {
        &self.name
    }
}

impl PartialEq for Tag {
    fn eq(&self, other: &Self) -> bool {
        self.name.clone() == other.name
    }
}

impl Eq for Tag {}

impl Hash for Tag {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}

impl Display for Tag {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&*format!("{}({})", self.name, self.uses))
    }
}

impl Serialize<Document> for Tag {
    fn serialize(&self) -> Document {
        doc![
            "name": self.name.clone(),
            "uses": self.uses as i32
        ]
    }
}

impl Deserialize<Document> for Tag {
    fn deserialize(document: &Document) -> Self
    where
        Self: Sized,
    {
        let mut tag = Tag {
            name: "".into(),
            uses: 0,
        };

        if let Ok(name) = document.get_str("name") {
            tag.name = name.into();
        }
        if let Ok(uses) = document.get_i64("uses") {
            tag.uses = uses as u32;
        }

        tag
    }
}

/// The data of a post.
#[derive(Default)]
pub struct PostData {
    /// The description of the post.
    description: Content,

    /// The list of tags the user has chosen for the post.
    post_tags: Vec<Tag>,

    /// A list of all tags that have been applied to a post.
    all_tags: Vec<Tag>,

    /// The current input the user has written for a new tag.
    tag_input: String,
}

/// Possible updates to a new post data.
#[derive(Clone)]
pub enum UpdatePostData {
    Description(Action),
    NewTag(String),
    SelectedTag(Tag),
    AllTags(Vec<Tag>),
    TagInput(String),
    RemoveTag(usize),
}

impl PostData {
    /// Updates the new post data.
    pub fn update(&mut self, update: UpdatePostData) {
        match update {
            UpdatePostData::Description(action) => self.description.perform(action),
            UpdatePostData::NewTag(name) => {
                let tag = Tag { name, uses: 0 }.reduced();

                if self
                    .post_tags
                    .iter()
                    .find(|pos_tag| **pos_tag == tag)
                    .is_none()
                {
                    self.post_tags.push(tag.clone());
                }
                self.tag_input = "".into();
            }
            UpdatePostData::SelectedTag(tag) => {
                if self
                    .post_tags
                    .iter()
                    .find(|pos_tag| **pos_tag == tag)
                    .is_none()
                {
                    self.post_tags.push(tag);
                }
                self.tag_input = "".into();
            }
            UpdatePostData::AllTags(tags) => self.all_tags = tags,
            UpdatePostData::TagInput(tag_input) => self.tag_input = tag_input,
            UpdatePostData::RemoveTag(index) => {
                self.post_tags.remove(index);
            }
        }
    }

    pub fn get_description(&self) -> &Content {
        &self.description
    }

    pub fn get_post_tags(&self) -> &Vec<Tag> {
        &self.post_tags
    }

    pub fn get_all_tags(&self) -> &Vec<Tag> {
        &self.all_tags
    }

    pub fn get_tag_input(&self) -> &String {
        &self.tag_input
    }

    pub fn no_tags(&self) -> bool {
        self.all_tags.is_empty()
    }

    pub fn set_description(&mut self, description: impl Into<Content>) {
        self.description = description.into();
    }

    pub fn set_post_tags(&mut self, post_tags: impl Into<Vec<Tag>>) {
        self.post_tags = post_tags.into();
    }

    pub fn set_tag_input(&mut self, tag_input: impl Into<String>) {
        self.tag_input = tag_input.into();
    }
}

/// The mode in which the progress will be saved.
#[derive(Debug, Clone, Copy)]
pub enum SaveMode {
    /// Saves the canvas locally.
    Offline,

    /// Saves the canvas in a database.
    Online,
}
