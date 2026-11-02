use std::sync::atomic::{AtomicU32, Ordering};
use duty_scheduler::db::Db;
use duty_scheduler::models::*;

static COUNTER: AtomicU32 = AtomicU32::new(0);

fn test_db() -> (Db, i64) {
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let path = format!("/tmp/duty_test_{}.db", n);
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(format!("{}-wal", path));
    let _ = std::fs::remove_file(format!("{}-shm", path));
    let db = Db::new(&path).expect("Failed to create test DB");
    let buildings = db.list_buildings().unwrap();
    let bid = buildings[0].id;

    db.create_person(&CreatePerson { name: Some("Алиса".into()), gender: Some("F".into()), building_id: bid, role: Some("resident".into()) }).unwrap();
    db.create_person(&CreatePerson { name: Some("Борис".into()), gender: Some("M".into()), building_id: bid, role: Some("resident".into()) }).unwrap();
    db.create_person(&CreatePerson { name: Some("Вика".into()), gender: Some("F".into()), building_id: bid, role: Some("resident".into()) }).unwrap();
    db.create_person(&CreatePerson { name: Some("Дима".into()), gender: Some("M".into()), building_id: bid, role: Some("resident".into()) }).unwrap();
    db.create_person(&CreatePerson { name: Some("Елена".into()), gender: Some("F".into()), building_id: bid, role: Some("resident".into()) }).unwrap();
    db.create_person(&CreatePerson { name: Some("Женя".into()), gender: Some("M".into()), building_id: bid, role: Some("resident".into()) }).unwrap();
    db.create_person(&CreatePerson { name: Some("Зина".into()), gender: Some("F".into()), building_id: bid, role: Some("resident".into()) }).unwrap();
    db.create_person(&CreatePerson { name: Some("Игорь".into()), gender: Some("M".into()), building_id: bid, role: Some("resident".into()) }).unwrap();

    (db, bid)
}

fn future_date(offset_days: i64) -> String {
    (chrono::Local::now().date_naive() + chrono::Duration::days(offset_days))
        .format("%Y-%m-%d").to_string()
}

fn find_person_for_room<'a>(people: &'a [Person], room: &Room) -> &'a Person {
    people.iter().find(|p| room.gender.is_empty() || p.gender == room.gender)
        .expect("no gender-compatible person found")
}

// ─── Rooms CRUD ───

#[test]
fn room_crud() {
    let (db, bid) = test_db();

    let rooms = db.list_rooms(bid).unwrap();
    assert!(!rooms.is_empty(), "seed should create rooms");

    let new_room = db.create_room(bid, &CreateRoom {
        key: "test_room".into(), name: "Тестовая".into(), required: 1, gender: "".into(),
    }).unwrap();
    assert_eq!(new_room.name, "Тестовая");
    assert_eq!(new_room.required, 1);

    let updated = db.update_room(new_room.id, &CreateRoom {
        key: "test_room".into(), name: "Тестовая Обновлённая".into(), required: 2, gender: "M".into(),
    }).unwrap();
    assert_eq!(updated.name, "Тестовая Обновлённая");
    assert_eq!(updated.required, 2);
    assert_eq!(updated.gender, "M");

    db.delete_room(new_room.id).unwrap();
    let rooms_after = db.list_rooms(bid).unwrap();
    assert!(!rooms_after.iter().any(|r| r.id == new_room.id));
}

// ─── Copy period without assignments ───

#[test]
fn copy_period_does_not_copy_assignments() {
    let (db, bid) = test_db();
    let people = db.list_duty_people(bid).unwrap();
    let rooms = db.list_rooms(bid).unwrap();

    let p1 = db.create_period(&CreatePeriod {
        name: "Неделя 1".into(), start_date: future_date(1), num_days: 7, building_id: bid,
    }).unwrap();
    db.set_current_period(p1.id).unwrap();

    let person = find_person_for_room(&people, &rooms[0]);
    let rid = rooms[0].id;

    db.set_assignment(p1.id, &SetAssignment { person_id: person.id, room_id: rid, day: 0 }).unwrap();
    db.set_assignment(p1.id, &SetAssignment { person_id: person.id, room_id: rid, day: 1 }).unwrap();

    let p2 = db.copy_period(p1.id, &CreatePeriod {
        name: "Неделя 2".into(), start_date: future_date(8), num_days: 7, building_id: bid,
    }).unwrap();

    let view = db.period_view(p2.id, bid).unwrap();
    let total_assignments: i64 = view.days.iter().map(|d| d.assignments.len() as i64).sum();
    assert_eq!(total_assignments, 0, "copied period should have zero assignments");
}

// ─── Fill period replaces on single-capacity room ───

