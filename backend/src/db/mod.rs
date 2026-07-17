use crate::models::game::{CreateGameRequest, GameListItem, GameState};
use sqlx::PgPool;
use uuid::Uuid;

/// 启动时自动建表（幂等）
pub async fn run_migrations(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS games (
            id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            sect_name   TEXT NOT NULL,
            state       JSONB NOT NULL,
            created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
            updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
        )",
    )
    .execute(pool)
    .await?;

    sqlx::query("ALTER TABLE games ADD COLUMN IF NOT EXISTS schema_version INT NOT NULL DEFAULT 3")
        .execute(pool)
        .await?;
    sqlx::query("ALTER TABLE games ADD COLUMN IF NOT EXISTS save_group_id UUID")
        .execute(pool)
        .await?;
    // 旧版每行即一局游戏，升级后各自成为独立槽位。
    sqlx::query("UPDATE games SET save_group_id = gen_random_uuid() WHERE save_group_id IS NULL")
        .execute(pool)
        .await?;
    sqlx::query("ALTER TABLE games ALTER COLUMN save_group_id SET DEFAULT gen_random_uuid()")
        .execute(pool)
        .await?;
    sqlx::query("ALTER TABLE games ALTER COLUMN save_group_id SET NOT NULL")
        .execute(pool)
        .await?;
    sqlx::query(
        "ALTER TABLE games ADD COLUMN IF NOT EXISTS save_type TEXT NOT NULL DEFAULT 'auto'",
    )
    .execute(pool)
    .await?;
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS game_snapshots (
            game_id UUID NOT NULL,
            turn INT NOT NULL,
            state JSONB NOT NULL,
            checksum TEXT NOT NULL DEFAULT '',
            created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
            PRIMARY KEY (game_id, turn)
        )",
    )
    .execute(pool)
    .await?;
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS events (
            id          BIGSERIAL PRIMARY KEY,
            game_id     UUID NOT NULL,
            year        INT NOT NULL,
            month       INT NOT NULL,
            mood        TEXT NOT NULL,
            text        TEXT NOT NULL,
            created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
        )",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "ALTER TABLE events ADD COLUMN IF NOT EXISTS category TEXT NOT NULL DEFAULT 'world'",
    )
    .execute(pool)
    .await?;
    sqlx::query(
        "ALTER TABLE events ADD COLUMN IF NOT EXISTS payload JSONB NOT NULL DEFAULT '{}'::jsonb",
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
    pool: &PgPool,
    req: &CreateGameRequest,
) -> Result<(Uuid, GameState), sqlx::Error> {
    let mut state = GameState::default();
    state.autosave = true;
    let state_json = serde_json::to_value(&state).unwrap();
    let save_group_id = Uuid::new_v4();

    let row = sqlx::query_as::<_, (Uuid,)>(
        "INSERT INTO games (sect_name, state, save_group_id, save_type)
         VALUES ($1, $2, $3, 'auto') RETURNING id",
    )
    .bind(&req.sect_name)
    .bind(&state_json)
    .bind(save_group_id)
    .fetch_one(pool)
    .await?;

    Ok((row.0, state))
}

pub async fn get_game(pool: &PgPool, id: Uuid) -> Result<Option<(String, GameState)>, sqlx::Error> {
    let row = sqlx::query_as::<_, (String, serde_json::Value)>(
        "SELECT sect_name, state FROM games WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    match row {
        Some((sect_name, state_json)) => {
            let mut state: GameState = serde_json::from_value(state_json).unwrap_or_default();
            crate::logic::sect::hydrate_player_sect(&mut state, &sect_name);
            Ok(Some((sect_name, state)))
        }
        None => Ok(None),
    }
}

pub async fn get_game_group(pool: &PgPool, id: Uuid) -> Result<Option<Uuid>, sqlx::Error> {
    sqlx::query_scalar("SELECT save_group_id FROM games WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
}

pub async fn list_games(pool: &PgPool) -> Result<Vec<GameListItem>, sqlx::Error> {
    sqlx::query_as::<_, GameListItem>(
        "SELECT id, save_group_id, save_type, sect_name,
                COALESCE((state->>'autosave')::bool, true) AS autosave,
                COALESCE((state->>'year')::int, 1) AS year,
                COALESCE((state->>'month')::int, 1) AS month,
                updated_at
         FROM games ORDER BY updated_at DESC",
    )
    .fetch_all(pool)
    .await
}

/// 在同一槽位内写入一个新的不可变时间点，并返回新存档 ID。
pub async fn create_save(
    pool: &PgPool,
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
    let state_json = serde_json::to_value(saved_state).unwrap();
    let save_type = if autosave { "auto" } else { "manual" };
    let id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO games (sect_name, state, save_group_id, save_type)
         VALUES ($1, $2, $3, $4) RETURNING id",
    )
    .bind(sect_name)
    .bind(state_json)
    .bind(group_id)
    .bind(save_type)
    .fetch_one(pool)
    .await?;
    Ok(Some(id))
}

pub async fn update_game(
    pool: &PgPool,
    id: Uuid,
    sect_name: &str,
    state: &GameState,
) -> Result<bool, sqlx::Error> {
    let state_json = serde_json::to_value(state).unwrap();
    let result = sqlx::query(
        "UPDATE games SET sect_name = $2, state = $3, updated_at = now() WHERE id = $1",
    )
    .bind(id)
    .bind(sect_name)
    .bind(&state_json)
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

pub async fn delete_game(pool: &PgPool, id: Uuid) -> Result<bool, sqlx::Error> {
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM events WHERE game_id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM game_snapshots WHERE game_id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    let result = sqlx::query("DELETE FROM games WHERE id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(result.rows_affected() > 0)
}

pub async fn delete_save_group(pool: &PgPool, group_id: Uuid) -> Result<bool, sqlx::Error> {
    let mut tx = pool.begin().await?;
    let ids: Vec<Uuid> =
        sqlx::query_scalar("SELECT id FROM games WHERE save_group_id = $1 FOR UPDATE")
            .bind(group_id)
            .fetch_all(&mut *tx)
            .await?;
    if ids.is_empty() {
        return Ok(false);
    }
    sqlx::query("DELETE FROM events WHERE game_id = ANY($1)")
        .bind(&ids)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM game_snapshots WHERE game_id = ANY($1)")
        .bind(&ids)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM games WHERE save_group_id = $1")
        .bind(group_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(true)
}

pub async fn append_events(
    pool: &PgPool,
    game_id: Uuid,
    events: &[crate::models::GameEvent],
) -> Result<(), sqlx::Error> {
    for ev in events {
        sqlx::query(
            "INSERT INTO events (game_id, year, month, mood, text, category, payload)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(game_id)
        .bind(ev.year)
        .bind(ev.month)
        .bind(&ev.mood)
        .bind(&ev.text)
        .bind(&ev.category)
        .bind(serde_json::json!({ "mood": ev.mood, "category": ev.category }))
        .execute(pool)
        .await?;
    }
    Ok(())
}
