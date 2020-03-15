table! {
    commands (user_id) {
        user_id -> Text,
        command -> Text,
        created_at -> Text,
        updated_at -> Text,
        current_step -> Text,
        data -> Nullable<Text>,
    }
}

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

joinable!(commands -> users (user_id));
joinable!(users_subscriptions -> users (user_id));

allow_tables_to_appear_in_same_query!(commands, users, users_subscriptions,);
