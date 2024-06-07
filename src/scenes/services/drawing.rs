use std::sync::Arc;

use directories::ProjectDirs;
use iced::{
    advanced::widget::Text,
    alignment::Horizontal,
    widget::{
        scrollable::{Direction, Properties},
        Button, Column, Container, Row, Scrollable, Space, TextInput,
    },
    Alignment, Element, Length, Renderer,
};
use json::{object::Object, JsonValue};
use mongodb::{bson::Uuid, Database};
use rfd::AsyncFileDialog;
use svg::node::element::SVG;

use crate::{
    canvas::{
        canvas::Canvas,
        layer::CanvasMessage,
        tool::{self, Pending, Tool},
        tools::{
            brush::BrushPending,
            brushes::{airbrush::Airbrush, eraser::Eraser, pen::Pen, pencil::Pencil},
            circle::CirclePending,
            ellipse::EllipsePending,
            line::LinePending,
            polygon::PolygonPending,
            rect::RectPending,
            triangle::TrianglePending,
        },
    },
    database, debug_message,
    scene::{Globals, Message},
    scenes::{
        data::drawing::{ModalTypes, PostData, UpdatePostData},
        drawing::DrawingMessage,
        scenes::Scenes,
    },
    utils::{
        self,
        errors::Error,
        icons::{Icon, ToolIcon, ICON},
        theme::{self, Theme},
    },
    widgets::{Card, Close, Closeable, ComboBox, Grid},
};

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

pub async fn get_drawing_offline(
    id: Uuid,
) -> Result<
    (
        Vec<(Uuid, String)>,
        Vec<(Arc<dyn Tool>, Uuid)>,
        Vec<JsonValue>,
    ),
    Error,
> {
    let proj_dirs = ProjectDirs::from("", "CharMe", "Chartsy")
        .ok_or(debug_message!("Unable to find project directory.").into())?;
    let file_path = proj_dirs
        .data_local_dir()
        .join(String::from("./") + &*id.to_string() + "/data.json");
    let data = tokio::fs::read_to_string(file_path)
        .await
        .map_err(|err| debug_message!("{}", err).into())?;

    let data = json::parse(&*data).map_err(|err| debug_message!("{}", err).into())?;

    if let JsonValue::Object(data) = data.clone() {
        let mut layers = vec![];
        let mut tools = vec![];
        let mut json_tools = vec![];

        if let Some(JsonValue::Array(layer_array)) = data.get("layers") {
            layers = layer_array
                .iter()
                .filter_map(|json| {
                    if let JsonValue::Object(object) = json {
                        Some((
                            Uuid::parse_str(object.get("id").unwrap().as_str().unwrap()).unwrap(),
                            object.get("name").unwrap().as_str().unwrap().to_string(),
                        ))
                    } else {
                        None
                    }
                })
                .collect();
        }
        if let Some(JsonValue::Array(tool_list)) = data.get("tools") {
            json_tools = tool_list.clone();

            for tool in tool_list {
                if let JsonValue::Object(tool) = tool {
                    if let Some(tool) = tool::get_json(tool) {
                        tools.push(tool);
                    }
                }
            }
        }

        Ok((layers, tools, json_tools))
    } else {
        Ok((vec![], vec![], vec![]))
    }
}

