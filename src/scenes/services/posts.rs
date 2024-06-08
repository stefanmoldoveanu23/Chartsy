use std::sync::Arc;

use iced::{
    advanced::widget::Text,
    widget::{
        tooltip::Position, Button, Column, Container, Row, Scrollable, Space, TextInput, Tooltip,
    },
    Alignment, Element, Length, Pixels, Renderer, Size,
};
use image::{load_from_memory_with_format, ImageFormat};
use mongodb::{bson::Uuid, Database};

use crate::{
    database, debug_message,
    scene::{Globals, Message},
    scenes::{
        data::{
            auth::{Role, User},
            posts::{CommentMessage, ModalType, Post, PostList, PostTabs},
        },
        posts::PostsMessage,
    },
    utils::{
        cache::{Cache, PixelImage},
        errors::Error,
        icons::{Icon, ICON},
        theme::{self, Theme},
    },
    widgets::{Card, Closeable, PostSummary, Rating, WaitPanel},
};

pub async fn delete_post(id: Uuid, globals: &Globals) -> Result<(), Error> {
    let user_id = globals
        .get_user()
        .ok_or(debug_message!("User is not logged in.").into())?
        .get_id();

    database::posts::delete_post(id, globals).await?;

    database::base::delete_data(format!("/{}/{}.webp", user_id, id)).await
}

pub async fn load_post(ids: (Uuid, Uuid)) -> Result<Arc<PixelImage>, Error> {
    let webp = database::base::download_file(format!("/{}/{}.webp", ids.1, ids.0)).await?;

    load_from_memory_with_format(webp.as_slice(), ImageFormat::WebP)
        .map_err(|err| debug_message!("{}", err).into())
        .map(|image| Arc::new(image.into()))
}

pub async fn load_profile_picture(id: Option<Uuid>) -> Result<Arc<PixelImage>, Error> {
    let webp = database::base::download_file(
        id.map(|id| format!("/{}/profile_picture.webp", id))
            .unwrap_or(String::from("/default_profile_picture.webp")),
    )
    .await?;

    load_from_memory_with_format(webp.as_slice(), ImageFormat::WebP)
        .map_err(|err| debug_message!("{}", err).into())
        .map(|image| Arc::new(image.into()))
}

pub async fn generate_recommended(db: Database, id: Uuid) -> Result<Vec<Post>, Error> {
    let mut posts = match database::posts::get_recommendations(&db, id).await {
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
        let mut posts_random = match database::posts::get_random_posts(&db, need, id, uuids).await {
            Ok(posts) => posts,
            Err(err) => {
                return Err(err);
            }
        };

        posts.append(&mut posts_random);
    }

    Ok(posts)
}

pub fn image_profile_link<'a>(
    post: &'a Post,
    cache: &Cache,
) -> Element<'a, Message, Theme, Renderer> {
    Tooltip::new(
        Button::new(cache.get_element(
            post.get_user().get_id(),
            Size::new(Length::Fixed(50.0), Length::Fixed(50.0)),
            Size::new(Length::Fixed(50.0), Length::Fixed(50.0)),
            Some(Pixels(5.0)),
        ))
        .on_press(PostsMessage::OpenProfile(post.get_user().clone()).into())
        .style(iced::widget::button::text),
        Text::new(format!("{}'s profile", post.get_user().get_user_tag())),
        Position::FollowCursor,
    )
    .into()
}

pub fn tag_profile_link<'a>(post: &'a Post) -> Element<'a, Message, Theme, Renderer> {
    Tooltip::new(
        Button::new(
            Text::new(format!("@{}", post.get_user().get_user_tag()))
                .size(15.0)
                .style(theme::text::gray),
        )
        .style(iced::widget::button::text)
        .on_press(PostsMessage::OpenProfile(post.get_user().clone()).into()),
        Text::new(format!("{}'s profile", post.get_user().get_user_tag())),
        Position::FollowCursor,
    )
    .into()
}

pub fn report_button<'a>(index: usize) -> Element<'a, Message, Theme, Renderer> {
    Tooltip::new(
        Button::new(
            Text::new(Icon::Report.to_string())
                .font(ICON)
                .style(theme::text::danger)
                .size(30.0),
        )
        .on_press(PostsMessage::ToggleModal(ModalType::ShowingReport(index)).into())
        .padding(0.0)
        .style(iced::widget::button::text),
        Text::new("Report post"),
        Position::FollowCursor,
    )
    .into()
}

pub fn delete_button<'a>(
    post: &Post,
    user_id: Uuid,
    user_role: &Role,
) -> Element<'a, Message, Theme, Renderer> {
    if *user_role == Role::Admin || user_id == post.get_user().get_id() {
        Tooltip::new(
            Button::new(
                Text::new(Icon::Trash.to_string())
                    .font(ICON)
                    .style(theme::text::danger)
                    .size(30),
            )
            .on_press(PostsMessage::DeletePost(post.get_id()).into())
            .padding(0.0)
            .style(iced::widget::button::text),
            Text::new("Delete post"),
            Position::FollowCursor,
        )
        .into()
    } else {
        Space::with_height(Length::Shrink).into()
    }
}

