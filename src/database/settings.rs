use mongodb::bson::{Bson, DateTime, doc, Document, Uuid};
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

/// Sets the currently logged users expiration date to a week from now.
/// The user will be automatically logged out and won't be able to log in automatically anymore.
/// The account will be automatically deleted in a month.
pub async fn delete_account(db: &Database, id: Uuid) -> Result<(), Error>
{
    match db.collection::<Document>("users").update_one(
        doc!{
            "id": id
        },
        doc! {
            "$set": {
                "expiration_date": Bson::DateTime(
                    DateTime::from_millis(DateTime::now().timestamp_millis() + 30 * 24 * 60 * 60 * 1000)
                )
            }
        },
        None
    ).await {
        Ok(result) => {
            if result.modified_count > 0 {
                Ok(())
            } else {
                Err(Error::DebugError(DebugError::new(
                    format!("Database could not find user with id {}.", id)
                )))
            }
        }
        Err(err) => Err(Error::DebugError(DebugError::new(err.to_string())))
    }
}