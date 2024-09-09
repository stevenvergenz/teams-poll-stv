// @generated automatically by Diesel CLI.

diesel::table! {
    polloptions (poll_id, id) {
        poll_id -> Uuid,
        id -> Int4,
        #[max_length = 300]
        description -> Varchar,
    }
}

diesel::table! {
    polls (id) {
        id -> Uuid,
        #[max_length = 300]
        title -> Varchar,
        winner_count -> Int4,
        write_ins_allowed -> Bool,
        close_after_time -> Nullable<Timestamp>,
        close_after_votes -> Nullable<Int4>,
        owner_id -> Uuid,
        created_at -> Timestamp,
        closed_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    users (id) {
        id -> Uuid,
        #[max_length = 100]
        display_name -> Varchar,
    }
}

diesel::joinable!(polloptions -> polls (poll_id));
diesel::joinable!(polls -> users (owner_id));

diesel::allow_tables_to_appear_in_same_query!(
    polloptions,
    polls,
    users,
);
