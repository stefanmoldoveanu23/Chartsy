use std::{future::Future, sync::Arc, time::Duration};

use iced::{
    widget::{image::Handle, Container, Image},
    Command, Element, Length, Pixels, Renderer, Size,
};
use image::{DynamicImage, RgbaImage};
use moka;
use mongodb::bson::Uuid;

use crate::{
    debug_message,
    scene::Message,
    widgets::{wait_panel::Appearance, WaitPanel},
};

use super::{errors::Error, theme::Theme};

/// An image represented by pixel data.
#[derive(Debug, Clone)]
pub struct PixelImage {
    /// The width of the [PixelImage].
    width: u32,

    /// The height of the [PixelImage].
    height: u32,

    /// The pixel data.
    data: Vec<u8>,
}

impl PixelImage {
    /// Initialize a [PixelImage].
    pub fn new(width: u32, height: u32, data: Vec<u8>) -> Self {
        PixelImage {
            width,
            height,
            data,
        }
    }

    pub fn get_width(&self) -> u32 {
        self.width
    }

    pub fn get_height(&self) -> u32 {
        self.height
    }

    pub fn get_data(&self) -> &Vec<u8> {
        &self.data
    }
}

impl From<DynamicImage> for PixelImage {
    fn from(value: DynamicImage) -> Self {
        Self::new(value.width(), value.height(), value.to_rgba8().to_vec())
    }
}

impl Into<DynamicImage> for PixelImage {
    fn into(self) -> DynamicImage {
        DynamicImage::ImageRgba8(RgbaImage::from_raw(self.width, self.height, self.data).unwrap())
    }
}

#[derive(Debug, Clone)]
pub struct Cache {
    cache_sync: moka::sync::Cache<Uuid, Arc<PixelImage>>,
    cache_async: moka::future::Cache<Uuid, Arc<PixelImage>>,
}

impl Cache {
    pub fn new() -> Self {
        Cache {
            cache_async: moka::future::Cache::builder()
                .time_to_idle(Duration::from_secs(60 * 60))
                .max_capacity(500 * 1024 * 1024)
                .build(),
            cache_sync: moka::sync::Cache::builder()
                .time_to_idle(Duration::from_secs(5 * 60))
                .max_capacity(50 * 1024 * 1024)
                .build(),
        }
    }

    /// Uploads image into cache.
    pub async fn insert(&self, id: Uuid, image: Arc<PixelImage>) -> Result<(), Error> {
        let cache_sync = self.cache_sync.clone();
        let cache_async = self.cache_async.clone();

        cache_async.insert(id, Arc::clone(&image)).await;

        tokio::task::spawn_blocking(move || {
            cache_sync.insert(id, Arc::clone(&image));

            ()
        })
        .await
        .map_err(|err| debug_message!("{}", err).into())?;

        Ok(())
    }

    /// Gets the handle of an image from its id.
    pub fn get_element<'a>(
        &self,
        id: Uuid,
        size: Size<Length>,
        backup_size: Size<Length>,
        text_size: impl Into<Option<Pixels>>,
    ) -> Element<'a, Message, Theme, Renderer> {
        match self.cache_sync.get(&id) {
            Some(pixels) => Image::new(Handle::from_rgba(
                pixels.get_width(),
                pixels.get_height(),
                pixels.get_data().clone(),
            ))
            .width(size.width)
            .height(size.height)
            .into(),
            None => {
                let mut appearance = Appearance::default();
                if let Some(text_size) = text_size.into() {
                    appearance = appearance.text_size(text_size);
                }

                Container::new(WaitPanel::new("Loading...").style(appearance))
                    .width(backup_size.width)
                    .height(backup_size.height)
                    .style(iced::widget::container::bordered_box)
                    .into()
            }
        }
    }

    pub fn get(&self, id: Uuid) -> Option<Arc<PixelImage>> {
        self.cache_sync.get(&id)
    }

    pub fn insert_if_not<F, I>(
        &self,
        items: impl IntoIterator<Item = I>,
        get_id: impl Fn(I) -> Uuid,
        load: impl Fn(I) -> F + Send + Copy + 'static,
    ) -> Command<Message>
    where
        F: Future<Output = Result<Arc<PixelImage>, Error>> + Send,
        I: Send + Clone + 'static,
    {
        Command::batch(items.into_iter().filter_map(move |item| {
            let id = (get_id)(item.clone());
            let cache_sync = self.cache_sync.clone();
            let cache_async = self.cache_async.clone();

            (!cache_sync.contains_key(&id)).then_some(Command::perform(
                async move {
                    let data = cache_async.try_get_with(id, (load)(item)).await?;

                    tokio::task::spawn_blocking(move || {
                        cache_sync.get_with(id, || Arc::clone(&data));
                    })
                    .await
                    .map_err(|err| Arc::new(debug_message!("{}", err).into()))
                },
                |result: Result<(), Arc<Error>>| match result {
                    Ok(_) => Message::None,
                    Err(err) => Message::Error(err.as_ref().clone()),
                },
            ))
        }))
    }
}
