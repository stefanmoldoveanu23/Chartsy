use mongodb::bson::{doc, Document, Uuid};
use mongodb::Database;
use mongodb::options::{AggregateOptions, UpdateOptions};
use crate::errors::debug::{debug_message, DebugError};
use crate::errors::error::Error;
use crate::database::base::resolve_cursor;
use crate::scenes::data::posts::{Comment, Post};

/// Gets a list of comments with the given filter, which will decide the parent of the comments.
pub async fn get_comments(db: &Database, filter: Document) -> Result<Vec<Comment>, Error>
{
    match db.collection::<Result<Document, mongodb::error::Error>>("comments").aggregate(
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
            }
        ],
        AggregateOptions::builder().allow_disk_use(true).build()
    ).await {
        Ok(ref mut cursor) => Ok(resolve_cursor::<Comment>(cursor).await),
        Err(err) => Err(Error::DebugError(DebugError::new(err.to_string())))
    }
}

/// Inserts a comment from the given document.
pub async fn create_comment(db: &Database, comment: &Document) -> Result<(), Error>
{
    db.collection::<Document>("comments").insert_one(
        comment,
        None
    ).await.map(|_| ()).map_err(|err| Error::DebugError(DebugError::new(debug_message!(err.to_string()))))
}

/// Generates recommendations for the user with the given id.
pub async fn get_recommendations(db: &Database, user_id: Uuid) -> Result<Vec<Post>, Error>
{
    match db.collection::<Document>("ratings").aggregate(
        Vec::from([
            // Groups all ratings by the post they were made on
            doc! {
                "$group": {
                    "_id": "$post_id",
                    "ratings": {
                        "$push": "$$ROOT"
                    }
                }
            },
            // Filters out all posts that the currently authenticated user hasn't rated
            doc! {
                "$match": {
                    "ratings": {
                        "$elemMatch": { "user_id": user_id }
                    }
                }
            },
            // Unwind all groups
            doc! {
                "$unwind": "$ratings"
            },
            // Join with the corresponding post
            doc! {
                "$lookup": {
                    "from": "posts",
                    "localField": "_id",
                    "foreignField": "id",
                    "as": "post"
                }
            },
            // Unwind by post
            doc! {
                "$unwind": "$post"
            },
            // Unwind by tags
            doc! {
                "$unwind": "$post.tags"
            },
            // Restructure to keep the essential data
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
            // Average score by user and tag
            doc! {
                "$group": {
                    "_id": { "user_id": "$user_id", "tag": "$tag" },
                    "score": { "$avg": "$value" }
                }
            },
            // Group by user, computing the magnitudes
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
            // Unwind; this way, each tag will have access to the authenticated users score for the same tag
            doc! {
                "$unwind": "$groups"
            },
            // Create the dot multiplication
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
            // Group by user and compute magnitudes and dot product
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
            // Group to isolate authenticated user
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
            // Unwind to get each user again, except the authenticated one
            doc! {
                "$unwind": "$users"
            },
            // Project to compute each user's similarity score
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
        AggregateOptions::builder().allow_disk_use(true).build()
    ).await {
        Ok(ref mut cursor) => {
            Ok(resolve_cursor::<Post>(cursor).await)
        },
        Err(err) => {
            Err(Error::DebugError(DebugError::new(debug_message!(err.to_string()))))
        }
    }
}

/// Gets the posts that contain all the given tags.
pub async fn get_filtered(db: &Database, user_id: Uuid, tags: Vec<String>) -> Result<Vec<Post>, Error>
{
    match db.collection::<Document>("posts").aggregate(
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
            }
        ],
        AggregateOptions::builder().allow_disk_use(true).build()
    ).await {
        Ok(ref mut cursor) => Ok(resolve_cursor::<Post>(cursor).await),
        Err(err) => Err(Error::DebugError(DebugError::new(debug_message!(err.to_string()))))
    }
}

/// Gets the posts of the user with the given id.
pub async fn get_user_posts(db: &Database, user_id: Uuid) -> Result<Vec<Post>, Error>
{
    match db.collection::<Document>("posts").aggregate(
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
        AggregateOptions::builder().allow_disk_use(true).build()
    ).await {
        Ok(ref mut cursor) => Ok(resolve_cursor::<Post>(cursor).await),
        Err(err) => Err(Error::DebugError(DebugError::new(debug_message!(err.to_string()))))
    }
}

/// Gets a list of "count" posts sampled randomly that are not in the "denied" list.
pub async fn get_random_posts(db: &Database, count: usize, user_id: Uuid, denied: Vec<Uuid>)
  -> Result<Vec<Post>, Error>
{
    match db.collection::<Document>("posts").aggregate(
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
            }
        ],
        AggregateOptions::builder().allow_disk_use(true).build()
    ).await {
        Ok(ref mut cursor) => Ok(resolve_cursor::<Post>(cursor).await),
        Err(err) => Err(Error::DebugError(DebugError::new(debug_message!(err.to_string()))))
    }
}

/// Updates the rating that the user has given to the post.
/// If there was no previous rating, it will be inserted.
pub async fn update_rating(db: &Database, post_id: Uuid, user_id: Uuid, rating: i32)
   -> Result<(), Error>
{
    db.collection::<Document>("ratings").update_one(
        doc! {
            "post_id": post_id,
            "user_id": user_id,
        },
        doc! {
            "$set": {
                "rating": rating
            }
        },
        UpdateOptions::builder().upsert(true).build()
    ).await.map(|_| ()).map_err(|err| Error::DebugError(DebugError::new(debug_message!(err.to_string()))))
}

/// Deletes the rating that the user has given the post.
pub async fn delete_rating(db: &Database, post_id: Uuid, user_id: Uuid) -> Result<(), Error>
{
    db.collection::<Document>("ratings").delete_one(
        doc! {
                "post_id": post_id,
                "user_id": user_id
            },
        None
    ).await.map(|_| ()).map_err(|err| Error::DebugError(DebugError::new(debug_message!(err.to_string()))))
}