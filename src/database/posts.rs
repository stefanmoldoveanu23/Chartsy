use crate::database::base::resolve_cursor;
use crate::debug_message;
use crate::scene::Globals;
use crate::scenes::data::auth::User;
use crate::scenes::data::posts::{Comment, Post};
use crate::utils::errors::{AuthError, DebugError, Error};
use crate::utils::serde::Deserialize;
use mongodb::bson::{doc, Document, Uuid};
use mongodb::options::{AggregateOptions, UpdateOptions};
use mongodb::Database;

/// Gets a list of comments with the given filter, which will decide the parent of the comments.
pub async fn get_comments(db: &Database, filter: Document) -> Result<Vec<Comment>, Error> {
    match db
        .collection::<Result<Document, mongodb::error::Error>>("comments")
        .aggregate(
            vec![
                doc! {
                    "$match": filter,
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
            ],
            AggregateOptions::builder().allow_disk_use(true).build(),
        )
        .await
    {
        Ok(ref mut cursor) => Ok(resolve_cursor::<Comment>(cursor).await),
        Err(err) => Err(Error::DebugError(DebugError::new(err.to_string()))),
    }
}

/// Inserts a comment from the given document.
pub async fn create_comment(db: &Database, comment: &Document) -> Result<(), Error> {
    db.collection::<Document>("comments")
        .insert_one(comment, None)
        .await
        .map(|_| ())
        .map_err(|err| debug_message!("{}", err).into())
}

/// Generates recommendations for the user with the given id.
pub async fn get_recommendations(db: &Database, user_id: Uuid) -> Result<Vec<Post>, Error> {
    match db
        .collection::<Document>("similarities")
        .aggregate(
            Vec::from([
                // Get only the similarities with the authenticated user.
                doc! {
                    "$match": {
                        "user_id": user_id
                    }
                },
                // Leave only the id of the other user.
                doc! {
                    "$project": {
                        "_id": {
                            "$arrayElemAt": [
                                {
                                    "$filter": {
                                        "input": "$user_id",
                                        "as": "ids",
                                        "cond": {
                                            "$ne": [
                                                "$$ids",
                                                user_id
                                            ]
                                        }
                                    }
                                },
                                0
                            ]
                        },
                        "score": 1
                    }
                },
                // Join with users to get full data
                doc! {
                    "$lookup": {
                        "from": "users",
                        "localField": "_id",
                        "foreignField": "id",
                        "as": "user"
                    }
                },
                // Unwind user data
                doc! {
                    "$unwind": "$user"
                },
                // Join with posts
                doc! {
                    "$lookup": {
                        "from": "posts",
                        "localField": "_id",
                        "foreignField": "user_id",
                        "as": "post",
                    }
                },
                // Unwind the posts
                doc! {
                    "$unwind": "$post"
                },
                // Add a field of random value
                doc! {
                    "$addFields": {
                        "randomValue": { "$rand": {} }
                    }
                },
                // Remove those with score less than the random value
                doc! {
                    "$match": {
                        "$expr": {
                            "$lte": ["$randomValue", "$score"]
                        }
                    }
                },
                // Sample 100 of those remaining
                doc! {
                    "$sample": {
                        "size": 100
                    }
                },
                // Join with ratings
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
                // Unwind the rating
                doc! {
                    "$unwind": {
                        "path": "$rating",
                        "preserveNullAndEmptyArrays": true
                    }
                },
            ]),
            AggregateOptions::builder().allow_disk_use(true).build(),
        )
        .await
    {
        Ok(ref mut cursor) => Ok(resolve_cursor::<Post>(cursor).await),
        Err(err) => Err(debug_message!("{}", err).into()),
    }
}

/// Gets the posts that contain all the given tags.
pub async fn get_filtered(
    db: &Database,
    user_id: Uuid,
    tags: Vec<String>,
) -> Result<Vec<Post>, Error> {
    match db
        .collection::<Document>("posts")
        .aggregate(
            vec![
                doc! {
                    "$match": {
                        "tags": { "$all": tags }
                    }
                },
                doc! {
                    "$project": {
                        "post": "$$ROOT"
                    }
                },
                doc! {
                    "$lookup": {
                        "from": "users",
                        "localField": "post.user_id",
                        "foreignField": "id",
                        "as": "user"
                    }
                },
                doc! {
                    "$unwind": "$user"
                },
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
                doc! {
                    "$unwind": {
                        "path": "$rating",
                        "preserveNullAndEmptyArrays": true
                    }
                },
                doc! {
                    "$limit": 100
                },
            ],
            AggregateOptions::builder().allow_disk_use(true).build(),
        )
        .await
    {
        Ok(ref mut cursor) => Ok(resolve_cursor::<Post>(cursor).await),
        Err(err) => Err(debug_message!("{}", err).into()),
    }
}

/// Gets the posts of the user with the given id.
pub async fn get_user_posts(db: &Database, user_id: Uuid) -> Result<Vec<Post>, Error> {
    match db
        .collection::<Document>("posts")
        .aggregate(
            vec![
                doc! {
                    "$match": {
                        "user_id": user_id
                    }
                },
                doc! {
                    "$project": {
                        "post": "$$ROOT"
                    }
                },
                doc! {
                    "$lookup": {
                        "from": "users",
                        "localField": "post.user_id",
                        "foreignField": "id",
                        "as": "user"
                    }
                },
                doc! {
                    "$unwind": "$user"
                },
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
                doc! {
                    "$unwind": {
                        "path": "$rating",
                        "preserveNullAndEmptyArrays": true
                    }
                },
            ],
            AggregateOptions::builder().allow_disk_use(true).build(),
        )
        .await
    {
        Ok(ref mut cursor) => Ok(resolve_cursor::<Post>(cursor).await),
        Err(err) => Err(debug_message!("{}", err).into()),
    }
}

/// Gets a list of "count" posts sampled randomly that are not in the "denied" list.
pub async fn get_random_posts(
    db: &Database,
    count: usize,
    user_id: Uuid,
    denied: Vec<Uuid>,
) -> Result<Vec<Post>, Error> {
    match db
        .collection::<Document>("posts")
        .aggregate(
            vec![
                doc! {
                    "$match": {
                        "id": {
                            "$nin": denied
                        }
                    }
                },
                doc! {
                    "$sample": {
                        "size": count as i32
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
                },
            ],
            AggregateOptions::builder().allow_disk_use(true).build(),
        )
        .await
    {
        Ok(ref mut cursor) => Ok(resolve_cursor::<Post>(cursor).await),
        Err(err) => Err(debug_message!("{}", err).into()),
    }
}

/// Updates the rating that the user has given to the post.
/// If there was no previous rating, it will be inserted.
pub async fn update_rating(
    db: &Database,
    post_id: Uuid,
    user_id: Uuid,
    rating: i32,
) -> Result<(), Error> {
    db.collection::<Document>("ratings")
        .update_one(
            doc! {
                "post_id": post_id,
                "user_id": user_id,
            },
            doc! {
                "$set": {
                    "rating": rating
                }
            },
            UpdateOptions::builder().upsert(true).build(),
        )
        .await
        .map(|_| ())
        .map_err(|err| debug_message!("{}", err).into())
}

/// Deletes the rating that the user has given the post.
pub async fn delete_rating(db: &Database, post_id: Uuid, user_id: Uuid) -> Result<(), Error> {
    db.collection::<Document>("ratings")
        .delete_one(
            doc! {
                "post_id": post_id,
                "user_id": user_id
            },
            None,
        )
        .await
        .map(|_| ())
        .map_err(|err| debug_message!("{}", err).into())
}

/// Returns the user that has the given tag.
pub async fn get_user_by_tag(db: &Database, user_tag: String) -> Result<User, Error> {
    match db
        .collection::<Document>("users")
        .find_one(
            doc! {
                "user_tag": user_tag.clone()
            },
            None,
        )
        .await
    {
        Ok(Some(ref user)) => Ok(Deserialize::deserialize(user)),
        Ok(None) => Err(Error::AuthError(AuthError::UserTagDoesNotExist(user_tag))),
        Err(err) => Err(debug_message!("{}", err).into()),
    }
}

/// Deletes the given post.
pub async fn delete_post(id: Uuid, globals: &Globals) -> Result<(), Error> {
    let db = globals
        .get_db()
        .ok_or(debug_message!("Could not access database.").into())?;

    let posts = db.collection::<Document>("posts");

    match posts
        .delete_one(
            doc! {
                "id": id
            },
            None,
        )
        .await
    {
        Ok(result) if result.deleted_count > 0 => Ok(()),
        Ok(_) => Err(debug_message!("Could not find post with id {} to delete.", id).into()),
        Err(err) => Err(debug_message!("{}", err).into()),
    }
}
