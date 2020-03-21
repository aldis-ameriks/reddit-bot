table! {
    dialogs (user_id) {
        user_id -> Text,
        command -> Text,
        step -> Text,
        data -> Text,
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
        send_on -> Integer,
        send_at -> Integer,
    }
}

joinable!(dialogs -> users (user_id));
joinable!(users_subscriptions -> users (user_id));

allow_tables_to_appear_in_same_query!(
    dialogs,
    users,
    users_subscriptions,
);