pub async fn create_drawing_offline(id: Uuid, json_data: Object) -> Result<(), Error> {
    let proj_dirs = ProjectDirs::from("", "CharMe", "Chartsy")
        .ok_or(debug_message!("Unable to find project directory.").into())?;

    let drawings_path = proj_dirs.data_local_dir().join("drawings.json");
    let drawings = json::parse(
        &*tokio::fs::read_to_string(drawings_path.clone())
            .await
            .map_err(|err| debug_message!("{}", err).into())?,
    )
    .map_err(|err| debug_message!("{}", err).into())?;

    if let JsonValue::Array(mut drawings) = drawings {
        let mut drawing = Object::new();
        drawing.insert("id", JsonValue::String(id.to_string()));
        drawing.insert("name", JsonValue::String(String::from("New drawing")));

        drawings.push(JsonValue::Object(drawing));

        tokio::fs::write(drawings_path, json::stringify(JsonValue::Array(drawings)))
            .await
            .map_err(|err| debug_message!("{}", err).into())?;
    }

    let dir_path = proj_dirs
        .data_local_dir()
        .join(String::from("./") + &*id.to_string());
    tokio::fs::create_dir_all(dir_path.clone())
        .await
        .map_err(|err| debug_message!("{}", err).into())?;

    let drawing_path = dir_path.join("data.webp");

    let file_path = dir_path.join("data.json");
    tokio::fs::write(
        file_path,
        json::stringify(JsonValue::Object(json_data)).as_bytes(),
    )
    .await
    .map_err(|err| debug_message!("{}", err).into())?;

    let svg = crate::canvas::svg::SVG::new(&vec![Uuid::new()]).as_document();
    let webp = utils::encoder::encode_svg(svg, "webp").await?;

    tokio::fs::write(drawing_path, webp)
        .await
        .map_err(|err| debug_message!("{}", err).into())
}

pub async fn create_post(
    user_id: Uuid,
    data: &SVG,
    description: String,
    tags: Vec<String>,
    db: &Database,
) -> Result<(), Error> {
    let img = utils::encoder::encode_svg(data.clone(), "webp").await?;
    let post_id = Uuid::new();

    match database::base::upload_file(format!("/{}/{}.webp", user_id, post_id), img).await {
        Ok(()) => {}
        Err(err) => {
            return Err(err);
        }
    }

    database::drawing::create_post(&db, post_id, user_id, description, tags).await
}

pub async fn download_drawing(document: &SVG) -> Result<(), Error> {
    let file = AsyncFileDialog::new()
        .set_title("Save As...")
        .set_directory("~")
        .add_filter(
            "image",
            &["png", "jpg", "jpeg", "webp", "svg", "tiff", "bmp"],
        )
        .save_file()
        .await;

    match file {
        Some(handle) => {
            let name = handle.file_name().to_string();
            let format = name
                .split(".")
                .last()
                .ok_or(debug_message!("File needs to have a readable format.").into())?;
            let img = utils::encoder::encode_svg(document.clone(), &*format).await?;

            handle
                .write(img.as_slice())
                .await
                .map_err(|err| err.to_string().into())
        }
        None => Err(debug_message!("Error getting file.").into()),
    }
}

pub fn tools_section<'a>(current_tool_id: String) -> Element<'a, Message, Theme, Renderer> {
    let tool_button = |name, pending: Box<dyn Pending>| -> Element<'a, Message, Theme, Renderer> {
        let style = if current_tool_id == pending.id() {
            iced::widget::button::primary
        } else {
            iced::widget::button::secondary
        };

        Button::<Message, Theme, Renderer>::new(
            Text::new(name).font(ICON).line_height(1.0).size(25.0),
        )
        .style(style)
        .on_press(CanvasMessage::ChangeTool(pending).into())
        .padding(10.0)
        .into()
    };

    let geometry_section: Element<Message, Theme, Renderer> = Grid::new(vec![
        tool_button(ToolIcon::Line.to_string(), Box::new(LinePending::None)),
        tool_button(ToolIcon::Rectangle.to_string(), Box::new(RectPending::None)),
        tool_button(
            ToolIcon::Triangle.to_string(),
            Box::new(TrianglePending::None),
        ),
        tool_button(
            ToolIcon::Polygon.to_string(),
            Box::new(PolygonPending::None),
        ),
        tool_button(ToolIcon::Circle.to_string(), Box::new(CirclePending::None)),
        tool_button(
            ToolIcon::Ellipse.to_string(),
            Box::new(EllipsePending::None),
        ),
    ])
    .spacing(25.0)
    .padding(18.0)
    .into();

    let brushes_section: Element<Message, Theme, Renderer> = Grid::new(vec![
        tool_button(
            ToolIcon::Pencil.to_string(),
            Box::new(BrushPending::<Pencil>::None),
        ),
        tool_button(
            ToolIcon::FountainPen.to_string(),
            Box::new(BrushPending::<Pen>::None),
        ),
        tool_button(
            ToolIcon::Airbrush.to_string(),
            Box::new(BrushPending::<Airbrush>::None),
        ),
    ])
    .spacing(25.0)
    .padding(18.0)
    .into();

    let eraser_section: Element<Message, Theme, Renderer> = Grid::new(vec![tool_button(
        ToolIcon::Eraser.to_string(),
        Box::new(BrushPending::<Eraser>::None),
    )])
    .spacing(25.0)
    .padding(18.0)
    .into();

    Container::new(Scrollable::new(
        Column::with_children(vec![
            Text::new("Geometry")
                .horizontal_alignment(Horizontal::Center)
                .size(20.0)
                .into(),
            geometry_section,
            Text::new("Brushes")
                .horizontal_alignment(Horizontal::Center)
                .size(20.0)
                .into(),
            brushes_section,
            Text::new("Eraser")
                .horizontal_alignment(Horizontal::Center)
                .size(20.0)
                .into(),
            eraser_section,
        ])
        .padding(8.0)
        .spacing(15.0)
        .width(Length::Fill),
    ))
    .padding(2.0)
    .width(Length::Fill)
    .style(iced::widget::container::bordered_box)
    .height(Length::FillPortion(1))
    .into()
}

