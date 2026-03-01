use cntp_i18n::tr;
use gpui::prelude::{FluentBuilder, *};
use gpui::{
    App, AsyncApp, Entity, FontWeight, IntoElement, Pixels, SharedString, TextAlign, TextRun,
    Window, div, img, px,
};
use std::path::Path;

use crate::ui::components::drag_drop::{DragPreview, TrackDragData};
use crate::ui::components::icons::{
    PLAY, PLAYLIST_ADD, PLAYLIST_REMOVE, PLUS, STAR, STAR_FILLED, icon,
};
use crate::ui::components::menu::menu_separator;
use crate::ui::library::add_to_playlist::AddToPlaylist;
use crate::ui::models::PlaylistEvent;
use crate::{
    library::{
        db::{self, LibraryAccess},
        types::Track,
    },
    playback::{
        interface::{PlaybackInterface, replace_queue},
        queue::QueueItemData,
    },
    ui::{
        app::Pool,
        availability::is_track_available,
        components::{
            context::context,
            menu::{menu, menu_item},
        },
        models::{Models, PlaybackInfo},
        theme::Theme,
    },
};

use super::ArtistNameVisibility;

pub struct TrackPlaylistInfo {
    pub id: i64,
    pub item_id: i64,
}

pub struct TrackItem {
    pub track: Track,
    pub is_start: bool,
    pub artist_name_visibility: ArtistNameVisibility,
    pub is_liked: Option<i64>,
    pub hover_group: SharedString,
    left_field: TrackItemLeftField,
    album_art: Option<SharedString>,
    pl_info: Option<TrackPlaylistInfo>,
    add_to: Entity<AddToPlaylist>,
    show_add_to: Entity<bool>,
    vinyl_numbering: bool,
    max_track_num_str: Option<SharedString>,
    is_available: bool,
}

#[derive(Eq, PartialEq)]
pub enum TrackItemLeftField {
    TrackNum,
    Art,
}

fn measure_track_number_width(window: &mut Window, text: &SharedString) -> Pixels {
    let style = window.text_style();
    let font_size = style.font_size.to_pixels(window.rem_size());

    let run = TextRun {
        len: text.len(),
        font: style.font(),
        color: style.color,
        background_color: None,
        underline: None,
        strikethrough: None,
    };

    let line = window
        .text_system()
        .shape_line(text.clone(), font_size, &[run], None);

    line.x_for_index(line.len())
}

impl TrackItem {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        cx: &mut App,
        track: Track,
        is_start: bool,
        anv: ArtistNameVisibility,
        left_field: TrackItemLeftField,
        pl_info: Option<TrackPlaylistInfo>,
        vinyl_numbering: bool,
        max_track_num_str: Option<SharedString>,
    ) -> Entity<Self> {
        cx.new(|cx| {
            let show_add_to = cx.new(|_| false);
            let add_to = AddToPlaylist::new(cx, show_add_to.clone(), track.id);
            let track_id = track.id;

            let playlist_tracker = cx.global::<Models>().playlist_tracker.clone();

            cx.subscribe(&playlist_tracker, move |this: &mut Self, _, ev, cx| {
                if PlaylistEvent::PlaylistUpdated(1) == *ev {
                    this.is_liked = cx.playlist_has_track(1, track_id).unwrap_or_default();
                    cx.notify();
                }
            })
            .detach();

            Self {
                hover_group: format!("track-{}", track.id).into(),
                is_liked: cx.playlist_has_track(1, track.id).unwrap_or_default(),
                album_art: track
                    .album_id
                    .map(|v| format!("!db://album/{v}/thumb").into()),
                add_to,
                show_add_to,
                is_available: is_track_available(&track),
                track,
                is_start,
                artist_name_visibility: anv,
                left_field,
                pl_info,
                vinyl_numbering,
                max_track_num_str,
            }
        })
    }
}

impl Render for TrackItem {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        let track_num_width = self
            .max_track_num_str
            .as_ref()
            .map(|max_num_str| measure_track_number_width(window, max_num_str))
            .unwrap_or(px(22.0));
        let current_track = cx.global::<PlaybackInfo>().current_track.read(cx).clone();
        let is_available = self.is_available;

        let track_location = self.track.location.clone();
        let track_location_2 = self.track.location.clone();
        let track_location_for_drag = self.track.location.clone();
        let track_id = self.track.id;
        let album_id = self.track.album_id;
        let track_title_for_drag: SharedString = self.track.title.clone().into();

        let show_artist_name = self.artist_name_visibility != ArtistNameVisibility::Never
            && self.artist_name_visibility
                != ArtistNameVisibility::OnlyIfDifferent(self.track.artist_names.clone());