pub fn generate_post_list<'a>(
    tab: PostTabs,
    list: &'a PostList,
    user: &User,
    cache: Cache,
) -> Container<'a, Message, Theme, Renderer> {
    let user_id = user.get_id();
    let user_role = user.get_role();

    Container::new(
        Scrollable::new(
            Column::with_children(
                list.get_loaded_posts()
                    .into_iter()
                    .map(|(post, index)| {
                        PostSummary::<Message, Theme, Renderer>::new(
                            Row::with_children(vec![
                                image_profile_link(post, &cache),
                                Column::with_children(vec![
                                    tag_profile_link(post),
                                    Text::new(post.get_user().get_username()).size(20.0).into(),
                                    Text::new(post.get_description().clone()).into(),
                                ])
                                .into(),
                                Space::with_width(Length::Fill).into(),
                                Column::with_children(vec![
                                    report_button(index),
                                    delete_button(post, user_id, user_role),
                                ])
                                .into(),
                            ])
                            .spacing(10.0),
                            cache.get_element(
                                post.get_id(),
                                Size::new(Length::Shrink, Length::Shrink),
                                Size::new(Length::Fixed(800.0), Length::Fixed(600.0)),
                                None,
                            ),
                        )
                        .padding(40)
                        .on_click_image(Into::<Message>::into(PostsMessage::ToggleModal(
                            ModalType::ShowingImage(post.get_id()),
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
            if viewport.relative_offset().y == 1.0 && !list.done_loading() {
                Some(PostsMessage::LoadBatch(tab).into())
            } else {
                None
            }
        })
        .width(Length::Fill),
    )
    .padding([20.0, 0.0, 0.0, 0.0])
}

pub fn generate_show_image<'a>(id: Uuid, cache: Cache) -> Element<'a, Message, Theme, Renderer> {
    Closeable::new(cache.get_element(
        id,
        Size::new(Length::Shrink, Length::Shrink),
        Size::new(Length::Fixed(800.0), Length::Fixed(600.0)),
        None,
    ))
    .width(Length::Fill)
    .height(Length::Fill)
    .on_close(
        Into::<Message>::into(PostsMessage::ToggleModal(ModalType::ShowingImage(id))),
        40.0,
    )
    .style(theme::closeable::Closeable::SpotLight)
    .into()
}

fn comment_input<'a>(post: &'a Post, post_index: usize) -> Column<'a, Message, Theme, Renderer> {
    Column::with_children(vec![Row::with_children(vec![
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
    .into()])
}

fn comment_with_children<'a>(
    post: &'a Post,
    post_index: usize,
    line: usize,
    index: usize,
) -> Element<'a, Message, Theme, Renderer> {
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
            Text::new(post.get_comments()[line][index].get_content().clone()).into(),
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

fn comment_without_children<'a>(
    post: &'a Post,
    post_index: usize,
    line: usize,
) -> Element<'a, Message, Theme, Renderer> {
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

pub fn generate_comment_chain<'a>(
    post: &'a Post,
    post_index: usize,
) -> Element<'a, Message, Theme, Renderer> {
    let mut comment_chain = comment_input(post, post_index);

    let mut position = if let Some(index) = post.get_open_comment() {
        Ok((0usize, *index))
    } else {
        Err(0usize)
    };

    let mut done = false;
    while !done {
        comment_chain = comment_chain.push(match position {
            Ok((line, index)) => {
                position =
                    if let Some(reply_index) = post.get_comments()[line][index].get_open_reply() {
                        Ok((
                            post.get_comments()[line][index].get_replies().unwrap(),
                            *reply_index,
                        ))
                    } else {
                        Err(post.get_comments()[line][index]
                            .get_replies()
                            .unwrap_or(post.get_comments().len()))
                    };

                comment_with_children(post, post_index, line, index)
            }
            Err(line) => {
                done = true;

                if line >= post.get_comments().len() {
                    WaitPanel::new("Loading...")
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .into()
                } else {
                    comment_without_children(post, post_index, line)
                }
            }
        });
    }

    comment_chain.into()
}

pub fn generate_show_post<'a>(
    post: &'a Post,
    post_index: usize,
    cache: &Cache,
) -> Element<'a, Message, Theme, Renderer> {
    let comment_chain = generate_comment_chain(post, post_index);

    Row::with_children(vec![
        Closeable::new(cache.get_element(
            post.get_id(),
            Size::new(Length::Shrink, Length::Shrink),
            Size::new(Length::Fixed(800.0), Length::Fixed(600.0)),
            None
        ))
        .width(Length::FillPortion(3))
        .height(Length::Fill)
        .style(theme::closeable::Closeable::SpotLight)
        .on_click(Into::<Message>::into(PostsMessage::ToggleModal(
            ModalType::ShowingImage(post.get_id()),
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
            comment_chain,
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

pub fn generate_show_report<'a>(
    post_index: usize,
    report_input: String,
) -> Element<'a, Message, Theme, Renderer> {
    Closeable::new(
        Card::new(
            Text::new("Report post").size(20.0),
            Column::with_children(vec![
                TextInput::new("Give a summary of the issue...", &*report_input.clone())
                    .on_input(|value| PostsMessage::UpdateReportInput(value.clone()).into())
                    .into(),
                Container::new(
                    Button::new("Submit").on_press(PostsMessage::SubmitReport(post_index).into()),
                )
                .center_x(Length::Fill)
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
