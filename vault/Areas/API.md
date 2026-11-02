# API

## Здания

| Метод | Путь | Описание |
|-------|------|----------|
| GET | `/api/buildings` | Список зданий |
| POST | `/api/buildings` | Создать здание `{name}` |
| GET | `/api/buildings/{id}` | Детали здания |
| DELETE | `/api/buildings/{id}` | Удалить здание (soft) |

## Помещения

| Метод | Путь | Описание |
|-------|------|----------|
| GET | `/api/buildings/{id}/rooms` | Помещения здания |
| POST | `/api/buildings/{id}/rooms` | Добавить `{key, name, required, gender}` |
| DELETE | `/api/rooms/{id}` | Удалить помещение (soft) |

## Люди

| Метод | Путь | Описание |
|-------|------|----------|
| GET | `/api/people?building_id=` | Люди здания |
| POST | `/api/people` | Добавить `{name, gender, building_id}` |
| PUT | `/api/people/{id}` | Редактировать `{name, gender}` |
| DELETE | `/api/people/{id}` | Деактивировать (soft delete) |
| GET | `/api/people/{id}/history` | История дежурств по периодам |

## Периоды

| Метод | Путь | Описание |
|-------|------|----------|
| GET | `/api/periods` | Список периодов |
| POST | `/api/periods` | Создать `{name, start_date, num_days}` |
| GET | `/api/periods/current` | Текущий период |
| GET | `/api/periods/{id}` | Детали периода |
| DELETE | `/api/periods/{id}` | Удалить |
| POST | `/api/periods/{id}/set-current` | Сделать текущим |
| GET | `/api/periods/{id}/view?building_id=` | Полный вид (назначения +.stats) |
| POST | `/api/periods/{id}/copy` | Копировать `{name, start_date, num_days}` |
| POST | `/api/periods/next` | **Создать следующий период** (автоматически после текущего, та же длительность) |
| POST | `/api/periods/{id}/auto-fill` | **Автозаполнение** по рейтингу `{building_id}` |
| POST | `/api/periods/{id}/bulk` | **Массовый ввод истории** `{assignments: [{person_id, room_id, day}]}` |

## Назначения

| Метод | Путь | Описание |
|-------|------|----------|
| POST | `/api/periods/{id}/assignments` | Назначить `{person_id, room_id, day}` |
| DELETE | `/api/periods/{id}/assignments` | Снять `{person_id, room_id, day}` |

## Отсутствия

| Метод | Путь | Описание |
|-------|------|----------|
| GET | `/api/periods/{id}/absences` | Список отсутствующих |
| POST | `/api/periods/{id}/absences` | Пометить `{person_id, reason}` |
| DELETE | `/api/periods/{id}/absences/{pid}` | Снять отсутствие |

## Рейтинг и статистика

| Метод | Путь | Описание |
|-------|------|----------|
| GET | `/api/ranking?building_id=` | Очередь на дежурство (сортировка asc) |
| GET | `/api/stats/all` | Глобальная статистика |
