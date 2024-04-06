use std::sync::Arc;
use mongodb::bson::{Bson, doc, Document, Uuid, UuidRepresentation};
use mongodb::Database;
use mongodb::options::UpdateOptions;
use crate::canvas::tool;
use crate::canvas::tool::Tool;
use crate::errors::debug::DebugError;
use crate::errors::error::Error;
use crate::mongo;
use crate::scenes::data::drawing::Tag;

/// Gets the data for the drawing stored online with the given id.
pub async fn get_drawing(db: &Database, id: Uuid)
     -> Result<(Vec<(Uuid, String)>, Vec<(Arc<dyn Tool>, Uuid)>), Error>
{
    let layers = match db.collection::<Document>("canvases").find_one(
        doc!{
            "id": id
        },
        None
    ).await {
        Ok(Some(document)) => {
            if let Ok(layers) = document.get_array("layers") {
               layers.iter().filter_map(
                   |document| {
                       document.as_document().map(
                           |document| {
                               (
                                   if let Some(Bson::Binary(bin)) = document.get("id") {
                                       bin.to_uuid_with_representation(UuidRepresentation::Standard).unwrap()
                                   } else {
                                       Uuid::default()
                                   },
                                   document.get_str("name").unwrap().to_string()
                               )
                           }
                       )
                   }
               ).collect()
            } else {
                return Err(Error::DebugError(DebugError::new(
                    "Error retrieving layers from database!".to_string()
                )));
            }
        }
        Ok(None) => {
            return Err(Error::DebugError(DebugError::new(
                format!("The canvas with id {} could not be found in the database!", id)
            )));
        }
        Err(err) => {
            return Err(Error::DebugError(DebugError::new(err.to_string())));
        }
    };

    let tools = match db.collection::<Document>("tools").find(
        doc! {
            "canvas_id": id
        },
        None
    ).await {
        Ok(mut documents) => {
            mongo::base::resolve_cursor::<Document>(&mut documents).await.iter().filter_map(
                |document| tool::get_deserialized(document)
            ).collect()
        }
        Err(err) => {
            return Err(Error::DebugError(DebugError::new(err.to_string())));
        }
    };

    Ok((layers, tools))
}

/// Creates a new drawing with the given id, owned by the given user.
pub async fn create_drawing(db: &Database, id: Uuid, user_id: Uuid)
    -> Result<(Uuid, String), Error>
{
    let layer_id = Uuid::new();

    match db.collection::<Document>("canvases").insert_one(
        doc! {
                "id": id,
                "user_id": user_id,
                "layers": [doc!{
                    "id": layer_id,
                    "name": "New layer"
                }]
            },
        None
    ).await {
        Ok(_) => Ok((layer_id, "New layer".into())),
        Err(err) => Err(Error::DebugError(DebugError::new(err.to_string())))
    }
}

/// Creates a new post with the given id and credentials. The drawing itself will be stored
/// in dropbox, and will be identified using the post id.
pub async fn create_post(db: &Database, id: Uuid, user_id: Uuid, description: String, tags: Vec<String>)
     -> Result<(), Error>
{
    match db.collection::<Document>("posts").insert_one(
        doc! {
                "id": id,
                "user_id": user_id,
                "description": description,
                "tags": tags.clone()
            },
        None
    ).await {
        Ok(_) => { },
        Err(err) => {
            return Err(Error::DebugError(DebugError::new(err.to_string())));
        }
    };

    match db.collection::<Document>("tags").update_many(
        doc! {
                "name": {
                    "$in": tags
                }
            },
        doc! {
                "$inc": {
                    "uses": 1
                }
            },
        UpdateOptions::builder().upsert(true).build()
    ).await {
        Ok(_) => Ok(()),
        Err(err) => Err(Error::DebugError(DebugError::new(err.to_string())))
    }
}

/// Attempt to get a list of all tags.
pub async fn get_tags(db: &Database) -> Result<Vec<Tag>, Error>
{
    match db.collection::<Document>("tags").find(
        doc! { },
        None
    ).await {
        Ok(ref mut cursor) => {
            Ok(mongo::base::resolve_cursor::<Tag>(cursor).await)
        }
        Err(err) => Err(Error::DebugError(DebugError::new(err.to_string())))
    }
}

/// Updates the amount of layers that there are in the drawing of the given id.
pub async fn add_layer(db: &Database, id: Uuid, layer_id: Uuid) -> Result<(), Error>
{
    match db.collection::<Document>("canvases").update_one(
        doc! {
                "id": id
            },
        doc! {
                "push": {
                    "layers": doc! {
                        "id": layer_id,
                        "name": "New layer"
                    }
                }
            },
        None
    ).await {
        Ok(_) => Ok(()),
        Err(err) => Err(Error::DebugError(DebugError::new(err.to_string())))
    }
}

/// Change the name of a layer.
pub async fn update_layer_name(db: &Database, drawing_id: &Uuid, id: &Uuid, name: &String)
    -> Result<(), Error> {
    match db.collection::<Document>("canvases").update_one(
        doc! {
            "id": *drawing_id,
            "layers.id": &id
        },
        doc! {
            "$set": {
                "layers.$.name": name.clone()
            }
        },
        None
    ).await {
        Ok(result) => {
            if result.modified_count > 0 {
                Ok(())
            } else {
                Err(Error::DebugError(DebugError::new(format!(
                    "Couldn't find drawing with id {}!", *drawing_id
                ))))
            }
        }
        Err(err) => Err(Error::DebugError(DebugError::new(err.to_string())))
    }
}

/// Updates the tool data of the drawing, by deleting everything that was undone and inserting
/// everything in the given "tools" parameter.
pub async fn update_drawing(
    db: &Database,
    canvas_id: Uuid,
    delete_lower_bound: u32,
    delete_upper_bound: u32,
    tools: Vec<Document>
) -> Result<(), Error> {
    match db.collection::<Document>("tools").delete_many(
        doc! {
                "canvas_id": canvas_id,
                "order": {
                    "$gte": delete_lower_bound,
                    "$lte": delete_upper_bound
                }
            },
        None
    ).await {
        Ok(_) => { },
        Err(err) => {
            return Err(Error::DebugError(DebugError::new(err.to_string())));
        }
    }

    match db.collection::<Document>("tools").insert_many(
        tools,
        None
    ).await {
        Ok(_) => Ok(()),
        Err(err) => Err(Error::DebugError(DebugError::new(err.to_string())))
    }
}