#[test]
fn fill_period_replaces_on_single_room() {
    let (db, bid) = test_db();
    let people = db.list_duty_people(bid).unwrap();
    let rooms = db.list_rooms(bid).unwrap();
    let kitchen = rooms.iter().find(|r| r.name == "Кухня").expect("Кухня seed room");
    assert_eq!(kitchen.required, 1);

    let p = db.create_period(&CreatePeriod {
        name: "Неделя".into(), start_date: future_date(1), num_days: 3, building_id: bid,
    }).unwrap();

    let first = find_person_for_room(&people, kitchen);
    let second = people.iter().filter(|p| p.id != first.id)
        .find(|p| kitchen.gender.is_empty() || p.gender == kitchen.gender).unwrap();

    db.fill_period(p.id, &FillPeriodRequest { person_id: first.id, room_id: kitchen.id, from_day: None }).unwrap();
    db.fill_period(p.id, &FillPeriodRequest { person_id: second.id, room_id: kitchen.id, from_day: None }).unwrap();

    let view = db.period_view(p.id, bid).unwrap();
    for day in &view.days {
        let kitchen_people: Vec<i64> = day.assignments.iter()
            .filter(|a| a.room_id == kitchen.id)
            .map(|a| a.person_id)
            .collect();
        assert_eq!(kitchen_people, vec![second.id],
            "single-capacity room should hold only the last person after re-fill");
    }
}

// ─── Fill period adds on multi-capacity room ───

#[test]
fn fill_period_adds_on_multi_room() {
    let (db, bid) = test_db();
    let people = db.list_duty_people(bid).unwrap();
    let rooms = db.list_rooms(bid).unwrap();
    let restaurant = rooms.iter().find(|r| r.name == "Ресторан").expect("Ресторан seed room");
    assert_eq!(restaurant.required, 2);

    let p = db.create_period(&CreatePeriod {
        name: "Неделя".into(), start_date: future_date(1), num_days: 3, building_id: bid,
    }).unwrap();

    let first = find_person_for_room(&people, restaurant);
    let second = people.iter().filter(|p| p.id != first.id)
        .find(|p| restaurant.gender.is_empty() || p.gender == restaurant.gender).unwrap();

    db.fill_period(p.id, &FillPeriodRequest { person_id: first.id, room_id: restaurant.id, from_day: None }).unwrap();
    db.fill_period(p.id, &FillPeriodRequest { person_id: second.id, room_id: restaurant.id, from_day: None }).unwrap();

    let view = db.period_view(p.id, bid).unwrap();
    for day in &view.days {
        let mut restaurant_people: Vec<i64> = day.assignments.iter()
            .filter(|a| a.room_id == restaurant.id)
            .map(|a| a.person_id)
            .collect();
        restaurant_people.sort();
        let mut expected = vec![first.id, second.id];
        expected.sort();
        assert_eq!(restaurant_people, expected,
            "multi-capacity room should keep both people after re-fill");
    }
}

// ─── Clear room day and whole week ───

#[test]
fn clear_room_day_and_whole_period() {
    let (db, bid) = test_db();
    let people = db.list_duty_people(bid).unwrap();
    let rooms = db.list_rooms(bid).unwrap();
    let kitchen = rooms.iter().find(|r| r.name == "Кухня").expect("Кухня seed room");

    let p = db.create_period(&CreatePeriod {
        name: "Неделя".into(), start_date: future_date(1), num_days: 3, building_id: bid,
    }).unwrap();
    let first = find_person_for_room(&people, kitchen);
    let second = people.iter().filter(|p| p.id != first.id)
        .find(|p| kitchen.gender.is_empty() || p.gender == kitchen.gender).unwrap();

    db.fill_period(p.id, &FillPeriodRequest { person_id: first.id, room_id: kitchen.id, from_day: None }).unwrap();
    db.fill_period(p.id, &FillPeriodRequest { person_id: second.id, room_id: kitchen.id, from_day: None }).unwrap();

    let n = db.clear_room(p.id, kitchen.id, Some(0)).unwrap();
    assert_eq!(n, 1, "day clear should remove only day 0 assignment");

    let view = db.period_view(p.id, bid).unwrap();
    for day in &view.days {
        let kitchen_people: Vec<i64> = day.assignments.iter()
            .filter(|a| a.room_id == kitchen.id)
            .map(|a| a.person_id)
            .collect();
        if day.day == 0 {
            assert!(kitchen_people.is_empty(), "day 0 should be empty after clear");
        } else {
            assert_eq!(kitchen_people, vec![second.id], "other days should keep the assignment");
        }
    }

    let n = db.clear_room(p.id, kitchen.id, None).unwrap();
    assert_eq!(n, 2, "week clear should remove remaining day 1 and 2 assignments");

    let view = db.period_view(p.id, bid).unwrap();
    for day in &view.days {
        let kitchen_people: Vec<i64> = day.assignments.iter()
            .filter(|a| a.room_id == kitchen.id)
            .map(|a| a.person_id)
            .collect();
        assert!(kitchen_people.is_empty(), "kitchen should be fully empty after week clear");
    }
}

