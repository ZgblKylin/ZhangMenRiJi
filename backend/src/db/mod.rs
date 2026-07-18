use crate::models::game::{CreateGameRequest, GameListItem, GameState};
use sqlx::SqlitePool;
use uuid::Uuid;

/// 启动时自动建表（幂等）
pub async fn run_migrations(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS games (
            id          TEXT PRIMARY KEY,
            sect_name   TEXT NOT NULL,
            state       TEXT NOT NULL,
            schema_version INTEGER NOT NULL DEFAULT 3,
            save_group_id TEXT NOT NULL,
            save_type   TEXT NOT NULL DEFAULT 'auto',
            created_at  TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at  TEXT NOT NULL DEFAULT (datetime('now'))
        )",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS game_snapshots (
            game_id TEXT NOT NULL,
            turn INTEGER NOT NULL,
            state TEXT NOT NULL,
            checksum TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            PRIMARY KEY (game_id, turn)
        )",
    )
    .execute(pool)
    .await?;
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS events (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            game_id     TEXT NOT NULL,
            year        INTEGER NOT NULL,
            month       INTEGER NOT NULL,
            mood        TEXT NOT NULL,
            text        TEXT NOT NULL,
            category    TEXT NOT NULL DEFAULT 'world',
            payload     TEXT NOT NULL DEFAULT '{}',
            created_at  TEXT NOT NULL DEFAULT (datetime('now'))
        )",
    )
    .execute(pool)
    .await?;

    // 索引（幂等）
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_games_updated_at ON games (updated_at DESC)")
        .execute(pool)
        .await?;
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_games_save_group ON games (save_group_id, updated_at DESC)",
    )
    .execute(pool)
    .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_events_game_id ON events (game_id)")
        .execute(pool)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_events_game_time ON events (game_id, year DESC, month DESC)")
        .execute(pool).await?;
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_snapshots_game_turn ON game_snapshots (game_id, turn DESC)",
    )
    .execute(pool)
    .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_events_category ON events (game_id, category, year DESC, month DESC)")
        .execute(pool).await?;

    tracing::info!("数据库迁移完成");
    Ok(())
}

