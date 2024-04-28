use mongodb::bson::{doc, Document, Uuid};
use mongodb::Database;
use crate::errors::debug::{debug_message, DebugError};
use crate::errors::error::Error;
use crate::database;

/// Gets a list of drawings owned by the user with the given id.
pub async fn get_drawings(db: &Database, user_id: Uuid) -> Result<Vec<Document>, Error>
{
    match db.collection::<Document>("canvases").find(
        doc! {
            "user_id": user_id
        },
        None
    ).await {
        Ok(ref mut cursor) => Ok(database::base::resolve_cursor::<Document>(cursor).await),
        Err(err) => Err(Error::DebugError(DebugError::new(debug_message!(err.to_string()))))
    }
}