use iced::Command;
use mongodb::{Client, Collection, Database, options::ClientOptions};
use mongodb::bson::Document;
use mongodb::results::{DeleteResult, InsertManyResult, UpdateResult};
use crate::scene::{Action, Message};

use crate::config::{MONGO_NAME, MONGO_PASS};

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

#[derive(Debug, Clone)]
pub enum MongoRequestType {
    Get(Document),
    Insert(Vec<Document>),
    Update(Document, Document),
    Delete(Document),
}

#[derive(Debug, Clone)]
pub struct MongoRequest {
    collection_name: String,
    request_type: MongoRequestType,
}

impl MongoRequest {
    pub fn new(collection_name: String, request_type: MongoRequestType) -> Self {
        MongoRequest {
            collection_name,
            request_type
        }
    }

    async fn handle_get(database: &Database, collection_name: String, filter: Document)
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

    async fn handle_insert(database: &Database, collection_name: String, documents: Vec<Document>)
        -> Result<InsertManyResult, mongodb::error::Error>
    {
        let collection :Collection<Document>= database.collection(&*collection_name);
        collection.insert_many(documents, None).await
    }

    async fn handle_update(database: &Database, collection_name: String, filter: Document, update: Document)
        -> Result<UpdateResult, mongodb::error::Error>
    {
        let collection :Collection<Document>= database.collection(&*collection_name);
        collection.update_many(filter.clone(), update.clone(), None).await
    }

    async fn handle_delete(database: &Database, collection_name: String, filter: Document)
        -> Result<DeleteResult, mongodb::error::Error>
    {
        let collection :Collection<Document>= database.collection(&*collection_name);
        collection.delete_many(filter.clone(), None).await
    }

    pub fn send_requests(database: Database, requests: (Vec<Self>, fn(Vec<MongoResponse>) -> Box<dyn Action>))
                        -> Command<Message> {

        Command::perform(
            async move {
                let mut responses :Vec<MongoResponse>= vec![];

                for request in requests.0 {
                    match request.request_type {
                        MongoRequestType::Get(filter) => {
                            match MongoRequest::handle_get(&database, request.collection_name, filter).await {
                                Ok(documents) => {
                                    responses.push(MongoResponse::Get(documents));
                                }
                                Err(err) => {
                                    return Err(err);
                                }
                            }
                        }
                        MongoRequestType::Insert(documents) => {
                            match MongoRequest::handle_insert(&database, request.collection_name, documents).await {
                                Ok(result) => {
                                    responses.push(MongoResponse::Insert(result));
                                }
                                Err(err) => {
                                    return Err(err);
                                }
                            }
                        }
                        MongoRequestType::Update(filter, update) => {
                            match MongoRequest::handle_update(&database, request.collection_name, filter, update).await {
                                Ok(result) => {
                                    responses.push(MongoResponse::Update(result));
                                }
                                Err(err) => {
                                    return Err(err)
                                }
                            }
                        }
                        MongoRequestType::Delete(filter) => {
                            match MongoRequest::handle_delete(&database, request.collection_name, filter).await {
                                Ok(result) => {
                                    responses.push(MongoResponse::Delete(result));
                                }
                                Err(err) => {
                                    return Err(err)
                                }
                            }
                        }
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

pub enum MongoResponse {
    Get(Vec<Document>),
    Insert(InsertManyResult),
    Update(UpdateResult),
    Delete(DeleteResult),
}

unsafe impl Send for MongoResponse {}
unsafe impl Sync for MongoResponse {}