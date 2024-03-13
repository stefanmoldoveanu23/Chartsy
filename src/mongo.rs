use crate::scene::Message;
use async_recursion::async_recursion;
use mongodb::bson::{Bson, Document, Binary, doc, Uuid};
use mongodb::results::{DeleteResult, InsertManyResult, UpdateResult};
use mongodb::{options::ClientOptions, Client, Collection, Database};
use std::fmt::Debug;
use std::fs;
use std::fs::File;
use std::io::Write;
use directories::ProjectDirs;
use mongodb::bson::spec::BinarySubtype;
use mongodb::options::{DeleteOptions, FindOptions, InsertManyOptions, UpdateOptions};
use rand::random;
use sha2::{Digest, Sha256};

use crate::config::{MONGO_NAME, MONGO_PASS};
use crate::errors::debug::DebugError;
use crate::errors::error::Error;
use crate::scenes::auth::User;
use crate::serde::Deserialize;

/// Attempts to connect to the mongo [Database].
///
/// Returns an error upon failure.
pub async fn connect_to_mongodb() -> Result<Database, Error>
where
    Client: Send + 'static,
{
    let client_options = ClientOptions::parse(
        format!("mongodb+srv://{}:{}@chartsy.1fzpgot.mongodb.net/?retryWrites=true&w=majority&appName=Chartsy", MONGO_NAME, MONGO_PASS)
    ).await.map_err(|error| Error::from(error))?;

    let client = Client::with_options(client_options).map_err(|error| Error::from(error))?;

    Ok(client.database("chartsy"))
}

/// Checks if an authentication token is saved on the user's computer.
///
/// If there is one, the user will be automatically logged in.
pub async fn get_user_from_token(database: Database) -> Result<User, Message>
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

        let result = MongoRequest::send_requests(
            database.clone(),
            vec![
                MongoRequest::new(
                    "users".into(),
                    MongoRequestType::Get{filter: doc!{"code": bin}, options: None},
                )
            ]
        ).await;

        if let Ok(response) = result {
            if let Some(MongoResponse::Get(users)) = response.get(0) {
                if users.len() > 0 {
                    return Ok(User::deserialize(users.get(0).unwrap().clone()));
                }
            }
        }
    }

    Err(Message::Error(Error::DebugError(DebugError::new("No user previously logged in.".into()))))
}

/// When a user is logged in, the authentication token is updated in the database in order
/// to increase security.
pub async fn update_user_token(database: Database, user_id: Uuid)
{
    let code = random::<[u8; 32]>();
    let mut sha = Sha256::new();
    Digest::update(&mut sha, code);
    let hash = sha.finalize();
    let bin = Bson::Binary(Binary {
        bytes: Vec::from(hash.as_slice()),
        subtype: BinarySubtype::Generic,
    });

    let response = MongoRequest::send_requests(
        database.clone(),
        vec![
            MongoRequest::new(
                "users".into(),
                MongoRequestType::Update {
                    filter: doc! { "id": user_id },
                    update: doc! { "$set": {
                    "code": bin
                    } },
                    options: None,
                }
            )
        ]
    ).await;

    if response.is_ok() {
        let proj_dirs = ProjectDirs::from("", "CharMe", "Chartsy").unwrap();
        let file_path = proj_dirs.data_local_dir().join("./token");

        let mut file = File::create(file_path).unwrap();
        file.write(code.as_slice()).unwrap();
    }
}

/// The four [Database] request types:
/// - [Get](MongoRequestType::Get), with the filter set in a [Document].
/// - [Insert](MongoRequestType::Insert), with the requested [Documents](Document).
/// - [Update](MongoRequestType::Update), with the filter [Document] and the replacement.
/// - [Delete](MongoRequestType::Delete), with the filter [Document].
/// - [Chain](MongoRequestType::Chain), which can chain multiple [MongoRequests](MongoRequest); the first value is
/// the initial [MongoRequestType], and the second is a vector of pairs of a [Document], and a function that takes
/// the [MongoResponse] of the previous request and the document, and returns the next request; if an
/// error took place, the chain can also be halted by setting the next request as [Err].
#[derive(Debug)]
pub enum MongoRequestType {
    Get{ filter: Document, options: Option<FindOptions> },
    Insert{ documents: Vec<Document>, options: Option<InsertManyOptions> },
    Update{ filter: Document, update: Document, options: Option<UpdateOptions> },
    Delete{ filter: Document, options: Option<DeleteOptions> },
    Chain(
        Box<Self>,
        Vec<(
            Document,
            fn(MongoResponse, Document) -> Result<MongoRequest, Error>,
        )>,
    ),
}

/// A request to be sent to a mongo [Database].
///
/// Contains the name of the altered [Collection], and the [request type](MongoRequestType).
#[derive(Debug)]
pub struct MongoRequest {
    collection_name: String,
    request_type: MongoRequestType,
}

impl MongoRequest {
    /// Creates a new [MongoRequest] using the given data.
    pub fn new(collection_name: String, request_type: MongoRequestType) -> Self {
        MongoRequest {
            collection_name,
            request_type,
        }
    }

