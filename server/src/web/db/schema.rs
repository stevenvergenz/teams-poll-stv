// @generated automatically by Diesel CLI.

diesel::table! {
    ballots (id) {
        id -> Int4,
        poll_id -> Uuid,
        user_id -> Uuid,
        created_at -> Timestamp,
    }
}

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

diesel::table! {
    votes (ballot_id, preference) {
        ballot_id -> Int4,
        preference -> Int4,
        option -> Int4,
    }
}

diesel::joinable!(ballots -> polls (poll_id));
diesel::joinable!(ballots -> users (user_id));
diesel::joinable!(polloptions -> polls (poll_id));
diesel::joinable!(polls -> users (owner_id));
diesel::joinable!(votes -> ballots (ballot_id));

diesel::allow_tables_to_appear_in_same_query!(
    ballots,
    polloptions,
    polls,
    users,
    votes,
);
