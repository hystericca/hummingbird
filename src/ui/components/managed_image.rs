use std::sync::Arc;

use gpui::{
    App, ElementId, IntoElement, ObjectFit, Refineable, RenderImage, RenderOnce, StyleRefinement,
    Styled, StyledImage, Window, div, img,
};
use image::{Frame, ImageResult};
use smallvec::SmallVec;
use sqlx::SqlitePool;
use tracing::error;

use crate::{
    ui::{app::Pool, util::drop_image_from_app},
    util::rgb_to_bgr,
};

#[derive(Clone, Copy)]
pub enum ManagedImageKey {
    Album(i64),
}

impl ManagedImageKey {
    async fn retrieve(&self, pool: SqlitePool) -> anyhow::Result<Option<Arc<RenderImage>>> {
        match self {
            ManagedImageKey::Album(id) => {
                let Some((image_encoded,)): Option<(Vec<u8>,)> =
                    sqlx::query_as(include_str!("../../../queries/assets/find_album_art.sql"))
                        .bind(id)
                        .fetch_optional(&pool)
                        .await?
                else {
                    return Ok(None);
                };

                if image_encoded.is_empty() {
                    return Ok(None);
                }

                let image = crate::RUNTIME
                    .spawn_blocking(move || {
                        let mut image = image::load_from_memory(&image_encoded)?.to_rgba8();

                        rgb_to_bgr(&mut image);

                        let mut frames: SmallVec<[_; 1]> = SmallVec::new();
                        frames.push(Frame::new(image));

                        ImageResult::Ok(Some(Arc::new(RenderImage::new(frames))))
                    })
                    .await??;

                Ok(image)
            }
        }
    }
}

#[derive(IntoElement)]
pub struct ManagedImage {
    key: ManagedImageKey,
    id: ElementId,
    style: StyleRefinement,
    object_fit: ObjectFit,
}

impl ManagedImage {
    pub fn object_fit(mut self, object_fit: ObjectFit) -> Self {
        self.object_fit = object_fit;
        self
    }
}

impl Styled for ManagedImage {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for ManagedImage {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let key = self.key;
        let image = window
            .use_keyed_state(self.id, cx, move |_window, cx| {
                let pool = cx.global::<Pool>().0.clone();

                cx.spawn(async move |this, cx| {
                    let image = crate::RUNTIME
                        .spawn(async move { key.retrieve(pool).await })
                        .await
                        .unwrap();

                    if let Ok(Some(image)) = image {
                        this.update(cx, |this, cx| {
                            *this = Some(image);
                            cx.notify();
                        })
                        .ok();
                    } else if let Err(e) = image {
                        error!("Failed to retrieve image: {:?}", e)
                    }
                })
                .detach();

                cx.on_release(|this, cx| {
                    if let Some(image) = this.clone() {
                        drop_image_from_app(cx, image);
                    }
                })
                .detach();

                None::<Arc<RenderImage>>
            })
            .read(cx)
            .clone();

        if let Some(image) = image {
            let mut image = img(image).object_fit(self.object_fit);

            let style = image.style();
            style.refine(&self.style);

            image.into_any_element()
        } else {
            div().into_any_element()
        }
    }
}

pub fn managed_image(id: impl Into<ElementId>, key: ManagedImageKey) -> ManagedImage {
    ManagedImage {
        key,
        id: id.into(),
        style: StyleRefinement::default(),
        object_fit: ObjectFit::Cover,
    }
}
