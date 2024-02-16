use std::fmt::{Debug};
use async_recursion::async_recursion;
use iced::Command;
use mongodb::{Client, Collection, Database, options::ClientOptions};
use mongodb::bson::Document;
use mongodb::error::{ErrorKind};
use mongodb::results::{DeleteResult, InsertManyResult, UpdateResult};
use crate::scene::{Action, Message};

use crate::config::{MONGO_NAME, MONGO_PASS};

/// Attempts to connect to the mongo [Database].
///
/// Returns an error upon failure.
pub async fn connect_to_mongodb() -> Result<Database, mongodb::error::Error>
    where
        Client: Send + 'static,
{
    let client_options = ClientOptions::parse(
        format!("mongodb+srv://{}:{}@cluster0.jwkwr.mongodb.net/?retryWrites=true&w=majority", MONGO_NAME, MONGO_PASS)
    ).await?;
    let client = Client::with_options(client_options)?;

    Ok(client.database("chartsy"))
}

/// The four [Database] request types:
/// - [Get](MongoRequestType::Get), with the filter set in a [Document].
/// - [Insert](MongoRequestType::Insert), with the requested [Documents](Document).
/// - [Update](MongoRequestType::Update), with the filter [Document] and the replacement.
/// - [Delete](MongoRequestType::Delete), with the filter [Document].
/// - [Chain](MongoRequestType::Chain), which can chain multiple [MongoRequests](MongoRequest); the first value is
/// the initial [MongoRequestType], and the second is a vector of pairs of a [Document], and a function that takes
/// the [MongoResponse] of the previous request and the document, and returns the next request; if an
/// error took place, the chain can also be halted by setting the next request as [None].
#[derive(Debug, Clone)]
pub enum MongoRequestType
{
    Get(Document),
    Insert(Vec<Document>),
    Update(Document, Document),
    Delete(Document),
    Chain(Box<Self>, Vec<(Document, fn(MongoResponse, Document) -> Option<MongoRequest>)>),
}

/// A request to be sent to a mongo [Database].
///
/// Contains the name of the altered [Collection], and the [request type](MongoRequestType).
#[derive(Debug, Clone)]
pub struct MongoRequest {
    collection_name: String,
    request_type: MongoRequestType,
}

impl MongoRequest {
    /// Creates a new [MongoRequest] using the given data.
    pub fn new(collection_name: String, request_type: MongoRequestType) -> Self {
        MongoRequest {
            collection_name,
            request_type
        }
    }

    /// Sends a [Get](MongoRequestType::Get) request to the given [Database] and returns
    /// a list of the results.
    async fn handle_get(database: &Database, collection_name: &String, filter: Document)
        -> Result<Vec<Document>, mongodb::error::Error> {
        let collection :Collection<Result<Document, mongodb::error::Error>>= database.collection(&*collection_name);
        let cursor = collection.find(Some(filter), None).await;

        match cursor {
            Ok(mut cursor) => {
                let mut vec :Vec<Document>= vec![];
                let res :Result<Vec<Document>, mongodb::error::Error>;

                loop {
                    let exists = cursor.advance().await;

                    match exists {
                        Ok(exists) => {
                            if exists {
                                let value = Document::try_from(cursor.current());
                                match value {
                                    Ok(document) => vec.push(document),
                                    Err(err) => {
                                        res = Err(mongodb::error::Error::from(err));
                                        break;
                                    }
                                }
                            } else {
                                res = Ok(vec.clone());
                                break;
                            }
                        }
                        Err(err) => {
                            res = Err(err);
                            break;
                        }
                    }
                }

                res
            }
            Err(err) => {
                Err(err)
            }
        }
    }

    /// Sends an [Insert](MongoRequestType::Insert) request to the given [Database] and returns
    /// the [inserted ids](InsertManyResult).
    async fn handle_insert(database: &Database, collection_name: &String, documents: Vec<Document>)
        -> Result<InsertManyResult, mongodb::error::Error>
    {
        let collection :Collection<Document>= database.collection(&*collection_name);
        collection.insert_many(documents, None).await
    }

    /// Sends an [Update](MongoRequestType::Update) request to the given [Database] and returns
    /// the [results](UpdateResult).
    async fn handle_update(database: &Database, collection_name: &String, filter: Document, update: Document)
        -> Result<UpdateResult, mongodb::error::Error>
    {
        let collection :Collection<Document>= database.collection(&*collection_name);
        collection.update_many(filter.clone(), update.clone(), None).await
    }

