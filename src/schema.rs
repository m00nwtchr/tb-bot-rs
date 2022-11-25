#![allow(non_snake_case)]
// @generated automatically by Diesel CLI.

diesel::table! {
    GuildTomes (id) {
        id -> Unsigned<Integer>,
        guild -> Unsigned<Bigint>,
        source -> Text,
    }
}
