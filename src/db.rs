use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager, Pool};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use crate::models::*;
use crate::schema::{absences, assignments, buildings, people, periods, rooms};

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

pub type DbPool = Pool<ConnectionManager<SqliteConnection>>;
pub struct Db { pub pool: DbPool }

#[derive(Debug)]
pub enum DbError {
    Diesel(diesel::result::Error),
    Pool(String),
    Migration(String),
    Forbidden(String),
}
impl From<diesel::result::Error> for DbError { fn from(e: diesel::result::Error) -> Self { DbError::Diesel(e) } }

impl Db {
    pub fn new(path: &str) -> Result<Self, DbError> {
        let manager = ConnectionManager::<SqliteConnection>::new(path);
        let pool = r2d2::Pool::builder().max_size(5).build(manager).map_err(|e| DbError::Pool(e.to_string()))?;
        {
            let mut conn = pool.get().map_err(|e| DbError::Pool(e.to_string()))?;
            diesel::sql_query("PRAGMA journal_mode=WAL").execute(&mut conn)?;
            diesel::sql_query("PRAGMA foreign_keys=ON").execute(&mut conn)?;
            conn.run_pending_migrations(MIGRATIONS).map_err(|e| DbError::Migration(e.to_string()))?;
        }
        let db = Db { pool };
        db.seed()?;
        Ok(db)
    }

    fn conn(&self) -> Result<r2d2::PooledConnection<ConnectionManager<SqliteConnection>>, DbError> {
        self.pool.get().map_err(|e| DbError::Pool(e.to_string()))
    }

    fn seed(&self) -> Result<(), DbError> {
        let mut conn = self.conn()?;
        let count: i64 = buildings::table.count().get_result(&mut conn)?;
        if count > 0 { return Ok(()); }
        diesel::insert_into(buildings::table).values(NewBuilding { name: "Лоджия 4" }).execute(&mut conn)?;
        let bid: i32 = buildings::table.select(buildings::id).order(buildings::id.desc()).first(&mut conn)?;
        for (key, name, required, gender) in [
            ("restaurant", "Ресторан", 2, "M"), ("kitchen", "Кухня", 1, "F"),
            ("toilet", "Туалет", 1, "M"), ("shower", "Душ", 1, "M"),
            ("washbasin", "Умывальник", 1, "M"), ("trash", "Мусор", 1, "M"),
            ("street", "Улица", 1, "M"),
        ] {
            diesel::insert_into(rooms::table).values(NewRoom { building_id: bid, key, name, required, gender }).execute(&mut conn)?;
        }
        Ok(())
    }

    fn today_str(&self) -> String {
        chrono::Local::now().format("%Y-%m-%d").to_string()
    }