#[test]
fn clear_room_finished_period_fails() {
    let (db, bid) = test_db();
    let rooms = db.list_rooms(bid).unwrap();

    let p = db.create_period(&CreatePeriod {
        name: "Старая".into(), start_date: future_date(-20), num_days: 7, building_id: bid,
    }).unwrap();

    assert!(db.clear_room(p.id, rooms[0].id, None).is_err(), "finished period must not be editable");
    assert!(db.clear_room(p.id, rooms[0].id, Some(0)).is_err(), "finished period must not be editable");
}

// ─── Ranking counts distinct periods ───

#[test]
fn ranking_counts_total_assignments() {
    let (db, bid) = test_db();
    let people = db.list_duty_people(bid).unwrap();
    let rooms = db.list_rooms(bid).unwrap();

    let person = find_person_for_room(&people, &rooms[0]);

    let p1 = db.create_period(&CreatePeriod {
        name: "P1".into(), start_date: future_date(1), num_days: 7, building_id: bid,
    }).unwrap();

    db.set_assignment(p1.id, &SetAssignment { person_id: person.id, room_id: rooms[0].id, day: 0 }).unwrap();
    db.set_assignment(p1.id, &SetAssignment { person_id: person.id, room_id: rooms[0].id, day: 1 }).unwrap();
    db.set_assignment(p1.id, &SetAssignment { person_id: person.id, room_id: rooms[0].id, day: 2 }).unwrap();

    let p2 = db.create_period(&CreatePeriod {
        name: "P2".into(), start_date: future_date(10), num_days: 7, building_id: bid,
    }).unwrap();
    db.set_assignment(p2.id, &SetAssignment { person_id: person.id, room_id: rooms[0].id, day: 0 }).unwrap();

    let ranking = db.ranking(Some(bid)).unwrap();
    let entry = ranking.iter().find(|e| e.person.id == person.id).unwrap();
    assert_eq!(entry.total_duties, 4, "should count 4 total assignments, not 2 periods");
}

#[test]
fn ranking_person_without_duties_is_first() {
    let (db, bid) = test_db();
    let people = db.list_duty_people(bid).unwrap();
    let rooms = db.list_rooms(bid).unwrap();

    let person = find_person_for_room(&people, &rooms[0]);
    let other = people.iter().find(|p| p.id != person.id && (rooms[0].gender.is_empty() || p.gender == rooms[0].gender))
        .expect("need second gender-compatible person");

    let p1 = db.create_period(&CreatePeriod {
        name: "P1".into(), start_date: future_date(1), num_days: 7, building_id: bid,
    }).unwrap();
    db.set_assignment(p1.id, &SetAssignment { person_id: person.id, room_id: rooms[0].id, day: 0 }).unwrap();

    let ranking = db.ranking(Some(bid)).unwrap();
    let without_duty = ranking.iter().find(|e| e.person.id == other.id).unwrap();
    let with_duty = ranking.iter().find(|e| e.person.id == person.id).unwrap();
    assert_eq!(without_duty.total_duties, 0);
    assert_eq!(with_duty.total_duties, 1);
    assert!(without_duty.rank < with_duty.rank, "person without duties should rank higher");
}

// ─── Auto-fill: one person = one room for whole period ───

#[test]
fn auto_fill_one_person_per_room_per_period() {
    let (db, bid) = test_db();

    let p = db.create_period(&CreatePeriod {
        name: "Test".into(), start_date: future_date(1), num_days: 3, building_id: bid,
    }).unwrap();

    let count = db.auto_fill(p.id, bid).unwrap();
    assert!(count > 0, "auto-fill should insert assignments");

    let view = db.period_view(p.id, bid).unwrap();
    for day in &view.days {
        let mut assigned_people: Vec<i64> = day.assignments.iter().map(|a| a.person_id).collect();
        assigned_people.sort();
        assigned_people.dedup();
        assert_eq!(assigned_people.len(), day.assignments.len(),
            "each person should appear at most once per day");
    }

    let people = db.list_duty_people(bid).unwrap();
    for person in &people {
        let rooms_assigned: Vec<i64> = view.days.iter()
            .flat_map(|d| d.assignments.iter())
            .filter(|a| a.person_id == person.id)
            .map(|a| a.room_id)
            .collect();
        if !rooms_assigned.is_empty() {
            let mut unique_rooms = rooms_assigned.clone();
            unique_rooms.sort();
            unique_rooms.dedup();
            assert_eq!(unique_rooms.len(), 1,
                "person {} should be assigned to only one room across entire period", person.name);
        }
    }
}

// ─── Auto-fill respects gender ───

