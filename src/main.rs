use axum::{
    Router,
    extract::{State, Path, Json, Query},
    routing::{get, post, put, delete},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tower_http::services::ServeDir;

use duty_scheduler::models::ClearRoomRequest;
use duty_scheduler::db::Db;
use duty_scheduler::models::*;
use duty_scheduler::AppError;

type S = Arc<Db>;

#[derive(Debug, Deserialize)]
struct TokenQuery { token: Option<String> }
#[derive(Debug, Deserialize)]
struct BuildingQuery { token: Option<String> }
#[derive(Debug, Deserialize)]
struct ViewQuery { token: Option<String>, building_id: i64 }
#[derive(Debug, Deserialize)]
struct AbsenceQuery { token: Option<String>, building_id: Option<i64>, period_id: Option<i64> }

async fn extract_auth(db: &Db, token: Option<&str>, required_role: Option<&str>) -> Result<AuthContext, StatusCode> {
    let token = token.ok_or(StatusCode::UNAUTHORIZED)?;
    let ctx = db.validate_token(token).map_err(|_| StatusCode::UNAUTHORIZED)?;
    if let Some(role) = required_role {
        ctx.require_role(role)?;
    }
    Ok(ctx)
}

#[tokio::main]
async fn main() {
    let db = Arc::new(Db::new("duties.db").expect("Failed to init DB"));

    let app = Router::new()
        // ─── Public (unauthenticated) ───
        .route("/api/auth/me", get(auth_me))
        .route("/api/auth/activate", post(auth_activate))
        .route("/api/activate", get(activate_check))

        // ─── Buildings ───
        .route("/api/buildings", get(list_buildings).post(create_building))
        .route("/api/buildings/:id", get(get_building).delete(delete_building))
        .route("/api/buildings/:id/rooms", get(list_rooms).post(create_room))
        .route("/api/buildings/:id/share", get(building_share))
        .route("/api/rooms/:id", put(update_room).delete(delete_room))

        // ─── Public share (no auth, by building public token) ───
        .route("/api/public/:token", get(public_info))
        .route("/api/public/:token/periods/:id/view", get(public_period_view))
        .route("/api/public/:token/people/:id/history", get(public_person_history))

        // ─── People (unified: person = token holder) ───
        .route("/api/people", get(list_people).post(create_person))
        .route("/api/people/all", get(list_all_people))
        .route("/api/people/:id", get(get_person).put(update_person).delete(delete_person))
        .route("/api/people/:id/history", get(person_history))

        // ─── Periods ───
        .route("/api/periods", get(list_periods).post(create_period))
        .route("/api/periods/current", get(current_period))
        .route("/api/periods/next", post(next_period))
        .route("/api/periods/:id", get(get_period).put(update_period).delete(delete_period))
        .route("/api/periods/:id/set-current", post(set_current_period))
        .route("/api/periods/:id/view", get(period_view))
        .route("/api/periods/:id/assignments", post(set_assignment).delete(remove_assignment))
        .route("/api/periods/:id/bulk", post(bulk_assignments))
        .route("/api/periods/:id/fill", post(fill_period))
        .route("/api/periods/:id/clear", post(clear_room))
        .route("/api/periods/:id/auto-fill", post(auto_fill))
        .route("/api/periods/:id/copy", post(copy_period))

        // ─── Absences ───
        .route("/api/absences", get(list_absences).post(create_absence))
        .route("/api/absences/:id", delete(delete_absence))

        // ─── Ranking ───
        .route("/api/ranking", get(ranking))

        .fallback_service(ServeDir::new("static").append_index_html_on_directories(true))
        .with_state(db);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("Duty Scheduler running on http://localhost:3000");
    axum::serve(listener, app).await.unwrap();
}

// ─── Auth ───

async fn auth_me(State(db): State<S>, q: Query<TokenQuery>) -> Result<Json<AuthInfo>, StatusCode> {
    let ctx = db.validate_token(q.token.as_deref().ok_or(StatusCode::UNAUTHORIZED)?).map_err(|_| StatusCode::UNAUTHORIZED)?;
    let building_name = if ctx.building_id > 0 {
        db.get_building(ctx.building_id).map(|b| b.name).unwrap_or_default()
    } else { "Общее".to_string() };
    Ok(Json(AuthInfo { building_id: ctx.building_id, building_name, role: ctx.role, name: ctx.name }))
}

#[derive(Debug, Deserialize)]
struct ActivateInput { token: String, name: Option<String>, gender: Option<String> }

async fn auth_activate(State(db): State<S>, Json(input): Json<ActivateInput>) -> Result<Json<AuthInfo>, StatusCode> {
    let ctx = db.activate_token(&input.token, input.name.as_deref(), input.gender.as_deref())?;
    let building_name = if ctx.building_id > 0 {
        db.get_building(ctx.building_id).map(|b| b.name).unwrap_or_default()
    } else { "Общее".to_string() };
    Ok(Json(AuthInfo { building_id: ctx.building_id, building_name, role: ctx.role, name: ctx.name }))
}

#[derive(Debug, Deserialize)]
struct ActivateCheckQuery { token: String }
#[derive(Debug, Serialize)]
struct ActivateCheckInfo { building_name: String, role: String, used: bool, person_name: Option<String>, person_gender: Option<String> }

async fn activate_check(State(db): State<S>, q: Query<ActivateCheckQuery>) -> Result<Json<ActivateCheckInfo>, StatusCode> {
    let person = db.find_person_by_token(&q.token).map_err(|_| StatusCode::NOT_FOUND)?;
    let building_name = match person.building_id {
        Some(bid) => db.get_building(bid).map(|b| b.name).unwrap_or_default(),
        None => "Общее".to_string(),
    };
    Ok(Json(ActivateCheckInfo {
        building_name,
        role: person.role,
        used: person.used_at.is_some(),
        person_name: Some(person.name),
        person_gender: Some(person.gender),
    }))
}

// ─── People (unified) ───

async fn list_people(State(db): State<S>, q: Query<BuildingQuery>) -> Result<Json<Vec<Person>>, StatusCode> {
    let ctx = extract_auth(&db, q.token.as_deref(), None).await?;
    Ok(Json(db.list_people(Some(ctx.building_id))?))
}

async fn list_all_people(State(db): State<S>, q: Query<TokenQuery>) -> Result<Json<Vec<Person>>, StatusCode> {
    let _ctx = extract_auth(&db, q.token.as_deref(), Some("superadmin")).await?;
    Ok(Json(db.list_all_people_for_tokens()?))
}

async fn create_person(State(db): State<S>, q: Query<TokenQuery>, Json(input): Json<CreatePerson>) -> Result<Json<Person>, StatusCode> {
    let ctx = extract_auth(&db, q.token.as_deref(), Some("admin")).await?;
    if ctx.role == "admin" && input.building_id != ctx.building_id {
        return Err(StatusCode::FORBIDDEN);
    }
    Ok(Json(db.create_person(&input)?))
}

async fn get_person(State(db): State<S>, Path(id): Path<i64>, q: Query<TokenQuery>) -> Result<Json<Person>, StatusCode> {
    extract_auth(&db, q.token.as_deref(), None).await?;
    Ok(Json(db.get_person(id)?))
}

async fn update_person(State(db): State<S>, Path(id): Path<i64>, q: Query<TokenQuery>, Json(input): Json<UpdatePerson>) -> Result<Json<Person>, StatusCode> {
    let _ctx = extract_auth(&db, q.token.as_deref(), Some("admin")).await?;
    Ok(Json(db.update_person(id, &input)?))
}

async fn delete_person(State(db): State<S>, Path(id): Path<i64>, q: Query<TokenQuery>) -> StatusCode {
    match extract_auth(&db, q.token.as_deref(), Some("admin")).await {
        Ok(_) => { db.delete_person(id).ok(); StatusCode::OK }
        Err(e) => e,
    }
}

async fn person_history(State(db): State<S>, Path(id): Path<i64>, q: Query<TokenQuery>) -> Result<Json<PersonHistory>, StatusCode> {
    extract_auth(&db, q.token.as_deref(), None).await?;
    Ok(Json(db.person_history(id)?))
}

#[derive(Debug, Deserialize)]
struct PeriodQuery { token: Option<String>, building_id: Option<i64> }

// ─── Periods ───

async fn list_periods(State(db): State<S>, q: Query<PeriodQuery>) -> Result<Json<Vec<Period>>, AppError> {
    let ctx = extract_auth(&db, q.token.as_deref(), None).await.map_err(AppError::from)?;
    let bid = q.building_id.unwrap_or(ctx.building_id);
    Ok(Json(db.list_periods(bid)?))
}

async fn get_period(State(db): State<S>, Path(id): Path<i64>, q: Query<TokenQuery>) -> Result<Json<Period>, AppError> {
    extract_auth(&db, q.token.as_deref(), None).await.map_err(AppError::from)?;
    Ok(Json(db.get_period(id)?))
}

async fn current_period(State(db): State<S>, q: Query<PeriodQuery>) -> Result<Json<Option<Period>>, AppError> {
    let ctx = extract_auth(&db, q.token.as_deref(), None).await.map_err(AppError::from)?;
    let bid = q.building_id.unwrap_or(ctx.building_id);
    Ok(Json(db.current_period(bid)?))
}

async fn create_period(State(db): State<S>, q: Query<TokenQuery>, Json(input): Json<CreatePeriod>) -> Result<Json<Period>, AppError> {
    let _ctx = extract_auth(&db, q.token.as_deref(), Some("admin")).await.map_err(AppError::from)?;
    Ok(Json(db.create_period(&input)?))
}

async fn set_current_period(State(db): State<S>, Path(id): Path<i64>, q: Query<TokenQuery>) -> Result<StatusCode, AppError> {
    extract_auth(&db, q.token.as_deref(), Some("admin")).await.map_err(AppError::from)?;
    db.set_current_period(id)?;
    Ok(StatusCode::OK)
}

async fn update_period(State(db): State<S>, Path(id): Path<i64>, q: Query<TokenQuery>, Json(input): Json<UpdatePeriod>) -> Result<Json<Period>, AppError> {
    let _ctx = extract_auth(&db, q.token.as_deref(), Some("admin")).await.map_err(AppError::from)?;
    Ok(Json(db.update_period(id, &input)?))
}

async fn next_period(State(db): State<S>, q: Query<PeriodQuery>) -> Result<Json<Period>, AppError> {
    let ctx = extract_auth(&db, q.token.as_deref(), None).await.map_err(AppError::from)?;
    let bid = q.building_id.unwrap_or(ctx.building_id);
    Ok(Json(db.next_period(bid)?))
}

async fn delete_period(State(db): State<S>, Path(id): Path<i64>, q: Query<TokenQuery>) -> Result<StatusCode, AppError> {
    extract_auth(&db, q.token.as_deref(), Some("admin")).await.map_err(AppError::from)?;
    db.delete_period(id)?;
    Ok(StatusCode::OK)
}

async fn copy_period(State(db): State<S>, Path(id): Path<i64>, q: Query<TokenQuery>, Json(input): Json<CreatePeriod>) -> Result<Json<Period>, AppError> {
    let _ctx = extract_auth(&db, q.token.as_deref(), Some("admin")).await.map_err(AppError::from)?;
    Ok(Json(db.copy_period(id, &input)?))
}

// ─── Period View ───

async fn period_view(State(db): State<S>, Path(id): Path<i64>, q: Query<ViewQuery>) -> Result<Json<PeriodView>, StatusCode> {
    extract_auth(&db, q.token.as_deref(), None).await?;
    Ok(Json(db.period_view(id, q.building_id)?))
}

// ─── Assignments ───

async fn set_assignment(State(db): State<S>, Path(id): Path<i64>, q: Query<TokenQuery>, Json(input): Json<SetAssignment>) -> Result<Json<Assignment>, AppError> {
    let _ctx = extract_auth(&db, q.token.as_deref(), Some("admin")).await.map_err(AppError::from)?;
    Ok(Json(db.set_assignment(id, &input)?))
}

async fn remove_assignment(State(db): State<S>, Path(id): Path<i64>, q: Query<TokenQuery>, Json(input): Json<SetAssignment>) -> Result<StatusCode, AppError> {
    extract_auth(&db, q.token.as_deref(), Some("admin")).await.map_err(AppError::from)?;
    db.remove_assignment(id, &input)?;
    Ok(StatusCode::OK)
}

async fn bulk_assignments(State(db): State<S>, Path(id): Path<i64>, q: Query<TokenQuery>, Json(input): Json<BulkAssignments>) -> Result<Json<usize>, AppError> {
    let _ctx = extract_auth(&db, q.token.as_deref(), Some("admin")).await.map_err(AppError::from)?;
    Ok(Json(db.bulk_assignments(id, &input)?))
}

async fn fill_period(State(db): State<S>, Path(id): Path<i64>, q: Query<TokenQuery>, Json(input): Json<FillPeriodRequest>) -> Result<Json<usize>, AppError> {
    let _ctx = extract_auth(&db, q.token.as_deref(), Some("admin")).await.map_err(AppError::from)?;
    Ok(Json(db.fill_period(id, &input)?))
}

async fn clear_room(State(db): State<S>, Path(id): Path<i64>, q: Query<TokenQuery>, Json(input): Json<ClearRoomRequest>) -> Result<Json<usize>, AppError> {
    let _ctx = extract_auth(&db, q.token.as_deref(), Some("admin")).await.map_err(AppError::from)?;
    Ok(Json(db.clear_room(id, input.room_id, input.day)?))
}

async fn auto_fill(State(db): State<S>, Path(id): Path<i64>, q: Query<ViewQuery>) -> Result<Json<usize>, AppError> {
    let _ctx = extract_auth(&db, q.token.as_deref(), Some("admin")).await.map_err(AppError::from)?;
    Ok(Json(db.auto_fill(id, q.building_id)?))
}

// ─── Absences ───

async fn list_absences(State(db): State<S>, q: Query<AbsenceQuery>) -> Result<Json<Vec<Absence>>, StatusCode> {
    let ctx = extract_auth(&db, q.token.as_deref(), None).await?;
    let bid = q.building_id.unwrap_or(ctx.building_id);
    if bid != ctx.building_id { return Err(StatusCode::FORBIDDEN); }
    if let Some(pid) = q.period_id {
        let period = db.get_period(pid)?;
        Ok(Json(db.list_absences_for_period(&period)?))
    } else {
        Ok(Json(db.list_absences_for_building(bid)?))
    }
}

async fn create_absence(State(db): State<S>, q: Query<TokenQuery>, Json(input): Json<CreateAbsence>) -> Result<Json<Absence>, StatusCode> {
    extract_auth(&db, q.token.as_deref(), None).await?;
    Ok(Json(db.create_absence(&input)?))
}

async fn delete_absence(State(db): State<S>, Path(id): Path<i64>, q: Query<TokenQuery>) -> StatusCode {
    match extract_auth(&db, q.token.as_deref(), None).await {
        Ok(_) => { db.delete_absence(id).ok(); StatusCode::OK }
        Err(e) => e,
    }
}

// ─── Ranking ───

async fn ranking(State(db): State<S>, q: Query<TokenQuery>) -> Result<Json<Vec<RankingEntry>>, StatusCode> {
    extract_auth(&db, q.token.as_deref(), None).await?;
    Ok(Json(db.ranking(None)?))
}

// ─── Buildings ───

async fn list_buildings(State(db): State<S>, q: Query<TokenQuery>) -> Result<Json<Vec<Building>>, StatusCode> {
    extract_auth(&db, q.token.as_deref(), None).await?;
    Ok(Json(db.list_buildings()?))
}

async fn get_building(State(db): State<S>, Path(id): Path<i64>, q: Query<TokenQuery>) -> Result<Json<Building>, StatusCode> {
    let ctx = extract_auth(&db, q.token.as_deref(), None).await?;
    if ctx.building_id != id { return Err(StatusCode::FORBIDDEN); }
    Ok(Json(db.get_building(id)?))
}

async fn create_building(State(db): State<S>, q: Query<TokenQuery>, Json(input): Json<CreateBuilding>) -> Result<Json<Building>, StatusCode> {
    extract_auth(&db, q.token.as_deref(), Some("superadmin")).await?;
    Ok(Json(db.create_building(&input.name)?))
}

async fn delete_building(State(db): State<S>, Path(id): Path<i64>, q: Query<TokenQuery>) -> StatusCode {
    match extract_auth(&db, q.token.as_deref(), Some("superadmin")).await {
        Ok(_) => { db.delete_building(id).ok(); StatusCode::OK }
        Err(e) => e,
    }
}

// ─── Rooms ───

async fn list_rooms(State(db): State<S>, Path(bid): Path<i64>, q: Query<TokenQuery>) -> Result<Json<Vec<Room>>, StatusCode> {
    let ctx = extract_auth(&db, q.token.as_deref(), None).await?;
    if ctx.building_id != bid { return Err(StatusCode::FORBIDDEN); }
    Ok(Json(db.list_rooms(bid)?))
}

async fn create_room(State(db): State<S>, Path(bid): Path<i64>, q: Query<TokenQuery>, Json(input): Json<CreateRoom>) -> Result<Json<Room>, StatusCode> {
    let ctx = extract_auth(&db, q.token.as_deref(), Some("admin")).await?;
    if ctx.building_id != bid { return Err(StatusCode::FORBIDDEN); }
    Ok(Json(db.create_room(bid, &input)?))
}

async fn update_room(State(db): State<S>, Path(id): Path<i64>, q: Query<TokenQuery>, Json(input): Json<UpdateRoom>) -> Result<Json<Room>, StatusCode> {
    let _ctx = extract_auth(&db, q.token.as_deref(), Some("admin")).await?;
    Ok(Json(db.update_room(id, &input)?))
}

async fn delete_room(State(db): State<S>, Path(id): Path<i64>, q: Query<TokenQuery>) -> StatusCode {
    match extract_auth(&db, q.token.as_deref(), Some("admin")).await {
        Ok(_) => { db.delete_room(id).ok(); StatusCode::OK }
        Err(e) => e,
    }
}

// ─── Public share (no auth) ───

#[derive(Debug, Serialize)]
struct ShareInfo { token: String }

async fn building_share(State(db): State<S>, Path(id): Path<i64>, q: Query<TokenQuery>) -> Result<Json<ShareInfo>, StatusCode> {
    let ctx = extract_auth(&db, q.token.as_deref(), Some("admin")).await?;
    if ctx.role == "admin" && ctx.building_id != id {
        return Err(StatusCode::FORBIDDEN);
    }
    let token = db.building_public_token(id).map_err(|_| StatusCode::NOT_FOUND)?;
    Ok(Json(ShareInfo { token }))
}

#[derive(Debug, Serialize)]
struct PublicBuildingInfo { building: Building, periods: Vec<Period> }

async fn public_info(State(db): State<S>, Path(token): Path<String>) -> Result<Json<PublicBuildingInfo>, StatusCode> {
    let building = db.building_by_public_token(&token).map_err(|_| StatusCode::NOT_FOUND)?;
    let periods = db.list_periods(building.id)?;
    Ok(Json(PublicBuildingInfo { building, periods }))
}

async fn public_period_view(State(db): State<S>, Path((token, id)): Path<(String, i64)>) -> Result<Json<PeriodView>, StatusCode> {
    let building = db.building_by_public_token(&token).map_err(|_| StatusCode::NOT_FOUND)?;
    let period = db.get_period(id).map_err(|_| StatusCode::NOT_FOUND)?;
    if period.building_id != building.id { return Err(StatusCode::FORBIDDEN); }
    Ok(Json(db.period_view(id, building.id)?))
}

async fn public_person_history(State(db): State<S>, Path((token, pid)): Path<(String, i64)>) -> Result<Json<PersonHistory>, StatusCode> {
    let building = db.building_by_public_token(&token).map_err(|_| StatusCode::NOT_FOUND)?;
    let person = db.get_person(pid).map_err(|_| StatusCode::NOT_FOUND)?;
    if person.building_id != Some(building.id) { return Err(StatusCode::FORBIDDEN); }
    Ok(Json(db.person_history(pid)?))
}