        let track = self.track.clone();

        let show_clone = self.show_add_to.clone();

        context(("context", self.track.id as usize))
            .with(
                div()
                    .flex()
                    .flex_col()
                    .w_full()
                    .id(self.track.id as usize)
                    .when(is_available, |this| {
                        this.on_click({
                            let track = self.track.clone();
                            let plid = self.pl_info.as_ref().map(|pl| pl.id);
                            move |_, _, cx| play_from_track(cx, &track, plid)
                        })
                    })
                    .when(!is_available, |this| this.cursor_default().opacity(0.5))
                    .child(self.add_to.clone())
                    .when(self.is_start, |this| {
                        this.child(
                            div()
                                .text_color(theme.text_secondary)
                                .text_sm()
                                .font_weight(FontWeight::SEMIBOLD)
                                // 22px (from track # width) + 18 + 11
                                .px(px(track_num_width.to_f64() as f32 + 18.0 + 13.0))
                                .border_b_1()
                                .w_full()
                                .border_color(theme.border_color)
                                .mt(px(18.0))
                                .pb(px(6.0))
                                .when_some(self.track.disc_number, |this, num| {
                                    if self.vinyl_numbering {
                                        let side = (b'A' + (num - 1) as u8) as char;
                                        let side = side.to_string(); // TODO: fix this upstream
                                        this.child(tr!("TRACK_SIDE", "Side {{side}}", side = side))
                                    } else {
                                        this.child(tr!("TRACK_DISC", "Disc {{num}}", num = num))
                                    }
                                }),
                        )
                    })
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .border_b_1()
                            .h(px(39.0))
                            .id(("track", self.track.id as u64))
                            .w_full()
                            .border_color(theme.border_color)
                            .when(is_available, |this| this.cursor_pointer())
                            .when(!is_available, |this| this.cursor_default())
                            .px(px(18.0))
                            .py(px(6.0))
                            .group(self.hover_group.clone())
                            .when(is_available, |this| {
                                this.hover(|this| this.bg(theme.nav_button_hover))
                                    .active(|this| this.bg(theme.nav_button_active))
                            })
                            // only handle drag when we're not in a playlist
                            // playlists have their own drag handler
                            .when(self.pl_info.is_none() && is_available, |this| {
                                this.on_drag(
                                    TrackDragData::from_track(
                                        track_id,
                                        album_id,
                                        track_location_for_drag,
                                        track_title_for_drag.clone(),
                                    ),
                                    move |_, _, _, cx| {
                                        DragPreview::new(cx, track_title_for_drag.clone())
                                    },
                                )
                            })
                            .when_some(current_track, |this, track| {
                                this.bg(if track == self.track.location {
                                    theme.queue_item_current
                                } else {
                                    theme.background_primary
                                })
                            })
                            .max_w_full()
                            .when(self.left_field == TrackItemLeftField::TrackNum, |this| {
                                this.child(
                                    div()
                                        .min_w(track_num_width)
                                        .flex_shrink_0()
                                        .text_align(TextAlign::Right)
                                        .mr(px(13.0))
                                        .text_color(theme.text_secondary)
                                        // TODO: handle these numerals better
                                        .child(format!(
                                            "{}",
                                            self.track.track_number.unwrap_or_default()
                                        )),
                                )
                            })
                            .when(self.left_field == TrackItemLeftField::Art, |this| {
                                this.child(
                                    div()
                                        .w(px(22.0))
                                        .h(px(22.0))
                                        .mr(px(12.0))
                                        .my_auto()
                                        .rounded(px(3.0))
                                        .bg(theme.album_art_background)
                                        .when_some(self.album_art.clone(), |this, art| {
                                            this.child(
                                                img(art).w(px(22.0)).h(px(22.0)).rounded(px(3.0)),
                                            )
                                        }),
                                )
                            })
                            .child(
                                div()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .overflow_x_hidden()
                                    .text_ellipsis()
                                    .child(self.track.title.clone()),
                            )
                            .child(
                                div()
                                    .id("like")
                                    .mr(px(-4.0))
                                    .ml_auto()
                                    .my_auto()
                                    .rounded_sm()
                                    .p(px(4.0))
                                    .child(
                                        icon(if self.is_liked.is_some() {
                                            STAR_FILLED
                                        } else {
                                            STAR
                                        })
                                        .size(px(14.0))
                                        .text_color(theme.text_secondary),
                                    )
                                    .invisible()
                                    .group(self.hover_group.clone())
                                    .when(is_available, |this| {
                                        this.group_hover(self.hover_group.clone(), |this| {
                                            this.visible()
                                        })
                                        .hover(|this| this.bg(theme.button_secondary_hover))
                                        .active(|this| this.bg(theme.button_secondary_active))
                                        .on_click(
                                            cx.listener(move |this, _, _, cx| {
                                                cx.stop_propagation();
                                                let is_liked = this.is_liked;
                                                let entity = cx.entity().clone();
                                                if is_liked.is_some() {
                                                    this.is_liked = None;
                                                }
                                                toggle_like(is_liked, track_id, entity, cx);
                                            }),
                                        )
                                    }),
                            )
                            .child(
                                div()
                                    .font_weight(FontWeight::LIGHT)
                                    .text_sm()
                                    .my_auto()
                                    .text_color(theme.text_secondary)
                                    .text_ellipsis()
                                    .overflow_x_hidden()
                                    .flex_shrink()
                                    .ml(px(12.0))
                                    .when(show_artist_name, |this| {
                                        this.when_some(
                                            self.track.artist_names.clone(),
                                            |this, v| this.child(v.0),
                                        )
                                    }),
                            )
                            .child(div().ml(px(12.0)).flex_shrink_0().child(format!(
                                "{}:{:02}",
                                self.track.duration / 60,
                                self.track.duration % 60
                            ))),
                    ),
            )
            .child(
                div().bg(theme.elevated_background).child(
                    menu()
                        .item(
                            menu_item("track_play", Some(PLAY), "Play", move |_, _, cx| {
                                let data = QueueItemData::new(
                                    cx,
                                    track_location.clone(),
                                    Some(track_id),
                                    album_id,
                                );
                                let playback_interface = cx.global::<PlaybackInterface>();
                                let queue_length = cx
                                    .global::<Models>()
                                    .queue
                                    .read(cx)
                                    .data
                                    .read()
                                    .expect("couldn't get queue")
                                    .len();
                                playback_interface.queue(data);
                                playback_interface.jump(queue_length);
                            })
                            .disabled(!is_available),
                        )
                        .item(
                            menu_item(
                                "track_play_from_here",
                                None::<&str>,
                                tr!("PLAY_FROM_HERE", "Play from here"),
                                {
                                    let plid = self.pl_info.as_ref().map(|pl| pl.id);
                                    move |_, _, cx| play_from_track(cx, &track, plid)
                                },
                            )
                            .disabled(!is_available),
                        )
                        .item(
                            menu_item(
                                "track_add_to_queue",
                                Some(PLUS),
                                tr!("ADD_TO_QUEUE", "Add to queue"),
                                move |_, _, cx| {
                                    let data = QueueItemData::new(
                                        cx,
                                        track_location_2.clone(),
                                        Some(track_id),
                                        album_id,
                                    );
                                    let playback_interface = cx.global::<PlaybackInterface>();
                                    playback_interface.queue(data);
                                },
                            )
                            .disabled(!is_available),
                        )
                        .item(menu_separator())
                        .item(
                            menu_item(
                                "track_add_to_playlist",
                                Some(PLAYLIST_ADD),
                                tr!("ADD_TO_PLAYLIST", "Add to playlist"),
                                move |_, _, cx| show_clone.write(cx, true),
                            )
                            .disabled(!is_available),
                        )
                        .when_some(self.pl_info.as_ref(), |menu, info| {
                            let playlist_id = info.id;
                            let item_id = info.item_id;
                            let playlist_tracker = cx.global::<Models>().playlist_tracker.clone();
                            let pool = cx.global::<Pool>().0.clone();

                            menu.item(
                                menu_item(
                                    "track_remove_from_playlist",
                                    Some(PLAYLIST_REMOVE),
                                    tr!("REMOVE_FROM_PLAYLIST", "Remove from playlist"),
                                    move |_, _, cx| {
                                        remove_from_playlist(
                                            item_id,
                                            playlist_id,
                                            pool.clone(),
                                            playlist_tracker.clone(),
                                            cx,
                                        );
                                    },
                                )
                                .disabled(!is_available),
                            )
                        }),
                ),
            )
    }
}

