use directories::ProjectDirs;
use json::JsonValue;
use mongodb::bson::Uuid;

use crate::{database, debug_message, scene::Globals, utils::errors::Error};

pub async fn delete_drawing_offline(id: Uuid) -> Result<(), Error> {
    let proj_dirs = ProjectDirs::from("", "CharMe", "Chartsy")
        .ok_or(debug_message!("Unable to find project directory.").into())?;

    let drawings_path = proj_dirs.data_local_dir().join("drawings.json");
    let drawings = tokio::fs::read_to_string(drawings_path.clone())
        .await
        .map_err(|err| debug_message!("{}", err).into())?;

    let mut drawings = json::parse(&drawings).map_err(|err| debug_message!("{}", err).into())?;
    if let JsonValue::Array(ref mut drawings) = drawings {
        drawings.retain(|drawing| match drawing {
            JsonValue::Object(drawing) => {
                if let Some(JsonValue::String(drawing_id)) = drawing.get("id") {
                    let drawing_id = Uuid::parse_str(drawing_id);

                    drawing_id.is_ok_and(|drawing_id| id != drawing_id)
                } else {
                    false
                }
            }
            _ => false,
        });
    }

    tokio::fs::write(drawings_path, json::stringify(drawings))
        .await
        .map_err(|err| debug_message!("{}", err).into())?;

    let drawing_path = proj_dirs.data_local_dir().join(id.to_string());
    tokio::fs::remove_dir_all(drawing_path)
        .await
        .map_err(|err| debug_message!("{}", err).into())?;

    Ok(())
}

pub async fn delete_drawing_online(id: Uuid, globals: &Globals) -> Result<(), Error> {
    let user_id = globals
        .get_user()
        .ok_or(debug_message!("No user logged in.").into())?
        .get_id();

    database::drawing::delete_drawing(id, globals).await?;

    database::base::delete_data(format!("/{}/{}.webp", user_id, id)).await
}
