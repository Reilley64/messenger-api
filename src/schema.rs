// @generated automatically by Diesel CLI.

diesel::table! {
    group_users (id) {
        id -> Int8,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        group_id -> Int8,
        user_id -> Int8,
        is_admin -> Bool,
        #[max_length = 255]
        nickname -> Nullable<Varchar>,
    }
}

diesel::table! {
    groups (id) {
        id -> Int8,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        #[max_length = 255]
        name -> Nullable<Varchar>,
        message_request_id -> Nullable<Int8>,
    }
}

diesel::table! {
    message_content (message_id, user_id) {
        message_id -> Int8,
        user_id -> Int8,
        content -> Text,
    }
}

diesel::table! {
    message_requests (id) {
        id -> Int8,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        source_id -> Int8,
        destination_id -> Int8,
        approved_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    messages (id) {
        id -> Int8,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        group_id -> Int8,
        source_id -> Int8,
        #[max_length = 255]
        idempotency_key -> Nullable<Varchar>,
    }
}

diesel::table! {
    user_push_subscriptions (id) {
        id -> Int8,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        user_id -> Int8,
        endpoint -> Text,
        p256dh -> Text,
        auth -> Text,
    }
}

diesel::table! {
    users (id) {
        id -> Int8,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        #[max_length = 255]
        sub -> Varchar,
        #[max_length = 255]
        email -> Varchar,
        #[max_length = 255]
        phone_number -> Varchar,
        #[max_length = 255]
        first_name -> Varchar,
        #[max_length = 255]
        last_name -> Varchar,
        #[max_length = 255]
        display_name -> Nullable<Varchar>,
        #[max_length = 392]
        public_key -> Varchar,
    }
}

diesel::joinable!(group_users -> groups (group_id));
diesel::joinable!(group_users -> users (user_id));
diesel::joinable!(groups -> message_requests (message_request_id));
diesel::joinable!(messages -> groups (group_id));
diesel::joinable!(messages -> users (source_id));
diesel::joinable!(user_push_subscriptions -> users (user_id));

diesel::allow_tables_to_appear_in_same_query!(
    group_users,
    groups,
    message_content,
    message_requests,
    messages,
    user_push_subscriptions,
    users,
);