fn toggle_like(
    is_liked: Option<i64>,
    track_id: i64,
    entity: Entity<TrackItem>,
    cx: &mut Context<TrackItem>,
) {
    let pool = cx.global::<Pool>().0.clone();
    let playlist_tracker = cx.global::<Models>().playlist_tracker.clone();

    if let Some(item_id) = is_liked {
        // Optimistically clear liked state
        cx.notify();

        cx.spawn(async move |_, cx| {
            unlike_track(item_id, entity, playlist_tracker, pool, cx).await;
        })
        .detach();
    } else {
        cx.spawn(async move |_, cx| {
            like_track(track_id, entity, playlist_tracker, pool, cx).await;
        })
        .detach();
    }
}

async fn unlike_track(
    item_id: i64,
    entity: Entity<TrackItem>,
    playlist_tracker: Entity<crate::ui::models::PlaylistInfoTransfer>,
    pool: sqlx::SqlitePool,
    cx: &mut AsyncApp,
) {
    let task = crate::RUNTIME.spawn(async move { db::remove_playlist_item(&pool, item_id).await });

    match task.await {
        Ok(Ok(())) => {}
        Ok(Err(err)) => {
            tracing::error!("could not unlike song: {err:?}");
            let _ = entity.update(cx, |this, cx| {
                this.is_liked = Some(item_id);
                cx.notify();
            });
            return;
        }
        Err(err) => {
            tracing::error!("unlike task panicked: {err:?}");
            return;
        }
    }

    let _ = playlist_tracker.update(cx, |_, cx| {
        cx.emit(PlaylistEvent::PlaylistUpdated(1));
    });
}

