use std::io;
use dropbox_sdk::default_client::{NoauthDefaultClient, UserAuthDefaultClient};
use dropbox_sdk::files;
use dropbox_sdk::files::{DownloadArg, UploadArg, WriteMode};
use mongodb::{Client, Cursor};
use mongodb::bson::Document;
use mongodb::options::ClientOptions;
use crate::config;
use crate::errors::debug::{debug_message, DebugError};
use crate::errors::error::Error;
use crate::serde::Deserialize;

/// Attempts to connect to the database [Database].
///
/// Returns an error upon failure.
pub async fn connect_to_mongodb() -> Result<Client, Error>
    where
        Client: Send + 'static,
{
    let client_options = match ClientOptions::parse(
        format!(
            "mongodb+srv://{}:{}@chartsy.1fzpgot.mongodb.net/?retryWrites=true&w=majority&appName=Chartsy",
            config::mongo_name(),
            config::mongo_pass()
        )
    ).await {
        Ok(options) => options,
        Err(err) => {
            return Err(Error::DebugError(DebugError::new(debug_message!(err.to_string()))))
        }
    };

    Client::with_options(client_options).map_err(
        |err| Error::DebugError(DebugError::new(debug_message!(err.to_string())))
    )
}

/// Collects all entries of the cursor, attempting to deserialize them in the functions Type.
pub async fn resolve_cursor<Type>(cursor: &mut Cursor<Document>) -> Vec<Type>
where
    Type: Deserialize<Document>
{
    let mut objects = vec![];
    loop {
        let exists = cursor.advance().await;
        let document = match exists {
            Ok(true) => {
                match Document::try_from(cursor.current()) {
                    Ok(document) => document,
                    _ => { break; }
                }
            }
            _ => { break; }
        };

        let object = Type::deserialize(&document);

        objects.push(object);
    }

    objects
}

/// Connects to dropbox and returns a client by refreshing the token.
pub async fn connect_to_dropbox() -> Result<UserAuthDefaultClient, Error>
{
    tokio::task::spawn_blocking(
        || {
            let mut auth = dropbox_sdk::oauth2::Authorization::from_refresh_token(
                config::dropbox_id().into(),
                config::dropbox_refresh_token().into(),
            );

            let _token = auth
                .obtain_access_token(NoauthDefaultClient::default())
                .unwrap();
            UserAuthDefaultClient::new(auth)
        }
    ).await.map_err(|err| Error::DebugError(DebugError::new(debug_message!(err.to_string()))))
}

/// Uploads a file to dropbox.
pub async fn upload_file(path: String, data: Vec<u8>) -> Result<(), Error>
{
    let client = match connect_to_dropbox().await {
        Ok(client) => client,
        Err(err) => {
            return Err(err);
        }
    };

    match tokio::task::spawn_blocking(
        move || {

            match files::upload(
                &client,
                &UploadArg::new(path)
                    .with_mute(false)
                    .with_mode(WriteMode::Overwrite),
                data.as_slice()
            ) {
                Ok(Ok(_)) => Ok(()),
                Ok(Err(err)) => Err(Error::DebugError(DebugError::new(debug_message!(err.to_string())))),
                Err(err) => Err(Error::DebugError(DebugError::new(debug_message!(err.to_string()))))
            }
        }
    ).await {
        Ok(Ok(_)) => Ok(()),
        Ok(Err(err)) => Err(err),
        Err(err) => Err(Error::DebugError(DebugError::new(debug_message!(err.to_string()))))
    }
}

/// Downloads a file from dropbox.
pub async fn download_file(path: String) -> Result<Vec<u8>, Error>
{
    let client = match connect_to_dropbox().await {
        Ok(client) => client,
        Err(err) => {
            return Err(err);
        }
    };

    match tokio::task::spawn_blocking(
        move || {
            match files::download(
                &client,
                &DownloadArg::new(path.clone()),
                None,
                None
            ) {
                Ok(Ok(ref mut result)) => {
                    let mut data :Vec<u8>= vec![];

                    match result.body {
                        Some(ref mut reader) => {
                            match io::copy(reader, &mut data) {
                                Ok(_) => Ok(data),
                                Err(err) => Err(Error::DebugError(
                                    DebugError::new(debug_message!(err.to_string()))
                                ))
                            }
                        }
                        None => Err(Error::DebugError(DebugError::new(
                            debug_message!(format!("Could not find reader for the file {}.", path))
                        )))
                    }
                }
                Ok(Err(err)) => Err(Error::DebugError(
                    DebugError::new(err.to_string())
                )),
                Err(err) => Err(Error::DebugError(DebugError::new(debug_message!(err.to_string()))))
            }
        }
    ).await {
        Ok(Ok(data)) => Ok(data),
        Ok(Err(err)) => Err(err),
        Err(err) => Err(Error::DebugError(DebugError::new(debug_message!(err.to_string()))))
    }
}