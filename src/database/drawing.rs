use crate::canvas::tool;
use crate::canvas::tool::Tool;
use crate::database;
use crate::utils::errors::{Error, DebugError};
use crate::debug_message;
use crate::scene::Globals;
use crate::scenes::data::drawing::Tag;
use mongodb::bson::{doc, Bson, Document, Uuid, UuidRepresentation};
use mongodb::Database;
use std::sync::Arc;

/// Gets the data for the drawing stored online with the given id.
pub async fn get_drawing(
    db: &Database,
    id: Uuid,
) -> Result<(Vec<(Uuid, String)>, Vec<(Arc<dyn Tool>, Uuid)>), Error> {
    let layers = match db
        .collection::<Document>("canvases")
        .find_one(
            doc! {
                "id": id
            },
            None,
        )
        .await
    {
        Ok(Some(document)) => {
            if let Ok(layers) = document.get_array("layers") {
                layers
                    .iter()
                    .filter_map(|document| {
                        document.as_document().map(|document| {
                            (
                                if let Some(Bson::Binary(bin)) = document.get("id") {
                                    bin.to_uuid_with_representation(UuidRepresentation::Standard)
                                        .unwrap()
                                } else {
                                    Uuid::default()
                                },
                                document.get_str("name").unwrap().to_string(),
                            )
                        })
                    })
                    .collect()
            } else {
                return Err(Error::DebugError(DebugError::new(debug_message!(
                    "Error retrieving layers from database!"
                ))));
            }
        }
        Ok(None) => {
            return Err(debug_message!(
                "The canvas with id {} could not be found in the database!",
                id
            )
            .into());
        }
        Err(err) => {
            return Err(debug_message!("{}", err).into());
        }
    };

    let tools = match db
        .collection::<Document>("tools")
        .find(
            doc! {
                "canvas_id": id
            },
            None,
        )
        .await
    {
        Ok(mut documents) => database::base::resolve_cursor::<Document>(&mut documents)
            .await
            .iter()
            .filter_map(|document| tool::get_deserialized(document))
            .collect(),
        Err(err) => {
            return Err(debug_message!("{}", err).into());
        }
    };

    Ok((layers, tools))
}

/// Creates a new drawing with the given id, owned by the given user.
pub async fn create_drawing(
    db: &Database,
    id: Uuid,
    user_id: Uuid,
) -> Result<(Uuid, String), Error> {
    let layer_id = Uuid::new();

    match db
        .collection::<Document>("canvases")
        .insert_one(
            doc! {
                "id": id,
                "name": "New drawing",
                "user_id": user_id,
                "layers": [doc!{
                    "id": layer_id,
                    "name": "New layer"
                }]
            },
            None,
        )
        .await
    {
        Ok(_) => Ok((layer_id, "New layer".into())),
        Err(err) => Err(debug_message!("{}", err).into()),
    }
}

/// Creates a new post with the given id and credentials. The drawing itself will be stored
/// in dropbox, and will be identified using the post id.
pub async fn create_post(
    db: &Database,
    id: Uuid,
    user_id: Uuid,
    description: String,
    tags: Vec<String>,
) -> Result<(), Error> {
    match db
        .collection::<Document>("posts")
        .insert_one(
            doc! {
                "id": id,
                "user_id": user_id,
                "description": description,
                "tags": tags.clone()
            },
            None,
        )
        .await
    {
        Ok(_) => Ok(()),
        Err(err) => Err(debug_message!("{}", err).into()),
    }
}

/// Attempt to get a list of all tags.
pub async fn get_tags(db: &Database) -> Result<Vec<Tag>, Error> {
    match db.collection::<Document>("tags").find(doc! {}, None).await {
        Ok(ref mut cursor) => Ok(database::base::resolve_cursor::<Tag>(cursor).await),
        Err(err) => Err(debug_message!("{}", err).into()),
    }
}

/// Updates the tool data of the drawing, by deleting everything that was undone and inserting
/// everything in the given "tools" parameter.
pub async fn update_drawing(
    db: &Database,
    canvas_id: Uuid,
    canvas_name: String,
    delete_lower_bound: u32,
    delete_upper_bound: u32,
    tools: Vec<Document>,
    removed_layers: Vec<Uuid>,
    layer_data: Vec<(Uuid, String)>,
) -> Result<(), Error> {
    match db
        .collection::<Document>("tools")
        .delete_many(
            doc! {
                "canvas_id": canvas_id,
                "$or": [
                    {
                        "order": {
                            "$gte": delete_lower_bound,
                            "$lte": delete_upper_bound
                        }
                    },
                    {
                        "layer": {
                            "$in": removed_layers
                        }
                    }
                ],
            },
            None,
        )
        .await
    {
        Ok(_) => {}
        Err(err) => {
            return Err(debug_message!("{}", err).into());
        }
    }

    if tools.len() > 0 {
        match db
            .collection::<Document>("tools")
            .insert_many(tools, None)
            .await
        {
            Ok(_) => {}
            Err(err) => {
                return Err(debug_message!("{}", err).into());
            }
        }
    }

    match db
        .collection::<Document>("canvases")
        .update_one(
            doc! {
                "id": canvas_id
            },
            doc! {
                "$set": {
                    "name": canvas_name,
                    "layers": layer_data.into_iter().map(
                        |(id, name)| doc! {
                            "id": id,
                            "name": name
                        }
                    ).collect::<Vec<Document>>()
                }
            },
            None,
        )
        .await
    {
        Ok(_) => Ok(()),
        Err(err) => Err(debug_message!("{}", err).into()),
    }
}

pub async fn delete_drawing(id: Uuid, globals: &Globals) -> Result<(), Error> {
    let db = globals
        .get_db()
        .ok_or(debug_message!("No database connection.").into())?;

    let canvases = db.collection::<Document>("canvases");

    match canvases
        .delete_one(
            doc! {
                "id": id
            },
            None,
        )
        .await
    {
        Ok(result) if result.deleted_count == 1 => Ok(()),
        Ok(_) => Err(debug_message!("Could not find drawing with id {} to delete.", id).into()),
        Err(err) => Err(debug_message!("{}", err).into()),
    }
}