#[test]
fn auto_fill_respects_gender() {
    let (db, bid) = test_db();

    let p = db.create_period(&CreatePeriod {
        name: "Test".into(), start_date: future_date(1), num_days: 3, building_id: bid,
    }).unwrap();

    db.auto_fill(p.id, bid).unwrap();

    let view = db.period_view(p.id, bid).unwrap();
    let rooms = db.list_rooms(bid).unwrap();
    let people = db.list_duty_people(bid).unwrap();

    for day in &view.days {
        for assignment in &day.assignments {
            let room = rooms.iter().find(|r| r.id == assignment.room_id).unwrap();
            let person = people.iter().find(|p| p.id == assignment.person_id).unwrap();
            if !room.gender.is_empty() {
                assert_eq!(room.gender, person.gender,
                    "person {} ({}) should not be in room {} ({})", person.name, person.gender, room.name, room.gender);
            }
        }
    }
}

// ─── Auto-fill ranking: fewer periods first ───

#[test]
fn auto_fill_prioritizes_less_served() {
    let (db, bid) = test_db();
    let people = db.list_duty_people(bid).unwrap();
    let rooms = db.list_rooms(bid).unwrap();

    let person = find_person_for_room(&people, &rooms[0]);

    let p1 = db.create_period(&CreatePeriod {
        name: "P1".into(), start_date: future_date(1), num_days: 1, building_id: bid,
    }).unwrap();
    db.set_assignment(p1.id, &SetAssignment { person_id: person.id, room_id: rooms[0].id, day: 0 }).unwrap();

    let p2 = db.create_period(&CreatePeriod {
        name: "P2".into(), start_date: future_date(3), num_days: 1, building_id: bid,
    }).unwrap();
    db.auto_fill(p2.id, bid).unwrap();

    let rank = db.ranking(Some(bid)).unwrap();
    let entry = rank.iter().find(|r| r.person.id == person.id).unwrap();
    assert_eq!(entry.total_duties, 2);
    let others: Vec<_> = rank.iter().filter(|r| r.person.id != person.id).collect();
    assert!(others.iter().all(|r| r.total_duties <= 1),
        "all others should have at most 1 duty total");
}

// ─── Absences (date-range based) ───

#[test]
fn absence_prevents_assignment() {
    let (db, bid) = test_db();
    let people = db.list_duty_people(bid).unwrap();
    let rooms = db.list_rooms(bid).unwrap();

    let person = find_person_for_room(&people, &rooms[0]);

    let p = db.create_period(&CreatePeriod {
        name: "Test".into(), start_date: future_date(1), num_days: 3, building_id: bid,
    }).unwrap();

    let _period = db.get_period(p.id).unwrap();
    db.create_absence(&CreateAbsence {
        person_id: person.id,
        start_date: future_date(0),
        end_date: future_date(5),
        reason: "Болеет".into(),
    }).unwrap();

    let result = db.set_assignment(p.id, &SetAssignment {
        person_id: person.id, room_id: rooms[0].id, day: 0,
    });
    assert!(result.is_err(), "cannot assign absent person");
}

#[test]
fn temporary_absence_no_block_outside_range() {
    let (db, bid) = test_db();
    let people = db.list_duty_people(bid).unwrap();
    let rooms = db.list_rooms(bid).unwrap();

    let person = find_person_for_room(&people, &rooms[0]);

    db.create_absence(&CreateAbsence {
        person_id: person.id,
        start_date: future_date(1),
        end_date: future_date(3),
        reason: "Болеет".into(),
    }).unwrap();

    let p = db.create_period(&CreatePeriod {
        name: "Test".into(), start_date: future_date(10), num_days: 3, building_id: bid,
    }).unwrap();

    let result = db.set_assignment(p.id, &SetAssignment {
        person_id: person.id, room_id: rooms[0].id, day: 0,
    });
    assert!(result.is_ok(), "absence outside period range should not block assignment");
}

#[test]
fn absence_crud() {
    let (db, bid) = test_db();
    let people = db.list_duty_people(bid).unwrap();

    let a = db.create_absence(&CreateAbsence {
        person_id: people[0].id,
        start_date: future_date(1),
        end_date: future_date(7),
        reason: "Отпуск".into(),
    }).unwrap();
    assert_eq!(a.reason, "Отпуск");
    assert_eq!(a.person_id, people[0].id);

    let list = db.list_absences_for_building(bid).unwrap();
    assert!(list.iter().any(|x| x.id == a.id));

    db.delete_absence(a.id).unwrap();
    let list2 = db.list_absences_for_building(bid).unwrap();
    assert!(!list2.iter().any(|x| x.id == a.id));
}

// ─── Period CRUD ───

#[test]
fn period_crud() {
    let (db, bid) = test_db();

    let p = db.create_period(&CreatePeriod {
        name: "Тест".into(), start_date: future_date(1), num_days: 7, building_id: bid,
    }).unwrap();
    assert_eq!(p.name, "Тест");
    assert_eq!(p.num_days, 7);

    let periods = db.list_periods(bid).unwrap();
    assert!(periods.iter().any(|pp| pp.id == p.id));

    db.delete_period(p.id).unwrap();
    let periods_after = db.list_periods(bid).unwrap();
    assert!(!periods_after.iter().any(|pp| pp.id == p.id));
}

