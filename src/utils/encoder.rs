use std::convert::identity;
use std::io::Cursor;
use image::{DynamicImage, ImageFormat, RgbaImage};
use resvg::tiny_skia::Transform;
use resvg::usvg::fontdb;
use svg::Document;
use tokio::task;
use crate::debug_message;
use super::errors::Error;

pub async fn encode_svg(svg: Document, format: impl Into<String>) -> Result<Vec<u8>, Error>
{
    let format = format.into();

    task::spawn_blocking(move || {
        let svg_data = svg.to_string();

        if &*format == "svg" {
            return Ok(svg_data.as_bytes().to_vec());
        }

        let opt = resvg::usvg::Options::default();
        let tree = match resvg::usvg::Tree::from_str(
            &*svg_data, &opt, &fontdb::Database::default()
        ) {
            Ok(tree) => tree,
            Err(err) => {
                return Err(debug_message!("{}", err.to_string()).into());
            }
        };

        let mut pixmap = match resvg::tiny_skia::Pixmap::new(
            tree.size().width() as u32,
            tree.size().height() as u32
        ) {
            Some(pixmap) => pixmap,
            None => {
                return Err(debug_message!("Error initializing pixmap.").into())
            }
        };

        resvg::render(&tree, Transform::default(), &mut pixmap.as_mut());

        if &*format == "png" {
            return Ok(pixmap.data().to_vec());
        }

        let rgba_image = match RgbaImage::from_raw(
            pixmap.width(),
            pixmap.height(),
            pixmap.data().to_vec()
        ) {
            Some(image) => image,
            None => {
                return Err(debug_message!("Error reading rgba image.").into());
            }
        };

        let dyn_image = DynamicImage::ImageRgba8(rgba_image);

        let dyn_image = match &*format {
            "webp" | "tiff" | "bmp" => dyn_image,
            "jpg" | "jpeg" => DynamicImage::ImageRgb8(dyn_image.to_rgb8()),
            _ => {
                return Err(debug_message!("{} is not a valid image format", format).into());
            }
        };

        let image_format = match &*format {
            "webp" => ImageFormat::WebP,
            "bmp" => ImageFormat::Bmp,
            "tiff" => ImageFormat::Tiff,
            "jpg" | "jpeg" => ImageFormat::Jpeg,
            _ => {
                return Err(debug_message!("{} is not a valid image format", format).into());
            }
        };

        let mut buffer = Cursor::new(vec![]);
        dyn_image.write_to(&mut buffer, image_format).map_or_else(
            |err| Err(err.to_string().into()),
            move |()| Ok(buffer.into_inner())
        )
    }).await.map_or_else(|err| Err(err.to_string().into()), identity)
}