pub fn style_section<'a>(canvas: &Canvas) -> Element<'a, Message, Theme, Renderer> {
    Container::new(Scrollable::new(
        canvas
            .get_style()
            .view()
            .map(|update| CanvasMessage::UpdateStyle(update).into()),
    ))
    .padding(2.0)
    .width(Length::Fill)
    .style(iced::widget::container::bordered_box)
    .height(Length::FillPortion(1))
    .into()
}

pub fn layers_section<'a>(canvas: &'a Canvas) -> Element<'a, Message, Theme, Renderer> {
    let title = Row::with_children(vec![
        Text::new("Layers").size(20.0).width(Length::Fill).into(),
        Button::new(Text::new(Icon::Add.to_string()).size(20.0).font(ICON))
            .padding(0.0)
            .style(iced::widget::button::text)
            .on_press(CanvasMessage::AddLayer.into())
            .into(),
    ])
    .padding(8.0)
    .width(Length::Fill)
    .into();

    let layer = |id: &'a Uuid| -> Element<'a, Message, Theme, Renderer> {
        let style = if *id == *canvas.get_current_layer() {
            iced::widget::button::primary
        } else {
            iced::widget::button::secondary
        };

        let layer = &canvas.get_layers().get(id).unwrap();
        let layer_count = canvas.get_layers().len();

        Button::new(
            Row::with_children(vec![
                if let Some(new_name) = layer.get_new_name() {
                    TextInput::new("Write layer name...", &*new_name.clone())
                        .on_input(|input| CanvasMessage::UpdateLayerName(*id, input).into())
                        .on_submit(CanvasMessage::ToggleEditLayerName(*id).into())
                        .into()
                } else {
                    Row::with_children(vec![
                        Text::new(layer.get_name().clone())
                            .width(Length::Fill)
                            .into(),
                        Button::new(Text::new(Icon::Edit.to_string()).font(ICON))
                            .style(iced::widget::button::text)
                            .on_press(CanvasMessage::ToggleEditLayerName(*id).into())
                            .into(),
                    ])
                    .align_items(Alignment::Center)
                    .into()
                },
                Button::new(
                    Text::new(
                        if layer.is_visible() {
                            Icon::Visible
                        } else {
                            Icon::Hidden
                        }
                        .to_string(),
                    )
                    .font(ICON),
                )
                .style(iced::widget::button::text)
                .on_press(CanvasMessage::ToggleLayer(*id).into())
                .into(),
                if layer_count > 1 {
                    Button::new(Text::new(Icon::X.to_string()).font(ICON))
                        .style(iced::widget::button::text)
                        .on_press(CanvasMessage::RemoveLayer(*id).into())
                        .into()
                } else {
                    Space::with_width(Length::Shrink).into()
                },
            ])
            .align_items(Alignment::Center),
        )
        .width(Length::Fill)
        .style(style)
        .on_press(CanvasMessage::ActivateLayer(*id).into())
        .into()
    };

    Container::new(Scrollable::new(Column::with_children(vec![
        title,
        Column::with_children(
            canvas
                .get_layer_order()
                .iter()
                .map(|id| layer(id))
                .collect::<Vec<Element<Message, Theme, Renderer>>>(),
        )
        .padding(8.0)
        .spacing(5.0)
        .into(),
    ])))
    .padding(2.0)
    .width(Length::Fill)
    .style(iced::widget::container::bordered_box)
    .height(Length::FillPortion(1))
    .into()
}