// ─── Person history ───

#[test]
fn person_history_across_periods() {
    let (db, bid) = test_db();
    let people = db.list_duty_people(bid).unwrap();
    let rooms = db.list_rooms(bid).unwrap();

    let person = find_person_for_room(&people, &rooms[0]);

    let p1 = db.create_period(&CreatePeriod {
        name: "P1".into(), start_date: future_date(1), num_days: 3, building_id: bid,
    }).unwrap();
    db.set_assignment(p1.id, &SetAssignment { person_id: person.id, room_id: rooms[0].id, day: 0 }).unwrap();

    let p2 = db.create_period(&CreatePeriod {
        name: "P2".into(), start_date: future_date(10), num_days: 3, building_id: bid,
    }).unwrap();
    db.set_assignment(p2.id, &SetAssignment { person_id: person.id, room_id: rooms[0].id, day: 1 }).unwrap();

    let history = db.person_history(person.id).unwrap();
    assert_eq!(history.periods.len(), 2, "should have 2 periods in history");
}

// ─── Uniqueness: cannot assign same person to same room twice ───

#[test]
fn duplicate_assignment_blocked() {
    let (db, bid) = test_db();
    let people = db.list_duty_people(bid).unwrap();
    let rooms = db.list_rooms(bid).unwrap();

    let person = find_person_for_room(&people, &rooms[0]);

    let p = db.create_period(&CreatePeriod {
        name: "Test".into(), start_date: future_date(1), num_days: 1, building_id: bid,
    }).unwrap();

    let r1 = db.set_assignment(p.id, &SetAssignment { person_id: person.id, room_id: rooms[0].id, day: 0 });
    assert!(r1.is_ok());

    let r2 = db.set_assignment(p.id, &SetAssignment { person_id: person.id, room_id: rooms[0].id, day: 0 });
    assert!(r2.is_err(), "duplicate assignment should fail");
}

// ─── Person stats count distinct periods ───

#[test]
fn person_stats_counts_total_assignments() {
    let (db, bid) = test_db();
    let people = db.list_duty_people(bid).unwrap();
    let rooms = db.list_rooms(bid).unwrap();

    let person = find_person_for_room(&people, &rooms[0]);

    let p1 = db.create_period(&CreatePeriod {
        name: "P1".into(), start_date: future_date(1), num_days: 3, building_id: bid,
    }).unwrap();
    db.set_current_period(p1.id).unwrap();
    db.set_assignment(p1.id, &SetAssignment { person_id: person.id, room_id: rooms[0].id, day: 0 }).unwrap();
    db.set_assignment(p1.id, &SetAssignment { person_id: person.id, room_id: rooms[0].id, day: 1 }).unwrap();
    db.set_assignment(p1.id, &SetAssignment { person_id: person.id, room_id: rooms[0].id, day: 2 }).unwrap();

    let p2 = db.create_period(&CreatePeriod {
        name: "P2".into(), start_date: future_date(10), num_days: 3, building_id: bid,
    }).unwrap();
    db.set_current_period(p2.id).unwrap();
    db.set_assignment(p2.id, &SetAssignment { person_id: person.id, room_id: rooms[0].id, day: 0 }).unwrap();

    let view = db.period_view(p2.id, bid).unwrap();
    let stat = view.person_stats.iter().find(|s| s.person.id == person.id).unwrap();
    assert_eq!(stat.total_duties, 4, "person_stats should count 4 total assignments, not 2 periods");
}

// ─── Auto-fill empty building ───

#[test]
fn auto_fill_with_no_people() {
    let path = "/tmp/duty_test_nopeople.db";
    let _ = std::fs::remove_file(path);
    let _ = std::fs::remove_file(format!("{}-wal", path));
    let _ = std::fs::remove_file(format!("{}-shm", path));
    let db = Db::new(path).unwrap();
    let b = db.create_building("Empty").unwrap();

    let p = db.create_period(&CreatePeriod {
        name: "Empty".into(), start_date: future_date(1), num_days: 3, building_id: b.id,
    }).unwrap();
    let count = db.auto_fill(p.id, b.id).unwrap();
    assert_eq!(count, 0);
}

// ─── Delete / edit period by status ───

#[test]
fn delete_period_not_started() {
    let (db, bid) = test_db();
    let p = db.create_period(&CreatePeriod {
        name: "Future".into(), start_date: future_date(10), num_days: 7, building_id: bid,
    }).unwrap();
    let result = db.delete_period(p.id);
    assert!(result.is_ok(), "should delete period that hasn't started");
    assert!(db.list_periods(bid).unwrap().iter().find(|x| x.id == p.id).is_none());
}