    /// Sends a [Delete](MongoRequestType::Delete) request to the given [Database] and returns
    /// the [number of deleted records](DeleteResult).
    async fn handle_delete(database: &Database, collection_name: &String, filter: Document)
        -> Result<DeleteResult, mongodb::error::Error>
    {
        let collection :Collection<Document>= database.collection(&*collection_name);
        collection.delete_many(filter.clone(), None).await
    }

    /// Sends a chain of requests to the given [Database] and returns the final [MongoResponse].
    #[async_recursion]
    async fn handle_chain(database: &Database, collection_name: &String, initial_request: Box<MongoRequestType>, chain: Vec<(Document, fn(MongoResponse, Document) -> Option<MongoRequest>)>)
        -> Result<Box<MongoResponse>, mongodb::error::Error>
    {
        let mut request = Some(MongoRequest { collection_name: collection_name.clone(), request_type: *initial_request });
        let mut response :Result<MongoResponse, mongodb::error::Error>= MongoRequest::handle_request(database, request.unwrap()).await;

        for transition in chain {
            if let Ok(res) = response {
                request = transition.1(res, transition.0);

                if let Some(req) = request {
                    response = MongoRequest::handle_request(database, req).await;
                } else {
                    response = Err(ErrorKind::Shutdown.into());
                    break;
                }
            } else {
                break;
            }

        }

        response.map(|res| Box::new(res))
    }

    /// Returns the [MongoResponse] to a [MongoRequest].
    #[async_recursion]
    async fn handle_request(database: &Database, mongo_request: MongoRequest)
        -> Result<MongoResponse, mongodb::error::Error>
    {
        let collection_name = &mongo_request.collection_name;

        match mongo_request.request_type {
            MongoRequestType::Get(filter) => MongoRequest::handle_get(database, collection_name, filter.clone()).await.map(|documents| MongoResponse::Get(documents)),
            MongoRequestType::Insert(documents) => MongoRequest::handle_insert(database, collection_name, documents.clone()).await.map(|result| MongoResponse::Insert(result)),
            MongoRequestType::Update(filter, update) => MongoRequest::handle_update(database, collection_name, filter.clone(), update.clone()).await.map(|result| MongoResponse::Update(result)),
            MongoRequestType::Delete(filter) => MongoRequest::handle_delete(database, collection_name, filter.clone()).await.map(|result| MongoResponse::Delete(result)),
            MongoRequestType::Chain(initial_request, chain) => MongoRequest::handle_chain(database, collection_name, initial_request.clone(), chain).await.map(|response| MongoResponse::Chain(response)),
        }
    }

    /// Sends a list of requests to the given [Database].
    ///
    /// The requests field is a tuple comprised of:
    /// - a [Vec] of [MongoRequests](MongoRequest);
    /// - a function that takes the [Vec] of [MongoResponses](MongoResponse) and returns a [Message](Action).
    pub fn send_requests(database: Database, requests: (Vec<Self>, fn(Vec<MongoResponse>) -> Box<dyn Action>))
                        -> Command<Message> {

        Command::perform(
            async move {
                let mut responses :Vec<MongoResponse>= vec![];

                for request in requests.0 {
                    match MongoRequest::handle_request(&database, request).await {
                        Ok(result) => {
                            responses.push(result);
                        },
                        Err(err) => {
                            return Err(err);
                        },
                    }
                }

                Ok(responses)
            },
                move |responses| {
                match responses {
                    Ok(responses) => {
                        Message::DoAction((requests.1)(responses))
                    }
                    Err(err) => {
                        Message::Error(format!("Error accessing mongo database: {}", err))
                    }
                }
            }
        )

    }
}

/// The response to a [MongoRequest] sent to a [Database]:
/// - [Get](MongoResponse::Get), with a list of [Documents](Document);
/// - [Insert](MongoResponse::Insert), with the list of [inserted ids](InsertManyResult);
/// - [Update](MongoResponse::Update), with the [update results](UpdateResult);
/// - [Delete](MongoResponse::Delete), with the [number of deleted records](DeleteResult);
/// - [Chain](MongoResponse::Chain), with the [MongoResponse] to the final [MongoRequest].
pub enum MongoResponse {
    Get(Vec<Document>),
    Insert(InsertManyResult),
    Update(UpdateResult),
    Delete(DeleteResult),
    Chain(Box<MongoResponse>),
}

unsafe impl Send for MongoResponse {}
unsafe impl Sync for MongoResponse {}