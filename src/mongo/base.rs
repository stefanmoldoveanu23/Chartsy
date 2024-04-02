use mongodb::{Client, Cursor, Database};
use mongodb::bson::Document;
use mongodb::options::ClientOptions;
use crate::config;
use crate::errors::error::Error;
use crate::serde::Deserialize;

/// Attempts to connect to the mongo [Database].
///
/// Returns an error upon failure.
pub async fn connect_to_mongodb() -> Result<Database, Error>
    where
        Client: Send + 'static,
{
    let client_options = ClientOptions::parse(
        format!(
            "mongodb+srv://{}:{}@chartsy.1fzpgot.mongodb.net/?retryWrites=true&w=majority&appName=Chartsy",
            config::mongo_name(),
            config::mongo_pass()
        )
    ).await.map_err(|error| Error::from(error))?;

    let client = Client::with_options(client_options).map_err(|error| Error::from(error))?;

    Ok(client.database("chartsy"))
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