#[test]
fn delete_period_started_fails() {
    let (db, bid) = test_db();
    let p = db.create_period(&CreatePeriod {
        name: "Active".into(), start_date: future_date(-2), num_days: 7, building_id: bid,
    }).unwrap();
    let result = db.delete_period(p.id);
    assert!(result.is_err(), "should not delete active period");
}

#[test]
fn delete_period_finished_fails() {
    let (db, bid) = test_db();
    let p = db.create_period(&CreatePeriod {
        name: "Old".into(), start_date: future_date(-20), num_days: 7, building_id: bid,
    }).unwrap();
    let result = db.delete_period(p.id);
    assert!(result.is_err(), "should not delete finished period");
}

#[test]
fn edit_period_not_started() {
    let (db, bid) = test_db();
    let p = db.create_period(&CreatePeriod {
        name: "Future".into(), start_date: future_date(10), num_days: 7, building_id: bid,
    }).unwrap();
    let updated = db.update_period(p.id, &UpdatePeriod {
        name: Some("Updated".into()),
        start_date: None,
        num_days: Some(5),
    }).unwrap();
    assert_eq!(updated.name, "Updated");
    assert_eq!(updated.num_days, 5);
}

#[test]
fn edit_period_finished_fails() {
    let (db, bid) = test_db();
    let p = db.create_period(&CreatePeriod {
        name: "Old".into(), start_date: future_date(-20), num_days: 7, building_id: bid,
    }).unwrap();
    let result = db.update_period(p.id, &UpdatePeriod {
        name: Some("Nope".into()),
        start_date: None,
        num_days: None,
    });
    assert!(result.is_err(), "should not edit finished period");
}

// ─── Token / Auth (unified people table) ───

#[test]
fn person_crud_and_activate() {
    let (db, bid) = test_db();

    let person = db.create_person(&CreatePerson { name: Some("Админ".into()), gender: Some("M".into()), building_id: bid, role: Some("admin".into()) }).unwrap();
    assert_eq!(person.role, "admin");
    assert!(person.used_at.is_none());
    assert!(!person.token.is_empty());

    let list = db.list_people_for_tokens(bid, &person.token).unwrap();
    assert!(list.iter().any(|p| p.id == person.id), "admin person should appear in token list");

    let ctx = db.activate_token(&person.token, Some("Админ"), None).unwrap();
    assert_eq!(ctx.building_id, bid);
    assert_eq!(ctx.role, "admin");

    let me = db.validate_token(&person.token).unwrap();
    assert_eq!(me.building_id, bid);
    assert_eq!(me.role, "admin");

    let list = db.list_people_for_tokens(bid, &person.token).unwrap();
    let me = list.iter().find(|p| p.id == person.id).unwrap();
    assert!(me.used_at.is_some());
}

#[test]
fn activate_twice_fails() {
    let (db, bid) = test_db();
    // Admin CAN re-activate
    let person = db.create_person(&CreatePerson { name: None, gender: None, building_id: bid, role: Some("admin".into()) }).unwrap();
    db.activate_token(&person.token, Some("Admin"), None).unwrap();
    let result = db.activate_token(&person.token, Some("Admin2"), None);
    assert!(result.is_ok(), "admin should be able to re-activate");

    // Resident CAN also re-activate (no duplicate person created)
    let person2 = db.create_person(&CreatePerson { name: None, gender: None, building_id: bid, role: Some("resident".into()) }).unwrap();
    let ctx1 = db.activate_token(&person2.token, Some("Resident1"), Some("M")).unwrap();
    let people_count_before = db.list_duty_people(bid).unwrap().len();
    let ctx2 = db.activate_token(&person2.token, Some("Resident1"), Some("M")).unwrap();
    let people_count_after = db.list_duty_people(bid).unwrap().len();
    assert!(people_count_after == people_count_before, "re-activate should not create duplicate person");
    assert_eq!(ctx2.name.as_deref(), ctx1.name.as_deref());
}

#[test]
fn person_delete_deactivates() {
    let (db, bid) = test_db();
    let person = db.create_person(&CreatePerson { name: Some("Test".into()), gender: Some("M".into()), building_id: bid, role: Some("resident".into()) }).unwrap();
    db.delete_person(person.id).unwrap();
    let result = db.validate_token(&person.token);
    assert!(result.is_err(), "deleted person token should not validate");
}

#[test]
fn resident_token_creates_person() {
    let (db, bid) = test_db();
    let person = db.create_person(&CreatePerson { name: None, gender: None, building_id: bid, role: Some("resident".into()) }).unwrap();
    let ctx = db.activate_token(&person.token, Some("Новый"), Some("M")).unwrap();
    assert_eq!(ctx.name.as_deref(), Some("Новый"));

    let people = db.list_duty_people(bid).unwrap();
    assert!(people.iter().any(|p| p.name == "Новый"), "resident should be in people list");
}