pub fn menu_section<'a>(globals: &Globals) -> Element<'a, Message, Theme, Renderer> {
    Container::new(
        Column::with_children(vec![
            Space::with_height(Length::Fill).into(),
            Button::new(
                Text::new("Save")
                    .horizontal_alignment(Horizontal::Center)
                    .width(Length::Fill)
                    .size(20.0),
            )
            .on_press(CanvasMessage::Save.into())
            .width(Length::Fill)
            .padding(5.0)
            .into(),
            Space::with_height(Length::Fill).into(),
            if globals.get_db().is_some() && globals.get_user().is_some() {
                Button::new(
                    Text::new("Post")
                        .horizontal_alignment(Horizontal::Center)
                        .width(Length::Fill)
                        .size(20.0),
                )
                .on_press(DrawingMessage::ToggleModal(ModalTypes::PostPrompt).into())
            } else {
                Button::new(
                    Text::new("Post")
                        .horizontal_alignment(Horizontal::Center)
                        .width(Length::Fill)
                        .size(20.0),
                )
            }
            .padding(5.0)
            .width(Length::Fill)
            .into(),
            Space::with_height(Length::Fill).into(),
            Button::new(
                Text::new("Save as...")
                    .horizontal_alignment(Horizontal::Center)
                    .width(Length::Fill)
                    .size(20.0),
            )
            .on_press(DrawingMessage::SaveAs.into())
            .padding(5.0)
            .width(Length::Fill)
            .into(),
            Space::with_height(Length::Fill).into(),
            Button::new(
                Text::new("Delete")
                    .horizontal_alignment(Horizontal::Center)
                    .width(Length::Fill)
                    .size(20.0),
            )
            .style(iced::widget::button::danger)
            .on_press(DrawingMessage::DeleteDrawing.into())
            .padding(5.0)
            .width(Length::Fill)
            .into(),
            Space::with_height(Length::Fill).into(),
        ])
        .spacing(10.0)
        .align_items(Alignment::Center),
    )
    .padding(10.0)
    .style(iced::widget::container::bordered_box)
    .center_x(Length::Fill)
    .center_y(Length::FillPortion(1))
    .into()
}

