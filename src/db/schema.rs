table! {
    users (id) {
        id -> Text,
        created_at -> Text,
    }
}

table! {
    users_subscriptions (id) {
        id -> Integer,
        user_id -> Text,
        subreddit -> Text,
        last_sent_at -> Nullable<Text>,
    }
}

joinable!(users_subscriptions -> users (user_id));

allow_tables_to_appear_in_same_query!(
    users,
    users_subscriptions,
);