#[test]
fn admin_token_old_admin_downgraded() {
    let (db, bid) = test_db();
    let p1 = db.create_person(&CreatePerson { name: None, gender: None, building_id: bid, role: Some("admin".into()) }).unwrap();
    db.activate_token(&p1.token, Some("Admin1"), None).unwrap();

    let p2 = db.create_person(&CreatePerson { name: None, gender: None, building_id: bid, role: Some("admin".into()) }).unwrap();
    db.activate_token(&p2.token, Some("Admin2"), None).unwrap();

    // Old admin still can log in but is now resident
    let old_ctx = db.validate_token(&p1.token).unwrap();
    assert_eq!(old_ctx.role, "resident", "old admin should be downgraded to resident");
    assert!(db.validate_token(&p2.token).is_ok(), "new admin token should work");
}

// ─── Newcomer starts with building's minimum rating ───

#[test]
fn newcomer_starts_with_building_min_rating() {
    let (db, bid) = test_db();
    let people = db.list_duty_people(bid).unwrap();
    let rooms = db.list_rooms(bid).unwrap();

    // Give every resident exactly 1 duty day (one 1-day period each).
    let mut day = 1;
    for person in &people {
        let p = db.create_period(&CreatePeriod {
            name: "P".into(), start_date: future_date(day), num_days: 1, building_id: bid,
        }).unwrap();
        let room = rooms.iter().find(|r| r.gender.is_empty() || r.gender == person.gender).unwrap();
        db.set_assignment(p.id, &SetAssignment { person_id: person.id, room_id: room.id, day: 0 }).unwrap();
        day += 2;
    }

    let rank = db.ranking(Some(bid)).unwrap();
    assert!(rank.iter().all(|e| e.total_duties >= 1), "every resident should have at least 1 duty day");

    let newcomer = db.create_person(&CreatePerson {
        name: Some("Новенький".into()), gender: Some("M".into()), building_id: bid, role: Some("resident".into()),
    }).unwrap();
    assert_eq!(newcomer.duty_offset, 1, "newcomer should start with building min rating (1 day)");

    let rank = db.ranking(Some(bid)).unwrap();
    let entry = rank.iter().find(|e| e.person.id == newcomer.id).unwrap();
    assert_eq!(entry.total_duties, 1, "ranking should show effective rating for newcomer");
}

#[test]
fn newcomer_offset_zero_when_building_empty() {
    let path = "/tmp/duty_test_empty_building.db";
    let _ = std::fs::remove_file(path);
    let _ = std::fs::remove_file(format!("{}-wal", path));
    let _ = std::fs::remove_file(format!("{}-shm", path));
    let db = Db::new(path).unwrap();
    let b = db.create_building("Empty").unwrap();

    let newcomer = db.create_person(&CreatePerson {
        name: Some("Первый".into()), gender: Some("M".into()), building_id: b.id, role: Some("resident".into()),
    }).unwrap();
    assert_eq!(newcomer.duty_offset, 0, "no other residents -> offset 0");
}

// ─── Auto-fill tie-break by last duty date ───

fn kitchen_winner_after_auto_fill(old_person_id: i64, recent_person_id: i64) -> String {
    let (db, bid) = test_db();
    let people = db.list_duty_people(bid).unwrap();
    let rooms = db.list_rooms(bid).unwrap();
    let kitchen = rooms.iter().find(|r| r.name == "Кухня").unwrap();

    let p_old = db.create_period(&CreatePeriod { name: "Старый".into(), start_date: future_date(-40), num_days: 7, building_id: bid }).unwrap();
    let p_recent = db.create_period(&CreatePeriod { name: "Свежий".into(), start_date: future_date(-10), num_days: 7, building_id: bid }).unwrap();
    let p_mid = db.create_period(&CreatePeriod { name: "Средний".into(), start_date: future_date(-25), num_days: 7, building_id: bid }).unwrap();

    db.bulk_assignments(p_old.id, &BulkAssignments { assignments: vec![SetAssignment { person_id: old_person_id, room_id: kitchen.id, day: 0 }] }).unwrap();
    db.bulk_assignments(p_recent.id, &BulkAssignments { assignments: vec![SetAssignment { person_id: recent_person_id, room_id: kitchen.id, day: 0 }] }).unwrap();

    let elena = people.iter().find(|p| p.name == "Елена").unwrap();
    let zina = people.iter().find(|p| p.name == "Зина").unwrap();
    db.bulk_assignments(p_mid.id, &BulkAssignments { assignments: vec![
        SetAssignment { person_id: elena.id, room_id: kitchen.id, day: 0 },
        SetAssignment { person_id: elena.id, room_id: kitchen.id, day: 1 },
        SetAssignment { person_id: zina.id, room_id: kitchen.id, day: 2 },
        SetAssignment { person_id: zina.id, room_id: kitchen.id, day: 3 },
    ] }).unwrap();

    let p_new = db.create_period(&CreatePeriod { name: "Новый".into(), start_date: future_date(1), num_days: 3, building_id: bid }).unwrap();
    db.auto_fill(p_new.id, bid).unwrap();

    let view = db.period_view(p_new.id, bid).unwrap();
    let kitchen_people: Vec<i64> = view.days[0].assignments.iter()
        .filter(|a| a.room_id == kitchen.id)
        .map(|a| a.person_id)
        .collect();
    assert_eq!(kitchen_people.len(), 1, "kitchen must hold exactly one person");
    people.iter().find(|p| p.id == kitchen_people[0]).unwrap().name.clone()
}

