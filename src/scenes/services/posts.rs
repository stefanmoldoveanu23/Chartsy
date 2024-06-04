use mongodb::bson::Uuid;

use crate::{database, debug_message, scene::Globals, utils::errors::Error};

pub async fn delete_post(id: Uuid, globals: &Globals) -> Result<(), Error> {
    let user_id = globals
        .get_user()
        .ok_or(debug_message!("User is not logged in.").into())?
        .get_id();

    database::posts::delete_post(id, globals).await?;

    database::base::delete_data(format!("/{}/{}.webp", user_id, id)).await
}
