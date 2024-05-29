use crate::errors::auth::AuthError;
use crate::errors::debug::{debug_message, DebugError};
use crate::errors::error::Error;
use crate::scene::Globals;
use mongodb::bson::{doc, Bson, DateTime, Document, Uuid};
use mongodb::Database;

/// Updates the data of a user, given their [id](Uuid) and what needs to be updated.
pub async fn update_user(db: &Database, user_id: Uuid, update: Document) -> Result<(), Error> {
    match db
        .collection::<Document>("users")
        .update_one(
            doc! {
                "id": user_id
            },
            doc! {
                "$set": update
            },
            None,
        )
        .await
    {
        Ok(result) => {
            if result.modified_count > 0 {
                Ok(())
            } else {
                Err(debug_message!("Database couldn't find a user with id {}.", user_id).into())
            }
        }
        Err(err) => Err(debug_message!("{}", err).into()),
    }
}

/// Checks if there already exists a user with the requested tag.
pub async fn find_user_by_tag(globals: &Globals, user_tag: String) -> Result<(), Error> {
    let user_id = globals.get_user().unwrap().get_id();

    let db = globals.get_db().unwrap();
    let mut session = globals.start_session().await.unwrap()?;
    match session.start_transaction(None).await {
        Ok(_) => {}
        Err(err) => return Err(debug_message!("{}", err).into()),
    }

    match db
        .collection::<Document>("users")
        .find_one_with_session(
            doc! {
                "user_tag": user_tag.clone()
            },
            None,
            &mut session,
        )
        .await
    {
        Ok(Some(_)) => {
            return match session.abort_transaction().await {
                Ok(_) => Err(Error::AuthError(AuthError::UserTagAlreadyExists)),
                Err(err) => Err(debug_message!("{}", err).into()),
            }
        }
        Ok(None) => {}
        Err(err) => {
            return match session.abort_transaction().await {
                Ok(_) => Err(Error::DebugError(DebugError::new(err.to_string()))),
                Err(err) => Err(debug_message!("{}", err).into()),
            }
        }
    }

    match db
        .collection::<Document>("users")
        .update_one_with_session(
            doc! {
                "id": user_id
            },
            doc! {
                "$set": {
                    "user_tag": user_tag
                }
            },
            None,
            &mut session,
        )
        .await
    {
        Ok(result) => {
            if result.modified_count > 0 {
                match session.commit_transaction().await {
                    Ok(_) => Ok(()),
                    Err(err) => Err(debug_message!("{}", err).into()),
                }
            } else {
                match session.abort_transaction().await {
                    Ok(_) => Err(debug_message!(
                        "Database could not find user with id {}!",
                        user_id
                    )
                    .into()),
                    Err(err) => Err(debug_message!("{}", err).into()),
                }
            }
        }
        Err(err) => match session.abort_transaction().await {
            Ok(_) => Err(debug_message!("{}", err).into()),
            Err(err) => Err(debug_message!("{}", err).into()),
        },
    }
}

/// Sets the currently logged users expiration date to a week from now.
/// The user will be automatically logged out and won't be able to log in automatically anymore.
/// The account will be automatically deleted in a month.
pub async fn delete_account(db: &Database, id: Uuid) -> Result<(), Error> {
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
                Err(debug_message!("Database could not find user with id {}.", id).into())
            }
        }
        Err(err) => Err(debug_message!("{}", err).into())
    }
}