#[test]
fn auto_fill_tie_break_prefers_older_duty() {
    let (db, bid) = test_db();
    let people = db.list_duty_people(bid).unwrap();
    let alisa = people.iter().find(|p| p.name == "Алиса").unwrap();
    let vika = people.iter().find(|p| p.name == "Вика").unwrap();

    let winner = kitchen_winner_after_auto_fill(alisa.id, vika.id);
    assert_eq!(winner, "Алиса", "with equal rating the one who served longer ago wins");
}

#[test]
fn auto_fill_tie_break_swaps_with_swapped_dates() {
    let (db, bid) = test_db();
    let people = db.list_duty_people(bid).unwrap();
    let alisa = people.iter().find(|p| p.name == "Алиса").unwrap();
    let vika = people.iter().find(|p| p.name == "Вика").unwrap();

    let winner = kitchen_winner_after_auto_fill(vika.id, alisa.id);
    assert_eq!(winner, "Вика", "swapped duty dates should flip the winner");
}

#[test]
fn auto_fill_prefers_never_served_on_equal_rating() {
    let (db, bid) = test_db();
    let people = db.list_duty_people(bid).unwrap();
    let rooms = db.list_rooms(bid).unwrap();
    let kitchen = rooms.iter().find(|r| r.name == "Кухня").unwrap();
    let m_rooms: Vec<_> = rooms.iter().filter(|r| r.name != "Кухня").collect();

    let p_old = db.create_period(&CreatePeriod { name: "Старый".into(), start_date: future_date(-20), num_days: 7, building_id: bid }).unwrap();
    let f_people: Vec<_> = people.iter().filter(|p| p.gender == "F").collect();
    let m_people: Vec<_> = people.iter().filter(|p| p.gender == "M").collect();
    for (i, p) in f_people.iter().enumerate() {
        db.bulk_assignments(p_old.id, &BulkAssignments { assignments: vec![SetAssignment { person_id: p.id, room_id: kitchen.id, day: i as i32 }] }).unwrap();
    }
    for (i, p) in m_people.iter().enumerate() {
        db.bulk_assignments(p_old.id, &BulkAssignments { assignments: vec![SetAssignment { person_id: p.id, room_id: m_rooms[i].id, day: i as i32 }] }).unwrap();
    }

    let newcomer = db.create_person(&CreatePerson {
        name: Some("Новенькая".into()), gender: Some("F".into()), building_id: bid, role: Some("resident".into()),
    }).unwrap();
    assert_eq!(newcomer.duty_offset, 1, "newcomer starts at building min rating (1 day)");

    let p_new = db.create_period(&CreatePeriod { name: "Новый".into(), start_date: future_date(1), num_days: 3, building_id: bid }).unwrap();
    db.auto_fill(p_new.id, bid).unwrap();

    let view = db.period_view(p_new.id, bid).unwrap();
    let kitchen_people: Vec<i64> = view.days[0].assignments.iter()
        .filter(|a| a.room_id == kitchen.id)
        .map(|a| a.person_id)
        .collect();
    assert_eq!(kitchen_people, vec![newcomer.id],
        "never-served person with equal effective rating should win the tie");
}

// ─── Public share token ───

#[test]
fn building_public_token_roundtrip() {
    let (db, bid) = test_db();
    let t1 = db.building_public_token(bid).unwrap();
    assert_eq!(t1.len(), 16, "token should be 16 chars");

    let t2 = db.building_public_token(bid).unwrap();
    assert_eq!(t1, t2, "token should be stable across calls");

    let b = db.building_by_public_token(&t1).unwrap();
    assert_eq!(b.id, bid, "lookup by public token should return the building");
}

#[test]
fn building_public_token_unknown_rejected() {
    let (db, _bid) = test_db();
    assert!(db.building_by_public_token("nonexistent").is_err(), "unknown token -> error");
}

#[test]
fn public_token_grants_read_but_not_write() {
    let (db, bid) = test_db();
    let t = db.building_public_token(bid).unwrap();

    let p = db.create_period(&CreatePeriod {
        name: "Публичный".into(), start_date: future_date(1), num_days: 3, building_id: bid,
    }).unwrap();

    let info = db.building_by_public_token(&t).unwrap();
    assert_eq!(info.id, bid);

    let view = db.period_view(p.id, bid).unwrap();
    assert_eq!(view.building.id, bid, "public view should be building-scoped");
}