    fn period_status(&self, period: &Period) -> &'static str {
        let today = self.today_str();
        if period.start_date > today { "not_started" }
        else if period.end_date_str() > today { "active" }
        else { "finished" }
    }

    // ─── Buildings ───

    pub fn list_buildings(&self) -> Result<Vec<Building>, DbError> {
        let mut c = self.conn()?;
        Ok(buildings::table.filter(buildings::active.eq(1)).order(buildings::name).select(BuildingRow::as_select()).load(&mut c)?.into_iter().map(|r| r.into_api()).collect())
    }
    pub fn get_building(&self, id: i64) -> Result<Building, DbError> {
        let mut c = self.conn()?;
        Ok(buildings::table.filter(buildings::id.eq(id as i32)).select(BuildingRow::as_select()).first(&mut c)?.into_api())
    }
    pub fn create_building(&self, name: &str) -> Result<Building, DbError> {
        let mut c = self.conn()?;
        diesel::insert_into(buildings::table).values(NewBuilding { name }).execute(&mut c)?;
        Ok(buildings::table.order(buildings::id.desc()).select(BuildingRow::as_select()).first(&mut c)?.into_api())
    }
    pub fn building_public_token(&self, building_id: i64) -> Result<String, DbError> {
        let mut c = self.conn()?;
        let existing: String = buildings::table.filter(buildings::id.eq(building_id as i32)).select(buildings::public_token).first(&mut c)?;
        if existing.is_empty() {
            let token = Self::gen_token();
            diesel::update(buildings::table.filter(buildings::id.eq(building_id as i32))).set(buildings::public_token.eq(&token)).execute(&mut c)?;
            Ok(token)
        } else {
            Ok(existing)
        }
    }
    pub fn building_by_public_token(&self, token: &str) -> Result<Building, DbError> {
        let mut c = self.conn()?;
        Ok(buildings::table.filter(buildings::public_token.eq(token).and(buildings::active.eq(1))).select(BuildingRow::as_select()).first(&mut c)?.into_api())
    }
    pub fn delete_building(&self, id: i64) -> Result<(), DbError> {
        let mut c = self.conn()?;
        diesel::update(buildings::table.filter(buildings::id.eq(id as i32))).set(buildings::active.eq(0)).execute(&mut c)?;
        diesel::update(rooms::table.filter(rooms::building_id.eq(id as i32))).set(rooms::active.eq(0)).execute(&mut c)?;
        Ok(())
    }

    // ─── Rooms ───

    pub fn list_rooms(&self, building_id: i64) -> Result<Vec<Room>, DbError> {
        let mut c = self.conn()?;
        Ok(rooms::table.filter(rooms::building_id.eq(building_id as i32).and(rooms::active.eq(1))).order(rooms::name).select(RoomRow::as_select()).load(&mut c)?.into_iter().map(|r| r.into_api()).collect())
    }
    pub fn create_room(&self, building_id: i64, input: &CreateRoom) -> Result<Room, DbError> {
        let mut c = self.conn()?;
        diesel::insert_into(rooms::table).values(NewRoom { building_id: building_id as i32, key: &input.key, name: &input.name, required: input.required as i32, gender: &input.gender }).execute(&mut c)?;
        Ok(rooms::table.order(rooms::id.desc()).select(RoomRow::as_select()).first(&mut c)?.into_api())
    }
    pub fn update_room(&self, id: i64, input: &UpdateRoom) -> Result<Room, DbError> {
        let mut c = self.conn()?;
        if let Some(ref key) = input.key {
            diesel::update(rooms::table.filter(rooms::id.eq(id as i32))).set(rooms::key.eq(key)).execute(&mut c)?;
        }
        if let Some(ref name) = input.name {
            diesel::update(rooms::table.filter(rooms::id.eq(id as i32))).set(rooms::name.eq(name)).execute(&mut c)?;
        }
        if let Some(required) = input.required {
            diesel::update(rooms::table.filter(rooms::id.eq(id as i32))).set(rooms::required.eq(required as i32)).execute(&mut c)?;
        }
        if let Some(ref gender) = input.gender {
            diesel::update(rooms::table.filter(rooms::id.eq(id as i32))).set(rooms::gender.eq(gender)).execute(&mut c)?;
        }
        Ok(rooms::table.filter(rooms::id.eq(id as i32)).select(RoomRow::as_select()).first(&mut c)?.into_api())
    }
    pub fn delete_room(&self, id: i64) -> Result<(), DbError> {
        let mut c = self.conn()?;
        diesel::update(rooms::table.filter(rooms::id.eq(id as i32))).set(rooms::active.eq(0)).execute(&mut c)?;
        Ok(())
    }

    // ─── People (unified: person = token holder) ───

    pub fn list_people(&self, building_id: Option<i64>) -> Result<Vec<Person>, DbError> {
        let mut c = self.conn()?;
        let rows = match building_id {
            Some(bid) => people::table.filter(people::building_id.eq(bid as i32).and(people::active.eq(1))).order(people::name).select(PersonRow::as_select()).load::<PersonRow>(&mut c)?,
            None => people::table.filter(people::active.eq(1)).order(people::name).select(PersonRow::as_select()).load::<PersonRow>(&mut c)?,
        };
        Ok(rows.into_iter().map(|r| r.into_api()).collect())
    }

    pub fn list_duty_people(&self, building_id: i64) -> Result<Vec<Person>, DbError> {
        let mut c = self.conn()?;
        let rows = people::table
            .filter(people::building_id.eq(building_id as i32).and(people::active.eq(1)).and(people::role.ne("superadmin")))
            .order(people::name).select(PersonRow::as_select()).load::<PersonRow>(&mut c)?;
        Ok(rows.into_iter().map(|r| r.into_api()).collect())
    }

    fn gen_token() -> String {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        std::iter::repeat_with(|| {
            let idx = rng.gen_range(0..36);
            if idx < 10 { (b'0' + idx) as char } else { (b'a' + idx - 10) as char }
        }).take(16).collect()
    }

    /// Минимальный рейтинг (отработанные дни) среди активных жителей лоджи,
    /// исключая `exclude_id`. Если жителей нет — 0.
    fn min_resident_duty_days(&self, c: &mut SqliteConnection, building_id: i32, exclude_id: i32) -> Result<i32, DbError> {
        let residents: Vec<PersonRow> = people::table
            .filter(people::building_id.eq(building_id)
                .and(people::active.eq(1))
                .and(people::role.eq("resident"))
                .and(people::id.ne(exclude_id)))
            .select(PersonRow::as_select())
            .load(c)?;
        let mut min: Option<i64> = None;
        for r in &residents {
            let n: i64 = assignments::table.filter(assignments::person_id.eq(r.id)).count().get_result(c)?;
            min = Some(match min { Some(m) => m.min(n), None => n });
        }
        Ok(min.unwrap_or(0) as i32)
    }

    pub fn create_person(&self, input: &CreatePerson) -> Result<Person, DbError> {
        let mut c = self.conn()?;
        let token_str = Self::gen_token();
        let name = input.name.as_deref().unwrap_or("Житель");
        let gender = input.gender.as_deref().unwrap_or("M");
        let role = input.role.as_deref().unwrap_or("resident");
        diesel::insert_into(people::table).values(NewPerson {
            name, gender, building_id: Some(input.building_id as i32),
            token: &token_str, role,
        }).execute(&mut c)?;
        let new_id: i32 = people::table.order(people::id.desc()).select(people::id).first(&mut c)?;
        if role != "superadmin" {
            let offset = self.min_resident_duty_days(&mut c, input.building_id as i32, new_id)?;
            diesel::update(people::table.filter(people::id.eq(new_id)))
                .set(people::duty_offset.eq(offset))
                .execute(&mut c)?;
        }
        Ok(people::table.filter(people::id.eq(new_id)).select(PersonRow::as_select()).first(&mut c)?.into_api())
    }

    pub fn get_person(&self, id: i64) -> Result<Person, DbError> {
        let mut c = self.conn()?;
        Ok(people::table.filter(people::id.eq(id as i32)).select(PersonRow::as_select()).first(&mut c)?.into_api())
    }

    pub fn update_person(&self, id: i64, input: &UpdatePerson) -> Result<Person, DbError> {
        let mut c = self.conn()?;
        if let Some(ref name) = input.name {
            diesel::update(people::table.filter(people::id.eq(id as i32))).set(people::name.eq(name)).execute(&mut c)?;
        }
        if let Some(ref gender) = input.gender {
            diesel::update(people::table.filter(people::id.eq(id as i32))).set(people::gender.eq(gender)).execute(&mut c)?;
        }
        if let Some(ref role) = input.role {
            diesel::update(people::table.filter(people::id.eq(id as i32))).set(people::role.eq(role)).execute(&mut c)?;
            if role == "admin" {
                let person = people::table.filter(people::id.eq(id as i32)).select(PersonRow::as_select()).first(&mut c)?;
                if let Some(bid) = person.building_id {
                    diesel::update(people::table
                        .filter(people::building_id.eq(bid).and(people::role.eq("admin")).and(people::active.eq(1)).and(people::id.ne(id as i32))))
                        .set(people::role.eq("resident")).execute(&mut c)?;
                }
            }
        }
        Ok(people::table.filter(people::id.eq(id as i32)).select(PersonRow::as_select()).first(&mut c)?.into_api())
    }

    pub fn delete_person(&self, id: i64) -> Result<(), DbError> {
        let mut c = self.conn()?;
        diesel::update(people::table.filter(people::id.eq(id as i32))).set(people::active.eq(0)).execute(&mut c)?;
        Ok(())
    }

    // ─── Auth (token-based, using people table) ───

    pub fn validate_token(&self, token: &str) -> Result<AuthContext, DbError> {
        let mut c = self.conn()?;
        let row = people::table
            .filter(people::token.eq(token).and(people::active.eq(1)))
            .select(PersonRow::as_select())
            .first(&mut c)?;
        Ok(AuthContext {
            building_id: row.building_id.unwrap_or(0) as i64,
            role: row.role.clone(),
            name: if row.name.starts_with("Person-") || row.name == "Житель" { None } else { Some(row.name.clone()) },
        })
    }

    pub fn find_person_by_token(&self, token: &str) -> Result<Person, DbError> {
        let mut c = self.conn()?;
        Ok(people::table
            .filter(people::token.eq(token).and(people::active.eq(1)))
            .select(PersonRow::as_select())
            .first(&mut c)?.into_api())
    }

    pub fn activate_token(&self, token: &str, name: Option<&str>, gender: Option<&str>) -> Result<AuthContext, DbError> {
        let mut c = self.conn()?;
        let row = people::table
            .filter(people::token.eq(token).and(people::active.eq(1)))
            .select(PersonRow::as_select())
            .first(&mut c)?;

        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let role = row.role.clone();
        let bid = row.building_id.unwrap_or(0);

        let is_placeholder = row.name.starts_with("Person-") || row.name == "Житель" || row.name == "Админ";
        let needs_name = is_placeholder || row.used_at.is_none();

        if needs_name {
            let n = name.filter(|s| !s.is_empty()).unwrap_or(&row.name);
            let g = gender.unwrap_or(&row.gender);
            diesel::update(people::table.filter(people::id.eq(row.id)))
                .set((people::name.eq(n), people::gender.eq(g), people::used_at.eq(Some(now.clone()))))
                .execute(&mut c)?;
        } else {
            diesel::update(people::table.filter(people::id.eq(row.id)))
                .set(people::used_at.eq(Some(now.clone())))
                .execute(&mut c)?;
        }

        if role == "admin" {
            diesel::update(people::table
                .filter(people::building_id.eq(bid).and(people::role.eq("admin")).and(people::active.eq(1)).and(people::id.ne(row.id))))
                .set(people::role.eq("resident")).execute(&mut c)?;
        }

        let display_name = if needs_name {
            name.filter(|s| !s.is_empty()).map(|s| s.to_string())
        } else {
            Some(row.name.clone())
        };

        Ok(AuthContext {
            building_id: bid as i64,
            role,
            name: display_name,
        })
    }

    pub fn list_people_for_tokens(&self, building_id: i64, current_token: &str) -> Result<Vec<Person>, DbError> {
        let mut c = self.conn()?;
        Ok(people::table.filter(people::building_id.eq(building_id as i32).and(people::active.eq(1))
            .and(people::role.ne("superadmin"))
            .and(people::role.ne("admin").or(people::token.eq(current_token))))
            .order(people::created_at.desc())
            .select(PersonRow::as_select()).load(&mut c)?.into_iter().map(|r| r.into_api()).collect())
    }

    pub fn list_all_people_for_tokens(&self) -> Result<Vec<Person>, DbError> {
        let mut c = self.conn()?;
        Ok(people::table.filter(people::active.eq(1))
            .order(people::created_at.desc())
            .select(PersonRow::as_select()).load(&mut c)?.into_iter().map(|r| r.into_api()).collect())
    }

    pub fn delete_person_token(&self, id: i64) -> Result<(), DbError> {
        let mut c = self.conn()?;
        diesel::update(people::table.filter(people::id.eq(id as i32))).set(people::active.eq(0)).execute(&mut c)?;
        Ok(())
    }

    // ─── Periods ───

    pub fn list_periods(&self, building_id: i64) -> Result<Vec<Period>, DbError> {
        let mut c = self.conn()?;
        Ok(periods::table.filter(periods::building_id.eq(building_id as i32)).order(periods::start_date.desc()).select(PeriodRow::as_select()).load(&mut c)?.into_iter().map(|r| r.into_api()).collect())
    }
    pub fn get_period(&self, id: i64) -> Result<Period, DbError> {
        let mut c = self.conn()?;
        Ok(periods::table.filter(periods::id.eq(id as i32)).select(PeriodRow::as_select()).first(&mut c)?.into_api())
    }
    pub fn current_period(&self, building_id: i64) -> Result<Option<Period>, DbError> {
        let mut c = self.conn()?;
        Ok(periods::table.filter(periods::is_current.eq(1).and(periods::building_id.eq(building_id as i32))).select(PeriodRow::as_select()).first(&mut c).optional()?.map(|r| r.into_api()))
    }
    fn check_period_overlap(&self, building_id: i64, start_date: &str, num_days: i64, exclude_id: Option<i64>) -> Result<(), DbError> {
        let mut c = self.conn()?;
        let bid = building_id as i32;
        let existing: Vec<PeriodRow> = periods::table.filter(periods::building_id.eq(bid)).select(PeriodRow::as_select()).load(&mut c)?;
        let new_start = chrono::NaiveDate::parse_from_str(start_date, "%Y-%m-%d").map_err(|_| DbError::Diesel(diesel::result::Error::NotFound))?;
        let new_end = new_start + chrono::Duration::days(num_days);
        for p in &existing {
            if Some(p.id as i64) == exclude_id { continue; }
            if let Ok(p_start) = chrono::NaiveDate::parse_from_str(&p.start_date, "%Y-%m-%d") {
                let p_end = p_start + chrono::Duration::days(p.num_days as i64);
                if new_start < p_end && new_end > p_start {
                    return Err(DbError::Forbidden(format!("Период пересекается с '{}'", p.name)));
                }
            }
        }
        Ok(())
    }
    pub fn create_period(&self, input: &CreatePeriod) -> Result<Period, DbError> {
        self.check_period_overlap(input.building_id, &input.start_date, input.num_days, None)?;
        let mut c = self.conn()?;
        diesel::insert_into(periods::table).values(NewPeriod { name: &input.name, start_date: &input.start_date, num_days: input.num_days as i32, building_id: input.building_id as i32 }).execute(&mut c)?;
        Ok(periods::table.order(periods::id.desc()).select(PeriodRow::as_select()).first(&mut c)?.into_api())
    }
    pub fn set_current_period(&self, id: i64) -> Result<(), DbError> {
        let mut c = self.conn()?;
        let period = self.get_period(id)?;
        diesel::update(periods::table.filter(periods::is_current.eq(1).and(periods::building_id.eq(period.building_id as i32)))).set(periods::is_current.eq(0)).execute(&mut c)?;
        diesel::update(periods::table.filter(periods::id.eq(id as i32))).set(periods::is_current.eq(1)).execute(&mut c)?;
        Ok(())
    }
    pub fn update_period(&self, id: i64, input: &UpdatePeriod) -> Result<Period, DbError> {
        let period = self.get_period(id)?;
        if self.period_status(&period) != "not_started" {
            return Err(DbError::Forbidden("Нельзя редактировать период который уже начался".into()));
        }
        let new_start = input.start_date.as_deref().unwrap_or(&period.start_date);
        let new_days = input.num_days.unwrap_or(period.num_days);
        self.check_period_overlap(period.building_id, new_start, new_days, Some(id))?;
        let mut c = self.conn()?;
        if let Some(ref name) = input.name {
            diesel::update(periods::table.filter(periods::id.eq(id as i32))).set(periods::name.eq(name)).execute(&mut c)?;
        }
        if let Some(ref start_date) = input.start_date {
            diesel::update(periods::table.filter(periods::id.eq(id as i32))).set(periods::start_date.eq(start_date)).execute(&mut c)?;
        }
        if let Some(num_days) = input.num_days {
            diesel::update(periods::table.filter(periods::id.eq(id as i32))).set(periods::num_days.eq(num_days as i32)).execute(&mut c)?;
        }
        Ok(self.get_period(id)?)
    }
    pub fn delete_period(&self, id: i64) -> Result<(), DbError> {
        let period = self.get_period(id)?;
        if self.period_status(&period) != "not_started" {
            return Err(DbError::Forbidden("Нельзя удалить период который уже начался".into()));
        }
        let mut c = self.conn()?;
        diesel::delete(assignments::table.filter(assignments::period_id.eq(id as i32))).execute(&mut c)?;
        diesel::delete(periods::table.filter(periods::id.eq(id as i32))).execute(&mut c)?;
        Ok(())
    }
    pub fn next_period(&self, building_id: i64) -> Result<Period, DbError> {
        let mut c = self.conn()?;
        let cur: Option<PeriodRow> = periods::table.filter(periods::is_current.eq(1).and(periods::building_id.eq(building_id as i32))).select(PeriodRow::as_select()).first(&mut c).optional()?;
        let (start_date, num_days, name) = if let Some(ref p) = cur {
            let end = chrono::NaiveDate::parse_from_str(&p.start_date, "%Y-%m-%d").ok().ok_or(DbError::Diesel(diesel::result::Error::NotFound))?;
            (end.format("%Y-%m-%d").to_string(), p.num_days, format!("{} + 1", p.name))
        } else {
            (chrono::Local::now().format("%Y-%m-%d").to_string(), 7, "Период 1".to_string())
        };
        diesel::update(periods::table.filter(periods::is_current.eq(1).and(periods::building_id.eq(building_id as i32)))).set(periods::is_current.eq(0)).execute(&mut c)?;
        self.check_period_overlap(building_id, &start_date, num_days as i64, None)?;
        diesel::insert_into(periods::table).values(NewPeriod { name: &name, start_date: &start_date, num_days, building_id: building_id as i32 }).execute(&mut c)?;
        Ok(periods::table.order(periods::id.desc()).select(PeriodRow::as_select()).first(&mut c)?.into_api())
    }
    pub fn copy_period(&self, _source_id: i64, input: &CreatePeriod) -> Result<Period, DbError> {
        self.check_period_overlap(input.building_id, &input.start_date, input.num_days, None)?;
        let mut c = self.conn()?;
        diesel::insert_into(periods::table).values(NewPeriod { name: &input.name, start_date: &input.start_date, num_days: input.num_days as i32, building_id: input.building_id as i32 }).execute(&mut c)?;
        let new_id = periods::table.select(periods::id).order(periods::id.desc()).first::<i32>(&mut c)?;
        Ok(periods::table.filter(periods::id.eq(new_id)).select(PeriodRow::as_select()).first(&mut c)?.into_api())
    }

    // ─── Assignments ───

    pub fn set_assignment(&self, period_id: i64, input: &SetAssignment) -> Result<Assignment, DbError> {
        let mut c = self.conn()?;
        let (pid, rid, perid) = (input.person_id as i32, input.room_id as i32, period_id as i32);
        let room = rooms::table.filter(rooms::id.eq(rid).and(rooms::active.eq(1))).select(RoomRow::as_select()).first(&mut c)?;
        let person = people::table.filter(people::id.eq(pid).and(people::active.eq(1))).select(PersonRow::as_select()).first(&mut c)?;
        let period = self.get_period(period_id)?;
        if self.period_status(&period) == "finished" { return Err(DbError::Forbidden("Нельзя изменять завершённый период".into())); }
        let absent: bool = self.person_absent_on_period(pid, &period)?;
        if absent { return Err(DbError::Diesel(diesel::result::Error::NotFound)); }
        let count: i64 = assignments::table.filter(assignments::period_id.eq(perid).and(assignments::room_id.eq(rid)).and(assignments::day.eq(input.day))).count().get_result(&mut c)?;
        if count >= room.required as i64 { return Err(DbError::Diesel(diesel::result::Error::NotFound)); }
        diesel::insert_into(assignments::table).values(NewAssignment { period_id: perid, person_id: pid, room_id: rid, day: input.day }).execute(&mut c)?;
        let a = assignments::table.filter(assignments::period_id.eq(perid).and(assignments::person_id.eq(pid)).and(assignments::room_id.eq(rid)).and(assignments::day.eq(input.day)))
            .select((assignments::id, assignments::period_id, assignments::person_id, assignments::room_id, assignments::day)).first::<(i32, i32, i32, i32, i32)>(&mut c)?;
        Ok(Assignment { id: a.0 as i64, period_id: a.1 as i64, person_id: pid as i64, person_name: person.name.clone(), person_gender: person.gender.clone(), room_id: rid as i64, room_name: room.name, room_key: room.key, building_id: room.building_id as i64, day: input.day })
    }
    pub fn remove_assignment(&self, period_id: i64, input: &SetAssignment) -> Result<(), DbError> {
        let period = self.get_period(period_id)?;
        if self.period_status(&period) == "finished" { return Err(DbError::Forbidden("Нельзя изменять завершённый период".into())); }
        let mut c = self.conn()?;
        diesel::delete(assignments::table.filter(assignments::period_id.eq(period_id as i32).and(assignments::person_id.eq(input.person_id as i32)).and(assignments::room_id.eq(input.room_id as i32)).and(assignments::day.eq(input.day)))).execute(&mut c)?;
        Ok(())
    }
    pub fn bulk_assignments(&self, period_id: i64, input: &BulkAssignments) -> Result<usize, DbError> {
        let mut c = self.conn()?;
        let mut count = 0usize;
        for a in &input.assignments {
            diesel::insert_into(assignments::table).values(NewAssignment { period_id: period_id as i32, person_id: a.person_id as i32, room_id: a.room_id as i32, day: a.day }).execute(&mut c).ok();
            count += 1;
        }
        Ok(count)
    }

    pub fn fill_period(&self, period_id: i64, input: &FillPeriodRequest) -> Result<usize, DbError> {
        let period = self.get_period(period_id)?;
        let from = input.from_day.unwrap_or(0);
        let mut c = self.conn()?;
        let room = rooms::table.filter(rooms::id.eq(input.room_id as i32).and(rooms::active.eq(1))).select(RoomRow::as_select()).first(&mut c)?;
        if room.required > 1 {
            diesel::delete(assignments::table.filter(
                assignments::period_id.eq(period_id as i32)
                    .and(assignments::room_id.eq(input.room_id as i32))
                    .and(assignments::person_id.eq(input.person_id as i32))
                    .and(assignments::day.ge(from))
            )).execute(&mut c)?;
        } else {
            diesel::delete(assignments::table.filter(
                assignments::period_id.eq(period_id as i32)
                    .and(assignments::room_id.eq(input.room_id as i32))
                    .and(assignments::day.ge(from))
            )).execute(&mut c)?;
        }
        let mut count = 0usize;
        for day in from..period.num_days as i32 {
            diesel::insert_into(assignments::table).values(NewAssignment {
                period_id: period_id as i32, person_id: input.person_id as i32,
                room_id: input.room_id as i32, day,
            }).execute(&mut c)?;
            count += 1;
        }
        Ok(count)
    }

    /// Полностью очистить помещение в периоде: `day` = один день, `None` = вся неделя/период.
    pub fn clear_room(&self, period_id: i64, room_id: i64, day: Option<i32>) -> Result<usize, DbError> {
        let period = self.get_period(period_id)?;
        if self.period_status(&period) == "finished" {
            return Err(DbError::Forbidden("Нельзя изменять завершённый период".into()));
        }
        let mut c = self.conn()?;
        let (perid, rid) = (period_id as i32, room_id as i32);
        let n = match day {
            Some(d) => diesel::delete(assignments::table.filter(
                assignments::period_id.eq(perid)
                    .and(assignments::room_id.eq(rid))
                    .and(assignments::day.eq(d))
            )).execute(&mut c)?,
            None => diesel::delete(assignments::table.filter(
                assignments::period_id.eq(perid)
                    .and(assignments::room_id.eq(rid))
            )).execute(&mut c)?,
        };
        Ok(n)
    }

    // ─── Absences ───

    fn person_absent_on_period(&self, person_id: i32, period: &Period) -> Result<bool, DbError> {
        let mut c = self.conn()?;
        let period_end = period.end_date_str();
        let count: i64 = absences::table
            .filter(absences::person_id.eq(person_id)
                .and(absences::start_date.lt(&period_end))
                .and(absences::end_date.gt(&period.start_date)))
            .count().get_result(&mut c)?;
        Ok(count > 0)
    }

    pub fn list_absences_for_building(&self, building_id: i64) -> Result<Vec<Absence>, DbError> {
        let mut c = self.conn()?;
        let rows = absences::table
            .inner_join(people::table.on(absences::person_id.eq(people::id)))
            .filter(people::building_id.eq(building_id as i32))
            .order(absences::start_date.desc())
            .select((absences::id, absences::person_id, people::name, absences::start_date, absences::end_date, absences::reason))
            .load::<(i32, i32, String, String, String, String)>(&mut c)?;
        Ok(rows.into_iter().map(|(id, pid, name, sd, ed, reason)| Absence { id: id as i64, person_id: pid as i64, person_name: name, start_date: sd, end_date: ed, reason }).collect())
    }

    pub fn list_absences_for_period(&self, period: &Period) -> Result<Vec<Absence>, DbError> {
        let mut c = self.conn()?;
        let period_end = period.end_date_str();
        let rows = absences::table
            .inner_join(people::table.on(absences::person_id.eq(people::id)))
            .filter(absences::start_date.lt(&period_end).and(absences::end_date.gt(&period.start_date)))
            .select((absences::id, absences::person_id, people::name, absences::start_date, absences::end_date, absences::reason))
            .load::<(i32, i32, String, String, String, String)>(&mut c)?;
        Ok(rows.into_iter().map(|(id, pid, name, sd, ed, reason)| Absence { id: id as i64, person_id: pid as i64, person_name: name, start_date: sd, end_date: ed, reason }).collect())
    }

    pub fn create_absence(&self, input: &CreateAbsence) -> Result<Absence, DbError> {
        let mut c = self.conn()?;
        diesel::insert_into(absences::table).values(NewAbsence {
            person_id: input.person_id as i32,
            start_date: &input.start_date,
            end_date: &input.end_date,
            reason: &input.reason,
        }).execute(&mut c)?;
        let a = absences::table.order(absences::id.desc()).select(AbsenceRow::as_select()).first(&mut c)?;
        let name: String = people::table.filter(people::id.eq(input.person_id as i32)).select(people::name).first(&mut c)?;
        Ok(Absence { id: a.id as i64, person_id: a.person_id as i64, person_name: name, start_date: a.start_date, end_date: a.end_date, reason: a.reason })
    }

    pub fn delete_absence(&self, id: i64) -> Result<(), DbError> {
        let mut c = self.conn()?;
        diesel::delete(absences::table.filter(absences::id.eq(id as i32))).execute(&mut c)?;
        Ok(())
    }

    // ─── Period View ───

    pub fn period_view(&self, period_id: i64, building_id: i64) -> Result<PeriodView, DbError> {
        let period = self.get_period(period_id)?;
        let building = self.get_building(building_id)?;
        let rooms = self.list_rooms(building_id)?;
        let mut c = self.conn()?;
        let (perid, bid) = (period_id as i32, building_id as i32);

        let raw: Vec<(i32, i32, i32, String, String, i32, String, String, i32, i32)> = assignments::table
            .filter(assignments::period_id.eq(perid))
            .inner_join(rooms::table.on(assignments::room_id.eq(rooms::id)))
            .inner_join(people::table.on(assignments::person_id.eq(people::id)))
            .filter(rooms::building_id.eq(bid))
            .order((assignments::day, rooms::name))
            .select((assignments::id, assignments::period_id, assignments::person_id, people::name, people::gender, assignments::room_id, rooms::name, rooms::key, rooms::building_id, assignments::day))
            .load(&mut c)?;

        let assignment_list: Vec<Assignment> = raw.into_iter().map(|r| Assignment {
            id: r.0 as i64, period_id: r.1 as i64, person_id: r.2 as i64, person_name: r.3, person_gender: r.4,
            room_id: r.5 as i64, room_name: r.6, room_key: r.7, building_id: r.8 as i64, day: r.9,
        }).collect();

        let absence_list = self.list_absences_for_period(&period)?;
        let absent_ids: Vec<i64> = absence_list.iter().map(|a| a.person_id).collect();

        let mut day_schedules = Vec::new();
        for d in 0..(period.num_days as i32) {
            let da: Vec<Assignment> = assignment_list.iter().filter(|a| a.day == d).cloned().collect();
            day_schedules.push(DaySchedule { day: d, date: day_label(&period.start_date, d as i64), assignments: da });
        }

        let people_list = self.list_duty_people(building_id)?;
        let mut person_stats: Vec<PersonStat> = people_list.iter().map(|p| {
            let pid = p.id as i32;
            let total: i64 = assignments::table.filter(assignments::person_id.eq(pid)).count().get_result(&mut c).unwrap_or(0) + p.duty_offset;
            let by_room: Vec<(String, i64)> = rooms.iter().filter_map(|r| {
                let cnt: i64 = assignments::table.filter(assignments::person_id.eq(pid).and(assignments::room_id.eq(r.id as i32))).count().get_result(&mut c).unwrap_or(0);
                if cnt > 0 { Some((r.name.clone(), cnt)) } else { None }
            }).collect();
            PersonStat { person: p.clone(), total_duties: total, by_room }
        }).collect();
        person_stats.sort_by(|a, b| b.total_duties.cmp(&a.total_duties));

        let mut unfulfilled = Vec::new();
        for d in 0..(period.num_days as i32) {
            for room in &rooms {
                let count = assignment_list.iter().filter(|a| a.day == d && a.room_id == room.id).count() as i64;
                if count < room.required {
                    let reason = if let Some(a) = assignment_list.iter().find(|a| a.day == d && a.room_id == room.id && absent_ids.contains(&a.person_id)) {
                        format!("{} болеет/отсутствует", a.person_name)
                    } else { "нет назначения".to_string() };
                    unfulfilled.push(Unfulfilled { room_name: room.name.clone(), day: d, date: day_label(&period.start_date, d as i64), reason });
                }
            }
        }
        Ok(PeriodView { period, building, rooms, days: day_schedules, person_stats, absences: absence_list, unfulfilled })
    }

    // ─── Auto-fill ───

    pub fn auto_fill(&self, period_id: i64, building_id: i64) -> Result<usize, DbError> {
        let period = self.get_period(period_id)?;
        if self.period_status(&period) == "finished" {
            return Err(DbError::Forbidden("Нельзя заполнять завершённый период".into()));
        }
        let rooms = self.list_rooms(building_id)?;
        let people = self.list_duty_people(building_id)?;
        let mut c = self.conn()?;
        let perid = period_id as i32;

        let absent_ids: Vec<i32> = people.iter().filter(|p| {
            self.person_absent_on_period(p.id as i32, &period).unwrap_or(false)
        }).map(|p| p.id as i32).collect();

        for aid in &absent_ids {
            diesel::delete(assignments::table.filter(assignments::period_id.eq(perid).and(assignments::person_id.eq(*aid)))).execute(&mut c)?;
        }

        let existing: Vec<(i32, i32, i32)> = assignments::table
            .filter(assignments::period_id.eq(perid))
            .select((assignments::person_id, assignments::room_id, assignments::day))
            .load(&mut c)?;

        let mut room_day_filled: std::collections::HashMap<(i32, i32), i32> = std::collections::HashMap::new();
        let mut assigned_person_rooms: std::collections::HashMap<i32, i32> = std::collections::HashMap::new();
        for &(pid, rid, _day) in &existing {
            *room_day_filled.entry((rid, _day)).or_insert(0) += 1;
            assigned_person_rooms.entry(pid).or_insert(rid);
        }

        let mut last_duty: std::collections::HashMap<i32, chrono::NaiveDate> = std::collections::HashMap::new();
        let raw_dates: Vec<(i32, String, i32)> = assignments::table
            .inner_join(periods::table.on(assignments::period_id.eq(periods::id)))
            .select((assignments::person_id, periods::start_date, assignments::day))
            .load(&mut c)?;
        for (pid, start_date, day) in raw_dates {
            if let Ok(d) = chrono::NaiveDate::parse_from_str(&start_date, "%Y-%m-%d") {
                let date = d + chrono::Duration::days(day as i64);
                let e = last_duty.entry(pid).or_insert(date);
                if date > *e { *e = date; }
            }
        }

        let mut duty_counts: Vec<(i32, i64, Option<chrono::NaiveDate>)> = people.iter().map(|p| {
            let cc: i64 = assignments::table.filter(assignments::person_id.eq(p.id as i32)).count().get_result(&mut c).unwrap_or(0);
            (p.id as i32, cc + p.duty_offset, last_duty.get(&(p.id as i32)).copied())
        }).collect();
        duty_counts.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.2.cmp(&b.2)));
        {
            let mut rng = rand::thread_rng();
            let mut i = 0;
            while i < duty_counts.len() {
                let mut j = i + 1;
                while j < duty_counts.len() && duty_counts[j].1 == duty_counts[i].1 && duty_counts[j].2 == duty_counts[i].2 { j += 1; }
                if j > i + 1 {
                    use rand::seq::SliceRandom;
                    duty_counts[i..j].shuffle(&mut rng);
                }
                i = j;
            }
        }

        let mut inserted = 0usize;

        let pids: Vec<i32> = duty_counts.iter().map(|(id, _, _)| *id).collect();
        let mut rng = rand::thread_rng();
        'outer: for pid in pids {
            if absent_ids.contains(&pid) { continue; }
            if assigned_person_rooms.contains_key(&pid) { continue; }
            let person = match people.iter().find(|p| p.id as i32 == pid) {
                Some(p) => p, None => continue,
            };
            let mut room_indices: Vec<usize> = (0..rooms.len()).collect();
            use rand::seq::SliceRandom;
            room_indices.shuffle(&mut rng);
            for &ri in &room_indices {
                let room = &rooms[ri];
                let rid = room.id as i32;
                if !room.gender.is_empty() && person.gender != room.gender { continue; }
                let mut has_slot = false;
                for day in 0..(period.num_days as i32) {
                    let filled = room_day_filled.get(&(rid, day)).copied().unwrap_or(0);
                    if filled < room.required as i32 {
                        has_slot = true;
                        break;
                    }
                }
                if !has_slot { continue; }
                for day in 0..(period.num_days as i32) {
                    let filled = room_day_filled.get(&(rid, day)).copied().unwrap_or(0);
                    if filled >= room.required as i32 { continue; }
                    diesel::insert_into(assignments::table).values(NewAssignment { period_id: perid, person_id: pid, room_id: rid, day }).execute(&mut c).ok();
                    *room_day_filled.entry((rid, day)).or_insert(0) += 1;
                    inserted += 1;
                }
                assigned_person_rooms.insert(pid, rid);
                if let Some(cc) = duty_counts.iter_mut().find(|(id, _, _)| *id == pid) { cc.1 += period.num_days as i64; }
                continue 'outer;
            }
        }
        Ok(inserted)
    }

    // ─── Ranking ───

    pub fn ranking(&self, building_id: Option<i64>) -> Result<Vec<RankingEntry>, DbError> {
        let mut c = self.conn()?;
        let people_rows = match building_id {
            Some(bid) => people::table.filter(people::building_id.eq(bid as i32).and(people::active.eq(1)).and(people::role.ne("superadmin"))).select(PersonRow::as_select()).load::<PersonRow>(&mut c)?,
            None => people::table.filter(people::active.eq(1)).select(PersonRow::as_select()).load::<PersonRow>(&mut c)?,
        };
        let mut entries: Vec<RankingEntry> = people_rows.into_iter().map(|p| {
            let total: i64 = assignments::table.filter(assignments::person_id.eq(p.id)).count().get_result(&mut c).unwrap_or(0) + p.duty_offset as i64;
            RankingEntry { person: p.into_api(), total_duties: total, rank: 0 }
        }).collect();
        entries.sort_by_key(|e| e.total_duties);
        for (i, e) in entries.iter_mut().enumerate() { e.rank = i + 1; }
        Ok(entries)
    }

    // ─── Person History ───

    pub fn person_history(&self, person_id: i64) -> Result<PersonHistory, DbError> {
        let mut c = self.conn()?;
        let pid = person_id as i32;
        let person = people::table.filter(people::id.eq(pid)).select(PersonRow::as_select()).first(&mut c)?.into_api();
        let period_rows: Vec<PeriodRow> = periods::table.inner_join(assignments::table.on(assignments::period_id.eq(periods::id)))
            .filter(assignments::person_id.eq(pid)).distinct().order(periods::start_date.desc())
            .select(PeriodRow::as_select()).load(&mut c)?;
        let mut result = Vec::new();
        for p in period_rows {
            let raw: Vec<(i32, i32, i32, String, String, i32, String, String, i32, i32)> = assignments::table
                .filter(assignments::period_id.eq(p.id).and(assignments::person_id.eq(pid)))
                .inner_join(people::table.on(assignments::person_id.eq(people::id)))
                .inner_join(rooms::table.on(assignments::room_id.eq(rooms::id)))
                .order(assignments::day)
                .select((assignments::id, assignments::period_id, assignments::person_id, people::name, people::gender, assignments::room_id, rooms::name, rooms::key, rooms::building_id, assignments::day))
                .load(&mut c)?;
            let assigns = raw.into_iter().map(|r| Assignment { id: r.0 as i64, period_id: r.1 as i64, person_id: r.2 as i64, person_name: r.3, person_gender: r.4, room_id: r.5 as i64, room_name: r.6, room_key: r.7, building_id: r.8 as i64, day: r.9 }).collect();
            result.push(PeriodHistory { period: p.into_api(), assignments: assigns });
        }
        Ok(PersonHistory { person, periods: result })
    }
}

impl Period {
    pub fn end_date_str(&self) -> String {
        if let Ok(d) = chrono::NaiveDate::parse_from_str(&self.start_date, "%Y-%m-%d") {
            (d + chrono::Duration::days(self.num_days)).format("%Y-%m-%d").to_string()
        } else { self.start_date.clone() }
    }
}

fn day_label(start: &str, offset: i64) -> String {
    use chrono::Datelike;
    if let Ok(d) = chrono::NaiveDate::parse_from_str(start, "%Y-%m-%d") {
        let actual = d + chrono::Duration::days(offset);
        let wd = ["Вс","Пн","Вт","Ср","Чт","Пт","Сб"][actual.weekday().num_days_from_sunday() as usize];
        format!("{} {}", wd, actual.format("%d.%m"))
    } else { format!("Д{}", offset + 1) }
}