pub fn underlay<'a>(
    canvas: &'a Canvas,
    tools_section: Element<'a, Message, Theme, Renderer>,
    style_section: Element<'a, Message, Theme, Renderer>,
    layers_section: Element<'a, Message, Theme, Renderer>,
    menu_section: Element<'a, Message, Theme, Renderer>,
) -> Element<'a, Message, Theme, Renderer> {
    Column::with_children(vec![
        Row::with_children(vec![
            Button::new(Text::new(Icon::Leave.to_string()).font(ICON).size(30.0))
                .padding(0.0)
                .style(iced::widget::button::text)
                .on_press(Message::ChangeScene(Scenes::Main(None)))
                .into(),
            if let Some(new_name) = canvas.get_new_name() {
                TextInput::new("Add name", new_name)
                    .on_input(|value| CanvasMessage::SetNewName(value).into())
                    .on_submit(CanvasMessage::ToggleEditName.into())
                    .size(30.0)
                    .into()
            } else {
                Text::new(canvas.get_name()).size(30.0).into()
            },
            if canvas.get_new_name().is_some() {
                Space::with_width(Length::Shrink).into()
            } else {
                Button::new(Text::new(Icon::Edit.to_string()).font(ICON).size(30.0))
                    .padding(0.0)
                    .style(iced::widget::button::text)
                    .on_press(CanvasMessage::ToggleEditName.into())
                    .into()
            },
        ])
        .spacing(10.0)
        .padding(10.0)
        .into(),
        Row::with_children(vec![
            Column::with_children(vec![tools_section.into(), style_section.into()])
                .width(Length::Fixed(250.0))
                .height(Length::Fill)
                .into(),
            Container::new(Scrollable::with_direction(
                canvas,
                Direction::Both {
                    vertical: Properties::default(),
                    horizontal: Properties::default(),
                },
            ))
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into(),
            Column::with_children(vec![layers_section.into(), menu_section.into()])
                .align_items(Alignment::Center)
                .width(Length::Fixed(250.0))
                .height(Length::Fill)
                .into(),
        ])
        .padding(0)
        .spacing(20)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_items(Alignment::Center)
        .into(),
    ])
    .into()
}

pub fn post_prompt<'a>(post_data: &'a PostData) -> Element<'a, Message, Theme, Renderer> {
    Closeable::new(
        Card::new(
            Text::new("Create a new post"),
            Column::with_children(vec![
                Text::new("Description:").into(),
                TextInput::new("Write description here...", &*post_data.get_description())
                    .on_input(|new_value| {
                        DrawingMessage::UpdatePostData(UpdatePostData::Description(new_value))
                            .into()
                    })
                    .into(),
                Text::new("Tags:").into(),
                Grid::new(
                    post_data
                        .get_post_tags()
                        .iter()
                        .enumerate()
                        .map(|(index, tag)| {
                            Container::new(
                                Row::with_children(vec![
                                    Text::new(tag.get_name().clone())
                                        .style(theme::text::dark)
                                        .into(),
                                    Close::new(Into::<Message>::into(
                                        DrawingMessage::UpdatePostData(UpdatePostData::RemoveTag(
                                            index,
                                        )),
                                    ))
                                    .size(15.0)
                                    .into(),
                                ])
                                .spacing(5.0)
                                .align_items(Alignment::Center),
                            )
                            .style(theme::container::badge)
                            .padding(10.0)
                        }),
                )
                .padding(0.0)
                .spacing(5.0)
                .into(),
                Row::with_children(vec![
                    ComboBox::new(
                        post_data.get_all_tags().clone(),
                        "Add a new tag...",
                        &*post_data.get_tag_input(),
                        |tag| {
                            DrawingMessage::UpdatePostData(UpdatePostData::SelectedTag(tag)).into()
                        },
                    )
                    .width(Length::Fill)
                    .on_input(|new_value| {
                        DrawingMessage::UpdatePostData(UpdatePostData::TagInput(new_value)).into()
                    })
                    .into(),
                    Button::new(Text::new(Icon::Add.to_string()).size(30).font(ICON))
                        .on_press(
                            DrawingMessage::UpdatePostData(UpdatePostData::NewTag(
                                post_data.get_tag_input().clone(),
                            ))
                            .into(),
                        )
                        .style(iced::widget::button::text)
                        .padding(0)
                        .into(),
                ])
                .align_items(Alignment::Center)
                .spacing(10)
                .into(),
            ])
            .spacing(10.0)
            .height(Length::Shrink),
        )
        .footer(Button::new("Post").on_press(DrawingMessage::PostDrawing.into()))
        .width(Length::Fixed(300.0)),
    )
    .style(theme::closeable::Closeable::Transparent)
    .on_close(
        Into::<Message>::into(DrawingMessage::ToggleModal(ModalTypes::PostPrompt)),
        25.0,
    )
    .close_padding(7.0)
    .width(Length::Shrink)
    .height(Length::Shrink)
    .into()
}
