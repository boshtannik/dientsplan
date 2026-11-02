diesel::table! {
    absences (id) {
        id -> Integer,
        person_id -> Integer,
        start_date -> Text,
        end_date -> Text,
        reason -> Text,
    }
}

diesel::table! {
    assignments (id) {
        id -> Integer,
        period_id -> Integer,
        person_id -> Integer,
        room_id -> Integer,
        day -> Integer,
    }
}

diesel::table! {
    buildings (id) {
        id -> Integer,
        name -> Text,
        active -> Integer,
        public_token -> Text,
    }
}

diesel::table! {
    people (id) {
        id -> Integer,
        name -> Text,
        gender -> Text,
        building_id -> Nullable<Integer>,
        active -> Integer,
        token -> Text,
        role -> Text,
        created_at -> Text,
        used_at -> Nullable<Text>,
        duty_offset -> Integer,
    }
}

diesel::table! {
    periods (id) {
        id -> Integer,
        name -> Text,
        start_date -> Text,
        num_days -> Integer,
        is_current -> Integer,
        building_id -> Integer,
    }
}

diesel::table! {
    rooms (id) {
        id -> Integer,
        building_id -> Integer,
        key -> Text,
        name -> Text,
        required -> Integer,
        gender -> Text,
        active -> Integer,
    }
}

diesel::joinable!(assignments -> people (person_id));
diesel::joinable!(assignments -> periods (period_id));
diesel::joinable!(assignments -> rooms (room_id));
diesel::joinable!(rooms -> buildings (building_id));

diesel::allow_tables_to_appear_in_same_query!(
    absences, assignments, buildings, people, periods, rooms,
);