async fn like_track(
    track_id: i64,
    entity: Entity<TrackItem>,
    playlist_tracker: Entity<crate::ui::models::PlaylistInfoTransfer>,
    pool: sqlx::SqlitePool,
    cx: &mut AsyncApp,
) {
    let task = crate::RUNTIME.spawn(async move { db::add_playlist_item(&pool, 1, track_id).await });

    let new_id = match task.await {
        Ok(Ok(id)) => id,
        Ok(Err(err)) => {
            tracing::error!("could not like song: {err:?}");
            return;
        }
        Err(err) => {
            tracing::error!("like task panicked: {err:?}");
            return;
        }
    };

    let _ = entity.update(cx, |this, cx| {
        this.is_liked = Some(new_id);
        cx.notify();
    });

    let _ = playlist_tracker.update(cx, |_, cx| {
        cx.emit(PlaylistEvent::PlaylistUpdated(1));
    });
}

fn remove_from_playlist(
    item_id: i64,
    playlist_id: i64,
    pool: sqlx::SqlitePool,
    playlist_tracker: Entity<crate::ui::models::PlaylistInfoTransfer>,
    cx: &mut App,
) {
    cx.spawn(async move |cx| {
        let task =
            crate::RUNTIME.spawn(async move { db::remove_playlist_item(&pool, item_id).await });

        match task.await {
            Ok(Ok(())) => {}
            Ok(Err(err)) => {
                tracing::error!("could not remove track from playlist: {err:?}");
                return;
            }
            Err(err) => {
                tracing::error!("remove-from-playlist task panicked: {err:?}");
                return;
            }
        }

        let _ = playlist_tracker.update(cx, |_, cx| {
            cx.emit(PlaylistEvent::PlaylistUpdated(playlist_id));
        });
    })
    .detach();
}

pub fn play_from_track(cx: &mut App, track: &Track, pl_id: Option<i64>) {
    if !is_track_available(track) {
        return;
    }

    let queue_items = if let Some(pl_id) = pl_id {
        let ids = cx
            .get_playlist_tracks(pl_id)
            .expect("failed to retrieve playlist track info");
        let paths = cx
            .get_playlist_track_files(pl_id)
            .expect("failed to retrieve playlist track paths");

        ids.iter()
            .zip(paths.iter())
            .filter(|(_, path)| Path::new(path).exists())
            .map(|((_, track, album), path)| {
                QueueItemData::new(cx, path.into(), Some(*track), Some(*album))
            })
            .collect()
    } else if let Some(album_id) = track.album_id {
        cx.list_tracks_in_album(album_id)
            .expect("Failed to retrieve tracks")
            .iter()
            .filter(|track| is_track_available(track))
            .map(|track| {
                QueueItemData::new(cx, track.location.clone(), Some(track.id), track.album_id)
            })
            .collect()
    } else {
        Vec::from([QueueItemData::new(
            cx,
            track.location.clone(),
            Some(track.id),
            track.album_id,
        )])
    };

    if queue_items.is_empty() {
        return;
    }

    replace_queue(queue_items.clone(), cx);

    let playback_interface = cx.global::<PlaybackInterface>();
    if let Some(index) = queue_items
        .iter()
        .position(|t| t.get_path() == &track.location)
    {
        playback_interface.jump_unshuffled(index)
    } else if !queue_items.is_empty() {
        playback_interface.jump_unshuffled(0);
    }
}
