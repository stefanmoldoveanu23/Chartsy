use mongodb::bson::{doc, Document, Uuid};
use mongodb::Database;
use crate::errors::debug::DebugError;
use crate::errors::error::Error;

/// Updates the data of a user, given their [id](Uuid) and what needs to be updated.
pub async fn update_user(db: &Database, user_id: Uuid, update: Document) -> Result<(), Error>
{
    match db.collection::<Document>("users").update_one(
        doc! {
            "id": user_id
        },
        doc! {
            "$set": update
        },
        None
    ).await {
        Ok(result) => {
            if result.modified_count > 0 {
                Ok(())
            } else {
                Err(Error::DebugError(DebugError::new(
                    format!("Database couldn't find a user with id {}.", user_id)
                )))
            }
        }
        Err(err) => Err(Error::DebugError(DebugError::new(err.to_string())))
    }
}