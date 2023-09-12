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
pub enum MongoRequest {
    Get(Document),
    Insert(Vec<Document>),
    Update(Document, Document),
    Delete(Document),
}

impl MongoRequest {
    pub fn send_request(database: Database, request: (String, MongoRequest, fn(MongoResponse) -> Box<dyn Action>))
                        -> Command<Message> {

        match request.1 {
            MongoRequest::Get(document) => {
                Command::perform(
                    async move {
                        let collection :Collection<Result<Document, mongodb::error::Error>>= database.collection(&*request.0);
                        let cursor = collection.find(Some(document), None).await;

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
                    },
                    move |res| {
                        match res {
                            Ok(res) => {
                                Message::DoAction((request.2)(MongoResponse::Get(res)))
                            }
                            Err(err) => {
                                Message::Error(format!("Error getting from mongodb: {}", err))
                            }
                        }
                    }
                )
            }
            MongoRequest::Insert(documents) => {
                Command::perform(
                    async move {
                        let collection :Collection<Document>= database.collection(&*request.0);
                        collection.insert_many(documents, None).await
                    },
                    move |res| {
                        match res {
                            Ok(res) => {
                                Message::DoAction((request.2)(MongoResponse::Insert(res)))
                            }
                            Err(err) => {
                                Message::Error(format!("Error inserting into mongodb: {}", err))
                            }
                        }
                    }
                )
            }
            MongoRequest::Update(filter, update) => {
                Command::perform(
                    async move {
                        let collection :Collection<Document>= database.collection(&*request.0);
                        collection.update_many(filter.clone(), update.clone(), None).await
                    },
                    move |res| {
                        match res {
                            Ok(res) => {
                                Message::DoAction((request.2)(MongoResponse::Update(res)))
                            }
                            Err(err) => {
                                Message::Error(format!("Error updating in mongodb: {}", err))
                            }
                        }
                    }
                )
            }
            MongoRequest::Delete(document) => {
                Command::perform(
                    async move {
                        let collection :Collection<Document>= database.collection(&*request.0);
                        collection.delete_many(document.clone(), None).await
                    },
                    move |res| {
                        match res {
                            Ok(res) => {
                                Message::DoAction((request.2)(MongoResponse::Delete(res)))
                            }
                            Err(err) => {
                                Message::Error(format!("Error deleting in mongodb: {}", err))
                            }
                        }
                    }
                )
            }
        }
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