    /// Sends a [Get](MongoRequestType::Get) request to the given [Database] and returns
    /// a list of the results.
    async fn handle_get(
        database: &Database,
        collection_name: &String,
        filter: Document,
        options: Option<FindOptions>
    ) -> Result<Vec<Document>, Error> {
        let collection: Collection<Result<Document, mongodb::error::Error>> =
            database.collection(&*collection_name);
        let cursor = collection.find(Some(filter), options).await;

        match cursor {
            Ok(mut cursor) => {
                let mut vec: Vec<Document> = vec![];
                let res: Result<Vec<Document>, Error>;

                loop {
                    let exists = cursor.advance().await;

                    match exists {
                        Ok(exists) => {
                            if exists {
                                let value = Document::try_from(cursor.current());
                                match value {
                                    Ok(document) => vec.push(document),
                                    Err(err) => {
                                        res = Err(mongodb::error::Error::from(err).into());
                                        break;
                                    }
                                }
                            } else {
                                res = Ok(vec.clone());
                                break;
                            }
                        }
                        Err(err) => {
                            res = Err(err.into());
                            break;
                        }
                    }
                }

                res
            }
            Err(err) => Err(err.into()),
        }
    }

    /// Sends an [Insert](MongoRequestType::Insert) request to the given [Database] and returns
    /// the [inserted ids](InsertManyResult).
    async fn handle_insert(
        database: &Database,
        collection_name: &String,
        documents: Vec<Document>,
        options: Option<InsertManyOptions>
    ) -> Result<InsertManyResult, Error> {
        let collection: Collection<Document> = database.collection(&*collection_name);
        collection
            .insert_many(documents, options)
            .await
            .map_err(|error| Error::from(error))
    }

    /// Sends an [Update](MongoRequestType::Update) request to the given [Database] and returns
    /// the [results](UpdateResult).
    async fn handle_update(
        database: &Database,
        collection_name: &String,
        filter: Document,
        update: Document,
        options: Option<UpdateOptions>
    ) -> Result<UpdateResult, Error> {
        let collection: Collection<Document> = database.collection(&*collection_name);
        collection
            .update_many(filter.clone(), update.clone(), options)
            .await
            .map_err(|error| Error::from(error))
    }

    /// Sends a [Delete](MongoRequestType::Delete) request to the given [Database] and returns
    /// the [number of deleted records](DeleteResult).
    async fn handle_delete(
        database: &Database,
        collection_name: &String,
        filter: Document,
        options: Option<DeleteOptions>
    ) -> Result<DeleteResult, Error> {
        let collection: Collection<Document> = database.collection(&*collection_name);
        collection
            .delete_many(filter.clone(), options)
            .await
            .map_err(|error| Error::from(error))
    }

    /// Sends a chain of requests to the given [Database] and returns the final [MongoResponse].
    #[async_recursion]
    async fn handle_chain(
        database: &Database,
        collection_name: &String,
        initial_request: Box<MongoRequestType>,
        chain: Vec<(
            Document,
            fn(MongoResponse, Document) -> Result<MongoRequest, Error>,
        )>,
    ) -> Result<MongoResponse, Error> {
        let mut request = Ok(MongoRequest {
            collection_name: collection_name.clone(),
            request_type: *initial_request,
        });
        let mut response: Result<MongoResponse, Error> =
            MongoRequest::handle_request(database, request.unwrap()).await;

        for transition in chain {
            if let Ok(res) = response {
                request = transition.1(res, transition.0);

                response = match request {
                    Ok(req) => MongoRequest::handle_request(database, req).await,
                    Err(err) => Err(err),
                };

                if response.is_err() {
                    break;
                }
            } else {
                break;
            }
        }

        response
    }

    /// Returns the [MongoResponse] to a [MongoRequest].
    #[async_recursion]
    async fn handle_request(
        database: &Database,
        mongo_request: MongoRequest,
    ) -> Result<MongoResponse, Error> {
        let collection_name = &mongo_request.collection_name;

        match mongo_request.request_type {
            MongoRequestType::Get{ filter, options } => {
                MongoRequest::handle_get(database, collection_name, filter.clone(), options)
                    .await
                    .map(|documents| MongoResponse::Get(documents))
            }
            MongoRequestType::Insert{ documents, options } => {
                MongoRequest::handle_insert(database, collection_name, documents.clone(), options)
                    .await
                    .map(|result| MongoResponse::Insert(result))
            }
            MongoRequestType::Update{ filter, update, options } => MongoRequest::handle_update(
                database,
                collection_name,
                filter.clone(),
                update.clone(),
                options
            )
            .await
            .map(|result| MongoResponse::Update(result)),
            MongoRequestType::Delete{ filter, options } => {
                MongoRequest::handle_delete(database, collection_name, filter.clone(), options)
                    .await
                    .map(|result| MongoResponse::Delete(result))
            }
            MongoRequestType::Chain(initial_request, chain) => {
                MongoRequest::handle_chain(
                    database,
                    collection_name,
                    initial_request,
                    chain,
                )
                .await
            }
        }
    }

    /// Sends a list of requests to the given [Database].
    pub async fn send_requests(
        database: Database,
        requests: Vec<Self>,
    ) -> Result<Vec<MongoResponse>, Message> {
        let mut responses: Vec<MongoResponse> = vec![];

        for request in requests {
            match MongoRequest::handle_request(&database, request).await {
                Ok(result) => {
                    responses.push(result);
                }
                Err(err) => {
                    return Err(Message::Error(err));
                }
            }
        }

        Ok(responses)
    }
}

/// The response to a [MongoRequest] sent to a [Database]:
/// - [Get](MongoResponse::Get), with a list of [Documents](Document);
/// - [Insert](MongoResponse::Insert), with the list of [inserted ids](InsertManyResult);
/// - [Update](MongoResponse::Update), with the [update results](UpdateResult);
/// - [Delete](MongoResponse::Delete), with the [number of deleted records](DeleteResult).
pub enum MongoResponse {
    Get(Vec<Document>),
    Insert(InsertManyResult),
    Update(UpdateResult),
    Delete(DeleteResult),
}

unsafe impl Send for MongoResponse {}
unsafe impl Sync for MongoResponse {}