pub async fn create_game(
    pool: &SqlitePool,
    req: &CreateGameRequest,
) -> Result<(Uuid, GameState), sqlx::Error> {
    let mut state = GameState::default();
    state.autosave = true;
    let state_json = serde_json::to_string(&state).unwrap();
    let id = Uuid::new_v4();
    let save_group_id = Uuid::new_v4();

    sqlx::query(
        "INSERT INTO games (id, sect_name, state, schema_version, save_group_id, save_type)
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(id.to_string())
    .bind(&req.sect_name)
    .bind(&state_json)
    .bind(state.schema_version)
    .bind(save_group_id.to_string())
    .bind("auto")
    .execute(pool)
    .await?;

    Ok((id, state))
}

pub async fn get_game(
    pool: &SqlitePool,
    id: Uuid,
) -> Result<Option<(String, GameState)>, sqlx::Error> {
    let row =
        sqlx::query_as::<_, (String, String)>("SELECT sect_name, state FROM games WHERE id = $1")
            .bind(id.to_string())
            .fetch_optional(pool)
            .await?;

    match row {
        Some((sect_name, state_json)) => {
            let mut state: GameState = serde_json::from_str(&state_json).unwrap_or_default();
            crate::logic::sect::hydrate_player_sect(&mut state, &sect_name);
            Ok(Some((sect_name, state)))
        }
        None => Ok(None),
    }
}

pub async fn get_game_group(pool: &SqlitePool, id: Uuid) -> Result<Option<Uuid>, sqlx::Error> {
    let group_id: Option<String> =
        sqlx::query_scalar("SELECT save_group_id FROM games WHERE id = $1")
            .bind(id.to_string())
            .fetch_optional(pool)
            .await?;
    group_id
        .map(|id| Uuid::parse_str(&id).map_err(|error| sqlx::Error::Decode(error.into())))
        .transpose()
}

pub async fn list_games(pool: &SqlitePool) -> Result<Vec<GameListItem>, sqlx::Error> {
    let rows = sqlx::query_as::<
        _,
        (
            String,
            String,
            String,
            String,
            bool,
            i32,
            i32,
            chrono::DateTime<chrono::Utc>,
        ),
    >(
        "SELECT id, save_group_id, save_type, sect_name,
                COALESCE(CAST(json_extract(state, '$.autosave') AS INTEGER), 1) AS autosave,
                COALESCE(CAST(json_extract(state, '$.year') AS INTEGER), 1) AS year,
                COALESCE(CAST(json_extract(state, '$.month') AS INTEGER), 1) AS month,
                updated_at
         FROM games ORDER BY updated_at DESC",
    )
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(
            |(id, save_group_id, save_type, sect_name, autosave, year, month, updated_at)| {
                Ok(GameListItem {
                    id: Uuid::parse_str(&id).map_err(|error| sqlx::Error::Decode(error.into()))?,
                    save_group_id: Uuid::parse_str(&save_group_id)
                        .map_err(|error| sqlx::Error::Decode(error.into()))?,
                    save_type,
                    sect_name,
                    autosave,
                    year,
                    month,
                    updated_at,
                })
            },
        )
        .collect()
}

/// 在同一槽位内写入一个新的不可变时间点，并返回新存档 ID。
pub async fn create_save(
    pool: &SqlitePool,
    source_id: Uuid,
    sect_name: &str,
    state: &GameState,
    autosave: bool,
) -> Result<Option<Uuid>, sqlx::Error> {
    let Some(group_id) = get_game_group(pool, source_id).await? else {
        return Ok(None);
    };
    let mut saved_state = state.clone();
    saved_state.autosave = autosave;
    let state_json = serde_json::to_string(&saved_state).unwrap();
    let save_type = if autosave { "auto" } else { "manual" };
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO games (id, sect_name, state, schema_version, save_group_id, save_type)
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(id.to_string())
    .bind(sect_name)
    .bind(state_json)
    .bind(saved_state.schema_version)
    .bind(group_id.to_string())
    .bind(save_type)
    .execute(pool)
    .await?;
    Ok(Some(id))
}

pub async fn update_game(
    pool: &SqlitePool,
    id: Uuid,
    sect_name: &str,
    state: &GameState,
) -> Result<bool, sqlx::Error> {
    let state_json = serde_json::to_string(state).unwrap();
    let result = sqlx::query(
        "UPDATE games SET sect_name = $2, state = $3, updated_at = datetime('now') WHERE id = $1",
    )
    .bind(id.to_string())
    .bind(sect_name)
    .bind(&state_json)
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

pub async fn delete_game(pool: &SqlitePool, id: Uuid) -> Result<bool, sqlx::Error> {
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM events WHERE game_id = $1")
        .bind(id.to_string())
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM game_snapshots WHERE game_id = $1")
        .bind(id.to_string())
        .execute(&mut *tx)
        .await?;
    let result = sqlx::query("DELETE FROM games WHERE id = $1")
        .bind(id.to_string())
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(result.rows_affected() > 0)
}

pub async fn delete_save_group(pool: &SqlitePool, group_id: Uuid) -> Result<bool, sqlx::Error> {
    let mut tx = pool.begin().await?;
    let ids: Vec<String> = sqlx::query_scalar("SELECT id FROM games WHERE save_group_id = $1")
        .bind(group_id.to_string())
        .fetch_all(&mut *tx)
        .await?;
    if ids.is_empty() {
        return Ok(false);
    }
    for id in ids {
        sqlx::query("DELETE FROM events WHERE game_id = $1")
            .bind(&id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM game_snapshots WHERE game_id = $1")
            .bind(&id)
            .execute(&mut *tx)
            .await?;
    }
    sqlx::query("DELETE FROM games WHERE save_group_id = $1")
        .bind(group_id.to_string())
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(true)
}

pub async fn append_events(
    pool: &SqlitePool,
    game_id: Uuid,
    events: &[crate::models::GameEvent],
) -> Result<(), sqlx::Error> {
    for ev in events {
        sqlx::query(
            "INSERT INTO events (game_id, year, month, mood, text, category, payload)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(game_id.to_string())
        .bind(ev.year)
        .bind(ev.month)
        .bind(&ev.mood)
        .bind(&ev.text)
        .bind(&ev.category)
        .bind(
            serde_json::to_string(&serde_json::json!({
                "mood": ev.mood,
                "category": ev.category,
            }))
            .unwrap(),
        )
        .execute(pool)
        .await?;
    }
    Ok(())
}
