use std::fs;
use std::fs::File;
use std::io::Write;
use directories::ProjectDirs;
use mongodb::bson::{Binary, Bson, doc, Document, Uuid};
use mongodb::bson::spec::BinarySubtype;
use mongodb::Database;
use mongodb::options::UpdateOptions;
use rand::random;
use sha2::{Digest, Sha256};
use crate::errors::auth::AuthError;
use crate::errors::debug::DebugError;
use crate::errors::error::Error;
use crate::scenes::data::auth::User;
use crate::serde::Deserialize;

/// Checks if an authentication token is saved on the user's computer.
///
/// If there is one, the user will be automatically logged in.
pub async fn get_user_from_token(database: &Database) -> Result<User, Error>
{
    let proj_dirs = ProjectDirs::from("", "CharMe", "Chartsy").unwrap();
    let file_path = proj_dirs.data_local_dir().join("./token");
    let token = fs::read(file_path);

    if let Ok(token) = token {
        let mut sha = Sha256::new();
        Digest::update(&mut sha, token);
        let hash = sha.finalize();
        let bin = Bson::Binary(Binary {
            bytes: Vec::from(hash.as_slice()),
            subtype: BinarySubtype::Generic,
        });

        match database.collection::<Document>("users").find_one(
            doc! {
                "code": bin
            },
            None
        ).await {
            Ok(Some(ref document)) => Ok(User::deserialize(document)),
            Ok(None) => Err(Error::DebugError(DebugError::new(
                "No user previously logged in!"
            ))),
            Err(err) => Err(Error::DebugError(DebugError::new(err.to_string())))
        }
    } else {
        Err(Error::DebugError(DebugError::new("No user previously logged in!")))
    }
}

/// When a user is logged in, the authentication token is updated in the database in order
/// to increase security.
pub async fn update_user_token(database: &Database, user_id: Uuid)
{
    let code = random::<[u8; 32]>();
    let mut sha = Sha256::new();
    Digest::update(&mut sha, code);
    let hash = sha.finalize();
    let bin = Bson::Binary(Binary {
        bytes: Vec::from(hash.as_slice()),
        subtype: BinarySubtype::Generic,
    });

    let result = database.collection::<Document>("users").update_one(
        doc! {
            "id": user_id
        },
        doc! {
            "$set": {
                "code": bin
            }
        },
        None
    ).await;

    match result {
        Ok(result) if result.modified_count > 0 => {
            let proj_dirs = ProjectDirs::from("", "CharMe", "Chartsy").unwrap();
            let file_path = proj_dirs.data_local_dir().join("./token");

            let mut file = File::create(file_path).unwrap();
            file.write(code.as_slice()).unwrap();
        }
        _ => { }
    }
}

/// Creates a [User] by adding the data to the database if a user with the given email doesn't
/// already exist.
pub async fn create_user(db: &Database, user_email: String, user_data: Document) 
    -> Result<(), Error>
{
    match db.collection::<Document>("users").update_one(
        doc! {
                "email": user_email.clone()
            },
        doc! {
                "$setOnInsert": user_data
            },
        UpdateOptions::builder().upsert(true).build()
    ).await {
        Ok(result) => {
            if result.matched_count > 0 {
                Err(Error::AuthError(AuthError::RegisterUserAlreadyExists))
            } else {
                Ok(())
            }
        }
        Err(err) => Err(Error::DebugError(DebugError::new(err.to_string())))
    }
}

/// Checks if there is a user in the database with the given email that expects the given
/// validation code.
pub async fn validate_email(db: &Database, email: String, code: String)
    -> Result<(), Error>
{
    match db.collection::<Document>("users").update_one(
        doc! {
                "email": email.clone(),
                "code": code.clone()
            },
        doc! {
                "$set": {
                    "validated": true
                }
            },
        None
    ).await {
        Ok(result) => {
            if result.matched_count > 0 {
                Ok(())
            } else {
                Err(Error::AuthError(AuthError::RegisterBadCode))
            }
        }
        Err(err) => {
            Err(Error::DebugError(DebugError::new(err.to_string())))
        }
    }
}

/// Checks if there exists a [User] with the given login credentials.
pub async fn login(db: &Database, user_data: Document) -> Result<User, Error>
{
    match db.collection::<Document>("users").find_one(
        user_data,
        None
    ).await {
        Ok(Some(ref user)) => {
            Ok(User::deserialize(user))
        }
        Ok(None) => {
            Err(Error::AuthError(AuthError::LogInUserDoesntExist))
        }
        Err(err) => {
            Err(Error::DebugError(DebugError::new(err.to_string())))
        }
    }
}