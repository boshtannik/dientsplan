use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use crate::schema::{absences, assignments, buildings, people, periods, rooms};

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = buildings)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct BuildingRow { pub id: i32, pub name: String, pub active: i32, pub public_token: String }

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = rooms)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct RoomRow { pub id: i32, pub building_id: i32, pub key: String, pub name: String, pub required: i32, pub gender: String, pub active: i32 }

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = people)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct PersonRow {
    pub id: i32, pub name: String, pub gender: String,
    pub building_id: Option<i32>, pub active: i32,
    pub token: String, pub role: String,
    pub created_at: String, pub used_at: Option<String>,
    pub duty_offset: i32,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = periods)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct PeriodRow { pub id: i32, pub name: String, pub start_date: String, pub num_days: i32, pub is_current: i32, pub building_id: i32 }

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = assignments)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct AssignmentRow { pub id: i32, pub period_id: i32, pub person_id: i32, pub room_id: i32, pub day: i32 }

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = absences)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct AbsenceRow { pub id: i32, pub person_id: i32, pub start_date: String, pub end_date: String, pub reason: String }

#[derive(Debug, Insertable)]
#[diesel(table_name = buildings)]
pub struct NewBuilding<'a> { pub name: &'a str }

#[derive(Debug, Insertable)]
#[diesel(table_name = rooms)]
pub struct NewRoom<'a> { pub building_id: i32, pub key: &'a str, pub name: &'a str, pub required: i32, pub gender: &'a str }

#[derive(Debug, Insertable)]
#[diesel(table_name = people)]
pub struct NewPerson<'a> {
    pub name: &'a str, pub gender: &'a str, pub building_id: Option<i32>,
    pub token: &'a str, pub role: &'a str,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = periods)]
pub struct NewPeriod<'a> { pub name: &'a str, pub start_date: &'a str, pub num_days: i32, pub building_id: i32 }

#[derive(Debug, Insertable)]
#[diesel(table_name = assignments)]
pub struct NewAssignment { pub period_id: i32, pub person_id: i32, pub room_id: i32, pub day: i32 }

#[derive(Debug, Insertable)]
#[diesel(table_name = absences)]
pub struct NewAbsence<'a> { pub person_id: i32, pub start_date: &'a str, pub end_date: &'a str, pub reason: &'a str }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Building { pub id: i64, pub name: String, pub active: bool }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Room { pub id: i64, pub building_id: i64, pub key: String, pub name: String, pub required: i64, pub gender: String, pub active: bool }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Person {
    pub id: i64, pub name: String, pub gender: String,
    pub building_id: Option<i64>, pub active: bool,
    pub token: String, pub role: String,
    pub created_at: String, pub used_at: Option<String>,
    pub duty_offset: i64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Period { pub id: i64, pub name: String, pub start_date: String, pub num_days: i64, pub is_current: bool, pub building_id: i64 }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Assignment {
    pub id: i64, pub period_id: i64, pub person_id: i64, pub person_name: String, pub person_gender: String,
    pub room_id: i64, pub room_name: String, pub room_key: String, pub building_id: i64, pub day: i32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Absence { pub id: i64, pub person_id: i64, pub person_name: String, pub start_date: String, pub end_date: String, pub reason: String }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankingEntry { pub person: Person, pub total_duties: i64, pub rank: usize }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaySchedule { pub day: i32, pub date: String, pub assignments: Vec<Assignment> }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonStat { pub person: Person, pub total_duties: i64, pub by_room: Vec<(String, i64)> }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Unfulfilled { pub room_name: String, pub day: i32, pub date: String, pub reason: String }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeriodView {
    pub period: Period, pub building: Building, pub rooms: Vec<Room>,
    pub days: Vec<DaySchedule>, pub person_stats: Vec<PersonStat>,
    pub absences: Vec<Absence>, pub unfulfilled: Vec<Unfulfilled>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonHistory { pub person: Person, pub periods: Vec<PeriodHistory> }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeriodHistory { pub period: Period, pub assignments: Vec<Assignment> }

impl BuildingRow { pub fn into_api(self) -> Building { Building { id: self.id as i64, name: self.name, active: self.active == 1 } } }
impl RoomRow { pub fn into_api(self) -> Room { Room { id: self.id as i64, building_id: self.building_id as i64, key: self.key, name: self.name, required: self.required as i64, gender: self.gender, active: self.active == 1 } } }
impl PersonRow {
    pub fn into_api(self) -> Person {
        Person {
            id: self.id as i64, name: self.name, gender: self.gender,
            building_id: self.building_id.map(|x| x as i64), active: self.active == 1,
            token: self.token, role: self.role,
            created_at: self.created_at, used_at: self.used_at,
            duty_offset: self.duty_offset as i64,
        }
    }
}
impl PeriodRow { pub fn into_api(self) -> Period { Period { id: self.id as i64, name: self.name, start_date: self.start_date, num_days: self.num_days as i64, is_current: self.is_current == 1, building_id: self.building_id as i64 } } }

#[derive(Debug, Deserialize)]
pub struct CreateBuilding { pub name: String }
#[derive(Debug, Deserialize)]
pub struct CreateRoom { pub key: String, pub name: String, pub required: i64, pub gender: String }
#[derive(Debug, Deserialize)]
pub struct UpdateRoom {
    pub key: Option<String>,
    pub name: Option<String>,
    pub required: Option<i64>,
    pub gender: Option<String>,
}
#[derive(Debug, Deserialize)]
pub struct CreatePerson {
    pub name: Option<String>,
    pub gender: Option<String>,
    pub building_id: i64,
    pub role: Option<String>,
}
#[derive(Debug, Deserialize)]
pub struct CreatePeriod { pub name: String, pub start_date: String, pub num_days: i64, pub building_id: i64 }
#[derive(Debug, Deserialize)]
pub struct CreateAbsence { pub person_id: i64, pub start_date: String, pub end_date: String, pub reason: String }
#[derive(Debug, Deserialize)]
pub struct UpdatePeriod { pub name: Option<String>, pub start_date: Option<String>, pub num_days: Option<i64> }
#[derive(Debug, Deserialize)]
pub struct UpdatePerson { pub name: Option<String>, pub gender: Option<String>, pub role: Option<String> }
#[derive(Debug, Deserialize)]
pub struct ActivateToken { pub token: String, pub name: Option<String>, pub gender: Option<String> }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthInfo { pub building_id: i64, pub building_name: String, pub role: String, pub name: Option<String> }

#[derive(Debug, Clone)]
pub struct AuthContext {
    pub building_id: i64,
    pub role: String,
    pub name: Option<String>,
}

impl AuthContext {
    pub fn require_role(&self, required: &str) -> Result<(), StatusCode> {
        if self.role == required || self.role == "superadmin" {
            Ok(())
        } else {
            Err(StatusCode::FORBIDDEN)
        }
    }
}

use axum::http::StatusCode;

#[derive(Debug, Deserialize)]
pub struct SetAssignment { pub person_id: i64, pub room_id: i64, pub day: i32 }
#[derive(Debug, Deserialize)]
pub struct BulkAssignments { pub assignments: Vec<SetAssignment> }

#[derive(Debug, Deserialize)]
pub struct FillPeriodRequest {
    pub person_id: i64,
    pub room_id: i64,
    pub from_day: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct ClearRoomRequest {
    pub room_id: i64,
    pub day: Option<i32>,
}
