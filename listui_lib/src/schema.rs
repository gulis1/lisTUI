// @generated automatically by Diesel CLI.

diesel::table! {
    playlist (id) {
        id -> Integer,
        title -> Text,
        yt_id -> Text,
    }
}

diesel::table! {
    track (id) {
        id -> Integer,
        title -> Text,
        yt_id -> Nullable<Text>,
        playlist_id -> Nullable<Integer>,
    }
}

diesel::joinable!(track -> playlist (playlist_id));

diesel::allow_tables_to_appear_in_same_query!(
    playlist,
    track,
);
