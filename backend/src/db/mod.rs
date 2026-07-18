use crate::models::{
    game::{CreateGameRequest, GameListItem, GameState},
    GameEvent,
};
use serde::Serialize;
use sqlx::{Sqlite, SqlitePool, Transaction};
use uuid::Uuid;

const MAX_AUTO_SAVES: i64 = 10;

/// 带槽位修订号的存档记录。
///
/// `id` 是本次读取的时间点，`current_game_id` 是该槽位当前权威时间点。
/// 读取历史时间点时二者可以不同；写入以槽位级 `revision` 做 CAS。
#[derive(Debug, Clone)]
pub struct RevisionedGame {
    pub id: Uuid,
    pub save_group_id: Uuid,
    pub current_game_id: Uuid,
    pub revision: i64,
    pub sect_name: String,
    pub state: GameState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreatedSave {
    pub id: Uuid,
    pub save_group_id: Uuid,
    pub revision: i64,
}

/// 带乐观锁的数据库写入结果。
#[derive(Debug, Clone)]
pub enum RevisionWriteResult<T> {
    Applied(T),
    Conflict(RevisionedGame),
    NotFound,
}

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

    // 槽位 head 是跨存档节点的并发边界。不能只给 games 行加修订号，否则两个
    // 并发月结仍能从同一来源各自插入一个合法子档。
    let mut tx = pool.begin().await?;
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS save_group_heads (
            save_group_id TEXT PRIMARY KEY,
            current_game_id TEXT NOT NULL,
            revision INTEGER NOT NULL DEFAULT 1 CHECK (revision > 0),
            updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now'))
        )",
    )
    .execute(&mut *tx)
    .await?;
    // 修复被旧版本删除路径遗留的悬空 head；下一条查询会为该槽位重新选出
    // updated_at、rowid 均确定的最新时间点。
    sqlx::query(
        "DELETE FROM save_group_heads
         WHERE NOT EXISTS (
             SELECT 1 FROM games
             WHERE games.id = save_group_heads.current_game_id
               AND games.save_group_id = save_group_heads.save_group_id
         )",
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT OR IGNORE INTO save_group_heads
             (save_group_id, current_game_id, revision)
         SELECT game.save_group_id, game.id, 1
         FROM games AS game
         WHERE game.rowid = (
             SELECT candidate.rowid
             FROM games AS candidate
             WHERE candidate.save_group_id = game.save_group_id
             ORDER BY candidate.updated_at DESC, candidate.rowid DESC
             LIMIT 1
         )",
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_save_group_heads_updated
         ON save_group_heads (updated_at DESC)",
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

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
    state: &GameState,
) -> Result<(Uuid, Uuid), sqlx::Error> {
    let mut saved_state = state.clone();
    saved_state.autosave = true;
    let state_json = serialize_json(&saved_state)?;
    let id = Uuid::new_v4();
    let save_group_id = Uuid::new_v4();

    let mut tx = pool.begin().await?;
    sqlx::query(
        "INSERT INTO games (id, sect_name, state, schema_version, save_group_id, save_type)
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(id.to_string())
    .bind(&req.sect_name)
    .bind(&state_json)
    .bind(saved_state.schema_version)
    .bind(save_group_id.to_string())
    .bind("auto")
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO save_group_heads
             (save_group_id, current_game_id, revision)
         VALUES ($1, $2, 1)",
    )
    .bind(save_group_id.to_string())
    .bind(id.to_string())
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    Ok((id, save_group_id))
}

#[cfg(test)]
pub async fn get_game(
    pool: &SqlitePool,
    id: Uuid,
) -> Result<Option<(String, GameState)>, sqlx::Error> {
    Ok(get_game_with_revision(pool, id)
        .await?
        .map(|game| (game.sect_name, game.state)))
}

/// 读取任一存档节点，并同时返回其槽位当前 head 与修订号。
pub async fn get_game_with_revision(
    pool: &SqlitePool,
    id: Uuid,
) -> Result<Option<RevisionedGame>, sqlx::Error> {
    let row = sqlx::query_as::<_, (String, String, String, String, String, i64)>(
        "SELECT game.id, game.sect_name, game.state, game.save_group_id,
                head.current_game_id, head.revision
         FROM games AS game
         JOIN save_group_heads AS head
           ON head.save_group_id = game.save_group_id
         WHERE game.id = $1",
    )
    .bind(id.to_string())
    .fetch_optional(pool)
    .await?;

    row.map(decode_revisioned_game).transpose()
}

/// 读取一个槽位当前权威 head。
pub async fn get_current_game(
    pool: &SqlitePool,
    save_group_id: Uuid,
) -> Result<Option<RevisionedGame>, sqlx::Error> {
    let row = sqlx::query_as::<_, (String, String, String, String, String, i64)>(
        "SELECT game.id, game.sect_name, game.state, game.save_group_id,
                head.current_game_id, head.revision
         FROM save_group_heads AS head
         JOIN games AS game ON game.id = head.current_game_id
         WHERE head.save_group_id = $1
           AND game.save_group_id = head.save_group_id",
    )
    .bind(save_group_id.to_string())
    .fetch_optional(pool)
    .await?;

    row.map(decode_revisioned_game).transpose()
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
            String,
            i64,
            chrono::DateTime<chrono::Utc>,
        ),
    >(
        "SELECT game.id, game.save_group_id, game.save_type, game.sect_name,
                COALESCE(CAST(json_extract(game.state, '$.autosave') AS INTEGER), 1)
                    AS autosave,
                COALESCE(CAST(json_extract(game.state, '$.year') AS INTEGER), 1) AS year,
                COALESCE(CAST(json_extract(game.state, '$.month') AS INTEGER), 1) AS month,
                game.updated_at, head.current_game_id, head.revision, head.updated_at
         FROM save_group_heads AS head
         JOIN games AS current
           ON current.id = head.current_game_id
          AND current.save_group_id = head.save_group_id
         JOIN games AS game ON game.save_group_id = head.save_group_id
         ORDER BY head.updated_at DESC, head.save_group_id DESC,
                  CASE WHEN game.id = head.current_game_id THEN 0 ELSE 1 END,
                  game.updated_at DESC, game.rowid DESC",
    )
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(
            |(
                id,
                save_group_id,
                save_type,
                sect_name,
                autosave,
                year,
                month,
                updated_at,
                current_game_id,
                revision,
                head_updated_at,
            )| {
                Ok(GameListItem {
                    id: Uuid::parse_str(&id).map_err(|error| sqlx::Error::Decode(error.into()))?,
                    save_group_id: Uuid::parse_str(&save_group_id)
                        .map_err(|error| sqlx::Error::Decode(error.into()))?,
                    current_game_id: Uuid::parse_str(&current_game_id)
                        .map_err(|error| sqlx::Error::Decode(error.into()))?,
                    revision,
                    save_type,
                    sect_name,
                    autosave,
                    year,
                    month,
                    updated_at,
                    head_updated_at,
                })
            },
        )
        .collect()
}

/// 在同一槽位内写入一个新的不可变时间点，并返回新存档 ID。
#[cfg(test)]
pub async fn create_save(
    pool: &SqlitePool,
    source_id: Uuid,
    sect_name: &str,
    state: &GameState,
    autosave: bool,
) -> Result<Option<Uuid>, sqlx::Error> {
    create_save_with_events(pool, source_id, sect_name, state, autosave, &[]).await
}

/// 在同一事务中创建存档、附加事件，并清理超限自动档。
#[cfg(test)]
pub async fn create_save_with_events(
    pool: &SqlitePool,
    source_id: Uuid,
    sect_name: &str,
    state: &GameState,
    autosave: bool,
    events: &[GameEvent],
) -> Result<Option<Uuid>, sqlx::Error> {
    create_save_with_events_unchecked(pool, source_id, sect_name, state, autosave, events).await
}

/// 以槽位修订号比较并交换，创建一个新存档节点。
///
/// CAS 是事务的第一条写语句；只有成功认领该修订号的请求才会插入新节点、
/// 事件副本并执行自动档淘汰。
pub async fn create_save_with_events_if_revision(
    pool: &SqlitePool,
    source_id: Uuid,
    save_group_id: Uuid,
    expected_revision: i64,
    sect_name: &str,
    state: &GameState,
    autosave: bool,
    events: &[GameEvent],
) -> Result<RevisionWriteResult<CreatedSave>, sqlx::Error> {
    let mut saved_state = state.clone();
    saved_state.autosave = autosave;
    let state_json = serialize_json(&saved_state)?;
    let save_type = if autosave { "auto" } else { "manual" };
    let id = Uuid::new_v4();
    let mut tx = pool.begin().await?;

    let claimed =
        claim_group_revision(&mut tx, source_id, save_group_id, id, expected_revision).await?;
    if !claimed {
        tx.commit().await?;
        return classify_failed_cas(pool, source_id, save_group_id, expected_revision).await;
    }

    sqlx::query(
        "INSERT INTO games (id, sect_name, state, schema_version, save_group_id, save_type)
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(id.to_string())
    .bind(sect_name)
    .bind(state_json)
    .bind(saved_state.schema_version)
    .bind(save_group_id.to_string())
    .bind(save_type)
    .execute(&mut *tx)
    .await?;

    append_events_in_transaction(&mut tx, id, events).await?;

    if autosave {
        prune_auto_saves(&mut tx, save_group_id, id).await?;
    }

    tx.commit().await?;
    Ok(RevisionWriteResult::Applied(CreatedSave {
        id,
        save_group_id,
        revision: expected_revision + 1,
    }))
}

/// 旧 handler 的兼容入口。它保留“最后写入者覆盖”的旧语义，但仍在同一事务
/// 中维护槽位 head；handler 接入 revision 后应改用上面的 CAS API。
#[cfg(test)]
async fn create_save_with_events_unchecked(
    pool: &SqlitePool,
    source_id: Uuid,
    sect_name: &str,
    state: &GameState,
    autosave: bool,
    events: &[GameEvent],
) -> Result<Option<Uuid>, sqlx::Error> {
    let mut saved_state = state.clone();
    saved_state.autosave = autosave;
    let state_json = serialize_json(&saved_state)?;
    let save_type = if autosave { "auto" } else { "manual" };
    let id = Uuid::new_v4();
    let mut tx = pool.begin().await?;

    let group_id: Option<String> =
        sqlx::query_scalar("SELECT save_group_id FROM games WHERE id = $1")
            .bind(source_id.to_string())
            .fetch_optional(&mut *tx)
            .await?;
    let Some(group_id) = group_id else {
        tx.commit().await?;
        return Ok(None);
    };
    let save_group_id = parse_uuid(&group_id)?;

    sqlx::query(
        "INSERT INTO games (id, sect_name, state, schema_version, save_group_id, save_type)
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(id.to_string())
    .bind(sect_name)
    .bind(state_json)
    .bind(saved_state.schema_version)
    .bind(&group_id)
    .bind(save_type)
    .execute(&mut *tx)
    .await?;

    set_group_head_unchecked(&mut tx, save_group_id, id).await?;
    append_events_in_transaction(&mut tx, id, events).await?;

    if autosave {
        prune_auto_saves(&mut tx, save_group_id, id).await?;
    }

    tx.commit().await?;
    Ok(Some(id))
}

/// 以槽位修订号比较并交换，原地更新当前存档节点。
pub async fn update_game_with_events_if_revision(
    pool: &SqlitePool,
    id: Uuid,
    save_group_id: Uuid,
    expected_revision: i64,
    sect_name: &str,
    state: &GameState,
    events: &[GameEvent],
) -> Result<RevisionWriteResult<i64>, sqlx::Error> {
    let state_json = serialize_json(state)?;
    let mut tx = pool.begin().await?;

    let claimed = claim_group_revision(&mut tx, id, save_group_id, id, expected_revision).await?;
    if !claimed {
        tx.commit().await?;
        return classify_failed_cas(pool, id, save_group_id, expected_revision).await;
    }

    let result = sqlx::query(
        "UPDATE games
         SET sect_name = $2, state = $3, schema_version = $4,
             updated_at = strftime('%Y-%m-%d %H:%M:%f', 'now')
         WHERE id = $1",
    )
    .bind(id.to_string())
    .bind(sect_name)
    .bind(&state_json)
    .bind(state.schema_version)
    .execute(&mut *tx)
    .await?;

    if result.rows_affected() == 0 {
        // claim 的 EXISTS 与 UPDATE 位于同一写事务；正常情况下不会进入这里。
        tx.rollback().await?;
        return Ok(RevisionWriteResult::NotFound);
    }

    append_events_in_transaction(&mut tx, id, events).await?;
    tx.commit().await?;
    Ok(RevisionWriteResult::Applied(expected_revision + 1))
}

#[cfg(test)]
pub async fn delete_game(pool: &SqlitePool, id: Uuid) -> Result<bool, sqlx::Error> {
    let mut tx = pool.begin().await?;
    let id_string = id.to_string();
    // DELETE ... RETURNING 是事务中的第一条数据库语句，因此会直接取得写锁并
    // 遵守 busy_timeout；避免 BEGIN(DEFERRED) 先读后写时的锁升级 SQLITE_BUSY。
    let save_group_id: Option<String> =
        sqlx::query_scalar("DELETE FROM games WHERE id = $1 RETURNING save_group_id")
            .bind(&id_string)
            .fetch_optional(&mut *tx)
            .await?;
    let Some(save_group_id) = save_group_id else {
        tx.commit().await?;
        return Ok(false);
    };

    let current_game_id: Option<String> = sqlx::query_scalar(
        "SELECT current_game_id
         FROM save_group_heads
         WHERE save_group_id = $1",
    )
    .bind(&save_group_id)
    .fetch_optional(&mut *tx)
    .await?;
    delete_game_replicas_in_transaction(&mut tx, &id_string).await?;
    maintain_head_after_delete(
        &mut tx,
        &save_group_id,
        current_game_id
            .as_deref()
            .is_none_or(|current_id| current_id == id_string.as_str()),
    )
    .await?;
    tx.commit().await?;
    Ok(true)
}

/// 以槽位 revision 删除单个时间点。
///
/// head 条件更新是事务第一条写语句。若删除当前节点，事务在同一个 revision
/// 内重指向最近的剩余节点；组内已空则删除 head。
pub async fn delete_game_if_revision(
    pool: &SqlitePool,
    id: Uuid,
    save_group_id: Uuid,
    expected_revision: i64,
) -> Result<RevisionWriteResult<Option<RevisionedGame>>, sqlx::Error> {
    let mut tx = pool.begin().await?;
    let claimed =
        claim_group_revision_without_moving_head(&mut tx, id, save_group_id, expected_revision)
            .await?;
    if !claimed {
        tx.commit().await?;
        return classify_failed_cas(pool, id, save_group_id, expected_revision).await;
    }

    let current_game_id: String = sqlx::query_scalar(
        "SELECT current_game_id
         FROM save_group_heads
         WHERE save_group_id = $1",
    )
    .bind(save_group_id.to_string())
    .fetch_one(&mut *tx)
    .await?;
    let id_string = id.to_string();
    let result = sqlx::query("DELETE FROM games WHERE id = $1 AND save_group_id = $2")
        .bind(&id_string)
        .bind(save_group_id.to_string())
        .execute(&mut *tx)
        .await?;
    if result.rows_affected() != 1 {
        tx.rollback().await?;
        return Ok(RevisionWriteResult::NotFound);
    }
    delete_game_replicas_in_transaction(&mut tx, &id_string).await?;

    if current_game_id == id_string {
        let replacement: Option<String> = sqlx::query_scalar(
            "SELECT id
             FROM games
             WHERE save_group_id = $1
             ORDER BY updated_at DESC, rowid DESC
             LIMIT 1",
        )
        .bind(save_group_id.to_string())
        .fetch_optional(&mut *tx)
        .await?;
        if let Some(replacement) = replacement {
            sqlx::query(
                "UPDATE save_group_heads
                 SET current_game_id = $2
                 WHERE save_group_id = $1",
            )
            .bind(save_group_id.to_string())
            .bind(replacement)
            .execute(&mut *tx)
            .await?;
        } else {
            sqlx::query("DELETE FROM save_group_heads WHERE save_group_id = $1")
                .bind(save_group_id.to_string())
                .execute(&mut *tx)
                .await?;
        }
    }

    let current = get_current_game_in_transaction(&mut tx, save_group_id).await?;
    tx.commit().await?;
    Ok(RevisionWriteResult::Applied(current))
}

async fn delete_game_replicas_in_transaction(
    tx: &mut Transaction<'_, Sqlite>,
    id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM events WHERE game_id = $1")
        .bind(id)
        .execute(&mut **tx)
        .await?;
    sqlx::query("DELETE FROM game_snapshots WHERE game_id = $1")
        .bind(id)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

async fn delete_game_in_transaction(
    tx: &mut Transaction<'_, Sqlite>,
    id: &str,
) -> Result<u64, sqlx::Error> {
    delete_game_replicas_in_transaction(tx, id).await?;
    let result = sqlx::query("DELETE FROM games WHERE id = $1")
        .bind(id)
        .execute(&mut **tx)
        .await?;
    Ok(result.rows_affected())
}

#[cfg(test)]
pub async fn delete_save_group(pool: &SqlitePool, group_id: Uuid) -> Result<bool, sqlx::Error> {
    let mut tx = pool.begin().await?;
    // 与单档删除相同，首条语句直接进入写事务，避免并发 CAS 下的读锁升级失败。
    let ids: Vec<String> =
        sqlx::query_scalar("DELETE FROM games WHERE save_group_id = $1 RETURNING id")
            .bind(group_id.to_string())
            .fetch_all(&mut *tx)
            .await?;
    if ids.is_empty() {
        sqlx::query("DELETE FROM save_group_heads WHERE save_group_id = $1")
            .bind(group_id.to_string())
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        return Ok(false);
    }
    for id in ids {
        delete_game_replicas_in_transaction(&mut tx, &id).await?;
    }
    sqlx::query("DELETE FROM save_group_heads WHERE save_group_id = $1")
        .bind(group_id.to_string())
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(true)
}

/// 以槽位 revision 删除整个槽位。认领 revision 是事务第一条写语句；成功后
/// head 随槽位一并删除，因此 Applied 不再返回当前状态。
pub async fn delete_save_group_if_revision(
    pool: &SqlitePool,
    save_group_id: Uuid,
    expected_revision: i64,
) -> Result<RevisionWriteResult<()>, sqlx::Error> {
    let mut tx = pool.begin().await?;
    let claimed = claim_save_group_revision(&mut tx, save_group_id, expected_revision).await?;
    if !claimed {
        tx.commit().await?;
        return classify_failed_group_cas(pool, save_group_id, expected_revision).await;
    }

    let ids: Vec<String> =
        sqlx::query_scalar("DELETE FROM games WHERE save_group_id = $1 RETURNING id")
            .bind(save_group_id.to_string())
            .fetch_all(&mut *tx)
            .await?;
    for id in ids {
        delete_game_replicas_in_transaction(&mut tx, &id).await?;
    }
    sqlx::query("DELETE FROM save_group_heads WHERE save_group_id = $1")
        .bind(save_group_id.to_string())
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(RevisionWriteResult::Applied(()))
}

async fn claim_group_revision(
    tx: &mut Transaction<'_, Sqlite>,
    source_id: Uuid,
    save_group_id: Uuid,
    target_id: Uuid,
    expected_revision: i64,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE save_group_heads
         SET current_game_id = $3,
             revision = revision + 1,
             updated_at = strftime('%Y-%m-%d %H:%M:%f', 'now')
         WHERE save_group_id = $1
           AND revision = $2
           AND EXISTS (
               SELECT 1 FROM games
               WHERE games.id = $4
                 AND games.save_group_id = save_group_heads.save_group_id
           )",
    )
    .bind(save_group_id.to_string())
    .bind(expected_revision)
    .bind(target_id.to_string())
    .bind(source_id.to_string())
    .execute(&mut **tx)
    .await?;
    Ok(result.rows_affected() == 1)
}

async fn claim_group_revision_without_moving_head(
    tx: &mut Transaction<'_, Sqlite>,
    source_id: Uuid,
    save_group_id: Uuid,
    expected_revision: i64,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE save_group_heads
         SET revision = revision + 1,
             updated_at = strftime('%Y-%m-%d %H:%M:%f', 'now')
         WHERE save_group_id = $1
           AND revision = $2
           AND EXISTS (
               SELECT 1 FROM games
               WHERE games.id = $3
                 AND games.save_group_id = save_group_heads.save_group_id
           )",
    )
    .bind(save_group_id.to_string())
    .bind(expected_revision)
    .bind(source_id.to_string())
    .execute(&mut **tx)
    .await?;
    Ok(result.rows_affected() == 1)
}

async fn claim_save_group_revision(
    tx: &mut Transaction<'_, Sqlite>,
    save_group_id: Uuid,
    expected_revision: i64,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE save_group_heads
         SET revision = revision + 1,
             updated_at = strftime('%Y-%m-%d %H:%M:%f', 'now')
         WHERE save_group_id = $1
           AND revision = $2",
    )
    .bind(save_group_id.to_string())
    .bind(expected_revision)
    .execute(&mut **tx)
    .await?;
    Ok(result.rows_affected() == 1)
}

async fn classify_failed_cas<T>(
    pool: &SqlitePool,
    source_id: Uuid,
    save_group_id: Uuid,
    expected_revision: i64,
) -> Result<RevisionWriteResult<T>, sqlx::Error> {
    if let Some(current) = get_current_game(pool, save_group_id).await? {
        // 来源可能刚被赢家的自动档淘汰；只要槽位 revision 已经前进，就应返回
        // Conflict 及赢家状态，而不是把同一场竞态误报成 NotFound。
        if current.revision != expected_revision {
            return Ok(RevisionWriteResult::Conflict(current));
        }
    }

    let source_group: Option<String> =
        sqlx::query_scalar("SELECT save_group_id FROM games WHERE id = $1")
            .bind(source_id.to_string())
            .fetch_optional(pool)
            .await?;
    let source_matches = source_group
        .as_deref()
        .map(parse_uuid)
        .transpose()?
        .is_some_and(|group_id| group_id == save_group_id);
    if !source_matches {
        return Ok(RevisionWriteResult::NotFound);
    }

    match get_current_game(pool, save_group_id).await? {
        Some(current) => Ok(RevisionWriteResult::Conflict(current)),
        None => Ok(RevisionWriteResult::NotFound),
    }
}

async fn classify_failed_group_cas<T>(
    pool: &SqlitePool,
    save_group_id: Uuid,
    _expected_revision: i64,
) -> Result<RevisionWriteResult<T>, sqlx::Error> {
    match get_current_game(pool, save_group_id).await? {
        Some(current) => Ok(RevisionWriteResult::Conflict(current)),
        None => Ok(RevisionWriteResult::NotFound),
    }
}

async fn get_current_game_in_transaction(
    tx: &mut Transaction<'_, Sqlite>,
    save_group_id: Uuid,
) -> Result<Option<RevisionedGame>, sqlx::Error> {
    let row = sqlx::query_as::<_, (String, String, String, String, String, i64)>(
        "SELECT game.id, game.sect_name, game.state, game.save_group_id,
                head.current_game_id, head.revision
         FROM save_group_heads AS head
         JOIN games AS game ON game.id = head.current_game_id
         WHERE head.save_group_id = $1
           AND game.save_group_id = head.save_group_id",
    )
    .bind(save_group_id.to_string())
    .fetch_optional(&mut **tx)
    .await?;
    row.map(decode_revisioned_game).transpose()
}

#[cfg(test)]
async fn set_group_head_unchecked(
    tx: &mut Transaction<'_, Sqlite>,
    save_group_id: Uuid,
    target_id: Uuid,
) -> Result<(), sqlx::Error> {
    let result = sqlx::query(
        "UPDATE save_group_heads
         SET current_game_id = $2,
             revision = revision + 1,
             updated_at = strftime('%Y-%m-%d %H:%M:%f', 'now')
         WHERE save_group_id = $1",
    )
    .bind(save_group_id.to_string())
    .bind(target_id.to_string())
    .execute(&mut **tx)
    .await?;
    if result.rows_affected() == 0 {
        sqlx::query(
            "INSERT INTO save_group_heads
                 (save_group_id, current_game_id, revision)
             VALUES ($1, $2, 1)",
        )
        .bind(save_group_id.to_string())
        .bind(target_id.to_string())
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

async fn prune_auto_saves(
    tx: &mut Transaction<'_, Sqlite>,
    save_group_id: Uuid,
    current_game_id: Uuid,
) -> Result<(), sqlx::Error> {
    let stale_ids: Vec<String> = sqlx::query_scalar(
        "SELECT id FROM games
         WHERE save_group_id = $1 AND save_type = 'auto'
         ORDER BY CASE WHEN id = $2 THEN 0 ELSE 1 END,
                  updated_at DESC, rowid DESC
         LIMIT -1 OFFSET $3",
    )
    .bind(save_group_id.to_string())
    .bind(current_game_id.to_string())
    .bind(MAX_AUTO_SAVES)
    .fetch_all(&mut **tx)
    .await?;

    for stale_id in stale_ids {
        // 原始清理函数不会再修改 head；本次新存档事务已经只递增过一次 revision。
        delete_game_in_transaction(tx, &stale_id).await?;
    }
    Ok(())
}

#[cfg(test)]
async fn maintain_head_after_delete(
    tx: &mut Transaction<'_, Sqlite>,
    save_group_id: &str,
    deleted_current: bool,
) -> Result<(), sqlx::Error> {
    if deleted_current {
        let replacement: Option<String> = sqlx::query_scalar(
            "SELECT id FROM games
             WHERE save_group_id = $1
             ORDER BY updated_at DESC, rowid DESC
             LIMIT 1",
        )
        .bind(save_group_id)
        .fetch_optional(&mut **tx)
        .await?;
        if let Some(replacement) = replacement {
            let result = sqlx::query(
                "UPDATE save_group_heads
                 SET current_game_id = $2,
                     revision = revision + 1,
                     updated_at = strftime('%Y-%m-%d %H:%M:%f', 'now')
                 WHERE save_group_id = $1",
            )
            .bind(save_group_id)
            .bind(&replacement)
            .execute(&mut **tx)
            .await?;
            if result.rows_affected() == 0 {
                sqlx::query(
                    "INSERT INTO save_group_heads
                         (save_group_id, current_game_id, revision)
                     VALUES ($1, $2, 1)",
                )
                .bind(save_group_id)
                .bind(replacement)
                .execute(&mut **tx)
                .await?;
            }
        } else {
            sqlx::query("DELETE FROM save_group_heads WHERE save_group_id = $1")
                .bind(save_group_id)
                .execute(&mut **tx)
                .await?;
        }
    } else {
        // 删除历史节点也会改变槽位卷宗，因此令已加载该组的写请求失效。
        sqlx::query(
            "UPDATE save_group_heads
             SET revision = revision + 1,
                 updated_at = strftime('%Y-%m-%d %H:%M:%f', 'now')
             WHERE save_group_id = $1",
        )
        .bind(save_group_id)
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

async fn append_events_in_transaction(
    tx: &mut Transaction<'_, Sqlite>,
    game_id: Uuid,
    events: &[GameEvent],
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
            serde_json::json!({
                "mood": ev.mood,
                "category": ev.category,
            })
            .to_string(),
        )
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

fn decode_revisioned_game(
    (id, sect_name, state_json, save_group_id, current_game_id, revision): (
        String,
        String,
        String,
        String,
        String,
        i64,
    ),
) -> Result<RevisionedGame, sqlx::Error> {
    // 缺字段由 serde(default) 兼容；语法损坏或类型不兼容必须显式报错，不能静默
    // 生成默认局并在下一次 CAS 时覆盖原存档。
    let mut state: GameState =
        serde_json::from_str(&state_json).map_err(|error| sqlx::Error::Decode(error.into()))?;
    crate::logic::sect::hydrate_player_sect(&mut state, &sect_name);
    Ok(RevisionedGame {
        id: parse_uuid(&id)?,
        save_group_id: parse_uuid(&save_group_id)?,
        current_game_id: parse_uuid(&current_game_id)?,
        revision,
        sect_name,
        state,
    })
}

fn parse_uuid(value: &str) -> Result<Uuid, sqlx::Error> {
    Uuid::parse_str(value).map_err(|error| sqlx::Error::Decode(error.into()))
}

fn serialize_json(value: &impl Serialize) -> Result<String, sqlx::Error> {
    serde_json::to_string(value).map_err(|error| sqlx::Error::Encode(error.into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn test_pool() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        run_migrations(&pool).await.unwrap();
        pool
    }

    async fn seed_game(pool: &SqlitePool) -> (Uuid, Uuid, GameState) {
        let mut state = GameState::default();
        state.autosave = true;
        let request = CreateGameRequest {
            sect_name: "回滚门".into(),
        };
        let (id, group_id) = create_game(pool, &request, &state).await.unwrap();
        (id, group_id, state)
    }

    fn test_event() -> GameEvent {
        GameEvent {
            text: "事务试剑".into(),
            mood: "neutral".into(),
            year: 1,
            month: 1,
            category: "sect".into(),
        }
    }

    async fn head(pool: &SqlitePool, group_id: Uuid) -> (Uuid, i64) {
        let (id, revision): (String, i64) = sqlx::query_as(
            "SELECT current_game_id, revision
             FROM save_group_heads WHERE save_group_id = $1",
        )
        .bind(group_id.to_string())
        .fetch_one(pool)
        .await
        .unwrap();
        (Uuid::parse_str(&id).unwrap(), revision)
    }

    #[tokio::test]
    async fn migration_backfills_repairs_and_preserves_existing_head_revision() {
        let pool = test_pool().await;
        sqlx::query("DROP TABLE save_group_heads")
            .execute(&pool)
            .await
            .unwrap();

        let group_id = Uuid::new_v4();
        let older_id = Uuid::new_v4();
        let newer_id = Uuid::new_v4();
        let state_json = serialize_json(&GameState::default()).unwrap();
        for id in [older_id, newer_id] {
            sqlx::query(
                "INSERT INTO games
                     (id, sect_name, state, schema_version, save_group_id, save_type,
                      created_at, updated_at)
                 VALUES ($1, '旧门', $2, 3, $3, 'auto',
                         '2026-01-01 00:00:00', '2026-01-01 00:00:00')",
            )
            .bind(id.to_string())
            .bind(&state_json)
            .bind(group_id.to_string())
            .execute(&pool)
            .await
            .unwrap();
        }

        run_migrations(&pool).await.unwrap();
        assert_eq!(head(&pool, group_id).await, (newer_id, 1));

        sqlx::query(
            "UPDATE save_group_heads
             SET revision = 7 WHERE save_group_id = $1",
        )
        .bind(group_id.to_string())
        .execute(&pool)
        .await
        .unwrap();
        run_migrations(&pool).await.unwrap();
        assert_eq!(head(&pool, group_id).await, (newer_id, 7));

        sqlx::query(
            "UPDATE save_group_heads
             SET current_game_id = 'dangling' WHERE save_group_id = $1",
        )
        .bind(group_id.to_string())
        .execute(&pool)
        .await
        .unwrap();
        run_migrations(&pool).await.unwrap();
        assert_eq!(head(&pool, group_id).await, (newer_id, 1));
    }

    #[tokio::test]
    async fn create_and_read_expose_group_head_at_revision_one() {
        let pool = test_pool().await;
        let (id, group_id, _) = seed_game(&pool).await;

        let loaded = get_game_with_revision(&pool, id).await.unwrap().unwrap();
        assert_eq!(loaded.id, id);
        assert_eq!(loaded.save_group_id, group_id);
        assert_eq!(loaded.current_game_id, id);
        assert_eq!(loaded.revision, 1);

        let current = get_current_game(&pool, group_id).await.unwrap().unwrap();
        assert_eq!(current.id, id);
        assert_eq!(current.revision, 1);
    }

    #[tokio::test]
    async fn loaded_history_can_claim_current_group_revision_as_a_new_branch() {
        let pool = test_pool().await;
        let (source_id, group_id, mut state) = seed_game(&pool).await;
        let manual_id = create_save(&pool, source_id, "回滚门", &state, false)
            .await
            .unwrap()
            .unwrap();

        let history = get_game_with_revision(&pool, source_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(history.current_game_id, manual_id);
        assert_eq!(history.revision, 2);
        state.month = 2;

        let result = update_game_with_events_if_revision(
            &pool,
            source_id,
            group_id,
            history.revision,
            "回滚门",
            &state,
            &[],
        )
        .await
        .unwrap();
        assert!(matches!(result, RevisionWriteResult::Applied(3)));
        assert_eq!(head(&pool, group_id).await, (source_id, 3));
    }

    #[tokio::test]
    async fn stale_in_place_update_returns_current_without_writing_loser_events() {
        let pool = test_pool().await;
        let (id, group_id, state) = seed_game(&pool).await;
        let mut winner_state = state.clone();
        winner_state.month = 2;
        let mut loser_state = state;
        loser_state.month = 3;

        let winner = update_game_with_events_if_revision(
            &pool,
            id,
            group_id,
            1,
            "回滚门",
            &winner_state,
            &[test_event()],
        )
        .await
        .unwrap();
        assert!(matches!(winner, RevisionWriteResult::Applied(2)));

        let loser = update_game_with_events_if_revision(
            &pool,
            id,
            group_id,
            1,
            "回滚门",
            &loser_state,
            &[GameEvent {
                text: "不应落库".into(),
                ..test_event()
            }],
        )
        .await
        .unwrap();
        let RevisionWriteResult::Conflict(current) = loser else {
            panic!("陈旧写入应返回权威状态");
        };
        assert_eq!(current.id, id);
        assert_eq!(current.revision, 2);
        assert_eq!(current.state.month, 2);

        let event_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM events")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(event_count, 1);
    }

    #[tokio::test]
    async fn concurrent_auto_saves_from_one_revision_cannot_fork() {
        let pool = test_pool().await;
        let (source_id, group_id, state) = seed_game(&pool).await;
        let mut first_state = state.clone();
        first_state.month = 2;
        let mut second_state = state;
        second_state.month = 3;
        let first_events = [test_event()];
        let second_events = [GameEvent {
            text: "另一支月令".into(),
            ..test_event()
        }];

        let first = create_save_with_events_if_revision(
            &pool,
            source_id,
            group_id,
            1,
            "回滚门",
            &first_state,
            true,
            &first_events,
        );
        let second = create_save_with_events_if_revision(
            &pool,
            source_id,
            group_id,
            1,
            "回滚门",
            &second_state,
            true,
            &second_events,
        );
        let (first, second) = tokio::join!(first, second);
        let results = [first.unwrap(), second.unwrap()];

        let applied = results
            .iter()
            .find_map(|result| match result {
                RevisionWriteResult::Applied(save) => Some(*save),
                _ => None,
            })
            .expect("必须恰有一个赢家");
        let conflict = results
            .iter()
            .find_map(|result| match result {
                RevisionWriteResult::Conflict(game) => Some(game),
                _ => None,
            })
            .expect("必须恰有一个冲突者");
        assert_eq!(applied.revision, 2);
        assert_eq!(applied.save_group_id, group_id);
        assert_eq!(conflict.id, applied.id);
        assert_eq!(conflict.current_game_id, applied.id);
        assert_eq!(conflict.revision, 2);

        let game_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM games WHERE save_group_id = $1")
                .bind(group_id.to_string())
                .fetch_one(&pool)
                .await
                .unwrap();
        let event_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM events")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!((game_count, event_count), (2, 1));
        assert_eq!(head(&pool, group_id).await, (applied.id, 2));
    }

    #[tokio::test]
    async fn concurrent_auto_saves_on_distinct_file_connections_return_applied_and_conflict() {
        let path = std::env::temp_dir().join(format!(
            "zhangmenriji_revision_concurrency_{}.db",
            Uuid::new_v4()
        ));
        let url = format!("sqlite://{}?mode=rwc", path.display());
        let pool = SqlitePoolOptions::new()
            .min_connections(2)
            .max_connections(4)
            .connect(&url)
            .await
            .unwrap();
        run_migrations(&pool).await.unwrap();
        let (source_id, group_id, state) = seed_game(&pool).await;
        let mut first_state = state.clone();
        first_state.month = 2;
        let mut second_state = state;
        second_state.month = 3;
        let first_events = [test_event()];
        let second_events = [GameEvent {
            text: "另一连接的月令".into(),
            ..test_event()
        }];

        let outcome = tokio::time::timeout(std::time::Duration::from_secs(10), async {
            tokio::join!(
                create_save_with_events_if_revision(
                    &pool,
                    source_id,
                    group_id,
                    1,
                    "多连接门",
                    &first_state,
                    true,
                    &first_events,
                ),
                create_save_with_events_if_revision(
                    &pool,
                    source_id,
                    group_id,
                    1,
                    "多连接门",
                    &second_state,
                    true,
                    &second_events,
                ),
            )
        })
        .await
        .expect("多连接 CAS 不应因 SQLite 锁等待而挂起");
        let results = [outcome.0.unwrap(), outcome.1.unwrap()];

        assert_eq!(
            results
                .iter()
                .filter(|result| matches!(result, RevisionWriteResult::Applied(_)))
                .count(),
            1
        );
        assert_eq!(
            results
                .iter()
                .filter(|result| matches!(result, RevisionWriteResult::Conflict(_)))
                .count(),
            1
        );
        assert_eq!(head(&pool, group_id).await.1, 2);
        let game_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM games WHERE save_group_id = $1")
                .bind(group_id.to_string())
                .fetch_one(&pool)
                .await
                .unwrap();
        let event_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM events")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!((game_count, event_count), (2, 1));

        pool.close().await;
        std::fs::remove_file(path).unwrap();
    }

    #[tokio::test]
    async fn new_save_and_in_place_update_compete_for_one_group_revision() {
        let pool = test_pool().await;
        let (source_id, group_id, state) = seed_game(&pool).await;
        let mut advanced_state = state.clone();
        advanced_state.month = 2;
        let mut managed_state = state;
        managed_state.silver += 50;
        let create_events = [test_event()];
        let update_events = [GameEvent {
            text: "并发掌门令".into(),
            ..test_event()
        }];

        let create = create_save_with_events_if_revision(
            &pool,
            source_id,
            group_id,
            1,
            "回滚门",
            &advanced_state,
            true,
            &create_events,
        );
        let update = update_game_with_events_if_revision(
            &pool,
            source_id,
            group_id,
            1,
            "回滚门",
            &managed_state,
            &update_events,
        );
        let (create, update) = tokio::join!(create, update);
        let create = create.unwrap();
        let update = update.unwrap();

        let create_applied = matches!(create, RevisionWriteResult::Applied(_));
        let update_applied = matches!(update, RevisionWriteResult::Applied(_));
        assert_ne!(create_applied, update_applied);
        assert!(matches!(
            (&create, &update),
            (
                RevisionWriteResult::Applied(_),
                RevisionWriteResult::Conflict(_)
            ) | (
                RevisionWriteResult::Conflict(_),
                RevisionWriteResult::Applied(_)
            )
        ));

        let current = get_current_game(&pool, group_id).await.unwrap().unwrap();
        assert_eq!(current.revision, 2);
        let event_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM events")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(event_count, 1);
    }

    #[tokio::test]
    async fn missing_source_is_not_a_revision_conflict() {
        let pool = test_pool().await;
        let (_, group_id, state) = seed_game(&pool).await;

        let result = update_game_with_events_if_revision(
            &pool,
            Uuid::new_v4(),
            group_id,
            1,
            "回滚门",
            &state,
            &[],
        )
        .await
        .unwrap();

        assert!(matches!(result, RevisionWriteResult::NotFound));
        assert_eq!(head(&pool, group_id).await.1, 1);
    }

    #[tokio::test]
    async fn event_insert_failure_rolls_back_state_update() {
        let pool = test_pool().await;
        let (id, group_id, original_state) = seed_game(&pool).await;
        sqlx::query(
            "CREATE TRIGGER reject_event
             BEFORE INSERT ON events
             BEGIN
                 SELECT RAISE(FAIL, 'event insert rejected');
             END",
        )
        .execute(&pool)
        .await
        .unwrap();

        let mut updated_state = original_state.clone();
        updated_state.silver += 777;
        let result = update_game_with_events_if_revision(
            &pool,
            id,
            group_id,
            1,
            "回滚门",
            &updated_state,
            &[test_event()],
        )
        .await;

        assert!(result.is_err());
        let state_json: String = sqlx::query_scalar("SELECT state FROM games WHERE id = $1")
            .bind(id.to_string())
            .fetch_one(&pool)
            .await
            .unwrap();
        let persisted_state: GameState = serde_json::from_str(&state_json).unwrap();
        assert_eq!(persisted_state.silver, original_state.silver);
        let event_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM events")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(event_count, 0);
        assert_eq!(head(&pool, group_id).await, (id, 1));
    }

    #[tokio::test]
    async fn event_insert_failure_rolls_back_new_auto_save() {
        let pool = test_pool().await;
        let (source_id, group_id, mut state) = seed_game(&pool).await;
        state.month = 2;
        sqlx::query(
            "CREATE TRIGGER reject_event
             BEFORE INSERT ON events
             BEGIN
                 SELECT RAISE(FAIL, 'event insert rejected');
             END",
        )
        .execute(&pool)
        .await
        .unwrap();

        let result = create_save_with_events_if_revision(
            &pool,
            source_id,
            group_id,
            1,
            "回滚门",
            &state,
            true,
            &[test_event()],
        )
        .await;

        assert!(result.is_err());
        let save_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM games WHERE save_group_id = $1")
                .bind(group_id.to_string())
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(save_count, 1);
        assert_eq!(head(&pool, group_id).await, (source_id, 1));
    }

    #[tokio::test]
    async fn missing_source_does_not_create_a_save_or_events() {
        let pool = test_pool().await;
        let result = create_save_with_events(
            &pool,
            Uuid::new_v4(),
            "无门",
            &GameState::default(),
            true,
            &[test_event()],
        )
        .await
        .unwrap();

        assert_eq!(result, None);
        let game_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM games")
            .fetch_one(&pool)
            .await
            .unwrap();
        let event_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM events")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!((game_count, event_count), (0, 0));
    }

    #[tokio::test]
    async fn auto_save_limit_removes_the_whole_stale_replica() {
        let pool = test_pool().await;
        let (source_id, group_id, mut state) = seed_game(&pool).await;
        let mut tx = pool.begin().await.unwrap();
        append_events_in_transaction(&mut tx, source_id, &[test_event()])
            .await
            .unwrap();
        tx.commit().await.unwrap();
        sqlx::query(
            "INSERT INTO game_snapshots (game_id, turn, state, checksum)
             VALUES ($1, 1, '{}', 'test')",
        )
        .bind(source_id.to_string())
        .execute(&pool)
        .await
        .unwrap();

        let mut latest_id = source_id;
        for month in 0..MAX_AUTO_SAVES {
            state.month = i32::try_from(month + 2).unwrap();
            latest_id = create_save(&pool, latest_id, "回滚门", &state, true)
                .await
                .unwrap()
                .unwrap();
        }

        let save_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM games WHERE save_group_id = $1")
                .bind(group_id.to_string())
                .fetch_one(&pool)
                .await
                .unwrap();
        let source_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM games WHERE id = $1")
            .bind(source_id.to_string())
            .fetch_one(&pool)
            .await
            .unwrap();
        let source_event_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM events WHERE game_id = $1")
                .bind(source_id.to_string())
                .fetch_one(&pool)
                .await
                .unwrap();
        let source_snapshot_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM game_snapshots WHERE game_id = $1")
                .bind(source_id.to_string())
                .fetch_one(&pool)
                .await
                .unwrap();

        assert_eq!(save_count, MAX_AUTO_SAVES);
        assert_eq!(
            (source_count, source_event_count, source_snapshot_count),
            (0, 0, 0)
        );
        assert_eq!(head(&pool, group_id).await.0, latest_id);
    }

    #[tokio::test]
    async fn deleting_current_save_repoints_then_removes_group_head() {
        let pool = test_pool().await;
        let (source_id, group_id, state) = seed_game(&pool).await;
        let manual_id = create_save(&pool, source_id, "回滚门", &state, false)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(head(&pool, group_id).await, (manual_id, 2));

        assert!(delete_game(&pool, manual_id).await.unwrap());
        assert_eq!(head(&pool, group_id).await, (source_id, 3));

        assert!(delete_game(&pool, source_id).await.unwrap());
        assert!(get_current_game(&pool, group_id).await.unwrap().is_none());
        let head_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM save_group_heads WHERE save_group_id = $1")
                .bind(group_id.to_string())
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(head_count, 0);
    }

    #[tokio::test]
    async fn revisioned_delete_of_history_keeps_current_and_increments_once() {
        let pool = test_pool().await;
        let (source_id, group_id, state) = seed_game(&pool).await;
        let manual_id = create_save(&pool, source_id, "回滚门", &state, false)
            .await
            .unwrap()
            .unwrap();

        let result = delete_game_if_revision(&pool, source_id, group_id, 2)
            .await
            .unwrap();
        let RevisionWriteResult::Applied(Some(current)) = result else {
            panic!("删除历史节点后应返回当前权威节点");
        };
        assert_eq!(current.id, manual_id);
        assert_eq!(current.current_game_id, manual_id);
        assert_eq!(current.revision, 3);
        assert!(get_game_with_revision(&pool, source_id)
            .await
            .unwrap()
            .is_none());
        assert_eq!(head(&pool, group_id).await, (manual_id, 3));
    }

    #[tokio::test]
    async fn revisioned_delete_of_head_repoints_without_a_second_increment() {
        let pool = test_pool().await;
        let (source_id, group_id, state) = seed_game(&pool).await;
        let manual_id = create_save(&pool, source_id, "回滚门", &state, false)
            .await
            .unwrap()
            .unwrap();

        let result = delete_game_if_revision(&pool, manual_id, group_id, 2)
            .await
            .unwrap();
        let RevisionWriteResult::Applied(Some(current)) = result else {
            panic!("删除 head 后应返回替代节点");
        };
        assert_eq!(current.id, source_id);
        assert_eq!(current.current_game_id, source_id);
        assert_eq!(current.revision, 3);
        assert_eq!(head(&pool, group_id).await, (source_id, 3));
    }

    #[tokio::test]
    async fn stale_revisioned_delete_returns_current_without_deleting_target() {
        let pool = test_pool().await;
        let (source_id, group_id, state) = seed_game(&pool).await;
        let manual_id = create_save(&pool, source_id, "回滚门", &state, false)
            .await
            .unwrap()
            .unwrap();

        let result = delete_game_if_revision(&pool, source_id, group_id, 1)
            .await
            .unwrap();
        let RevisionWriteResult::Conflict(current) = result else {
            panic!("陈旧删除必须返回槽位当前状态");
        };
        assert_eq!(current.id, manual_id);
        assert_eq!(current.revision, 2);
        assert!(get_game_with_revision(&pool, source_id)
            .await
            .unwrap()
            .is_some());
        assert_eq!(head(&pool, group_id).await, (manual_id, 2));
    }

    #[tokio::test]
    async fn revisioned_delete_of_last_save_removes_the_group_head() {
        let pool = test_pool().await;
        let (id, group_id, _) = seed_game(&pool).await;

        let result = delete_game_if_revision(&pool, id, group_id, 1)
            .await
            .unwrap();
        assert!(matches!(result, RevisionWriteResult::Applied(None)));
        assert!(get_current_game(&pool, group_id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn revisioned_group_delete_rejects_stale_then_removes_the_slot() {
        let pool = test_pool().await;
        let (source_id, group_id, state) = seed_game(&pool).await;
        let manual_id = create_save(&pool, source_id, "回滚门", &state, false)
            .await
            .unwrap()
            .unwrap();

        let stale = delete_save_group_if_revision(&pool, group_id, 1)
            .await
            .unwrap();
        let RevisionWriteResult::Conflict(current) = stale else {
            panic!("陈旧槽位删除必须冲突");
        };
        assert_eq!(current.id, manual_id);
        assert_eq!(current.revision, 2);

        let applied = delete_save_group_if_revision(&pool, group_id, 2)
            .await
            .unwrap();
        assert!(matches!(applied, RevisionWriteResult::Applied(())));
        let game_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM games WHERE save_group_id = $1")
                .bind(group_id.to_string())
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(game_count, 0);
        assert!(get_current_game(&pool, group_id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn file_connections_make_save_and_revisioned_delete_choose_one_winner() {
        let path = std::env::temp_dir().join(format!(
            "zhangmenriji_delete_concurrency_{}.db",
            Uuid::new_v4()
        ));
        let url = format!("sqlite://{}?mode=rwc", path.display());
        let pool = SqlitePoolOptions::new()
            .min_connections(2)
            .max_connections(4)
            .connect(&url)
            .await
            .unwrap();
        run_migrations(&pool).await.unwrap();
        let (source_id, group_id, state) = seed_game(&pool).await;
        let manual_id = create_save(&pool, source_id, "回滚门", &state, false)
            .await
            .unwrap()
            .unwrap();
        let mut next_state = state;
        next_state.month = 2;
        let create_events = [test_event()];

        let outcome = tokio::time::timeout(std::time::Duration::from_secs(10), async {
            tokio::join!(
                create_save_with_events_if_revision(
                    &pool,
                    manual_id,
                    group_id,
                    2,
                    "回滚门",
                    &next_state,
                    true,
                    &create_events,
                ),
                delete_game_if_revision(&pool, source_id, group_id, 2),
            )
        })
        .await
        .expect("存档与删除竞态不应因 SQLite 锁等待而挂起");
        let create = outcome.0.unwrap();
        let delete = outcome.1.unwrap();
        let create_applied = matches!(create, RevisionWriteResult::Applied(_));
        let delete_applied = matches!(delete, RevisionWriteResult::Applied(_));
        assert_ne!(create_applied, delete_applied);
        if create_applied {
            assert!(matches!(delete, RevisionWriteResult::Conflict(_)));
        } else {
            assert!(matches!(create, RevisionWriteResult::Conflict(_)));
        }
        assert_eq!(head(&pool, group_id).await.1, 3);

        pool.close().await;
        std::fs::remove_file(path).unwrap();
    }

    #[tokio::test]
    async fn deleting_history_invalidates_revision_without_moving_head() {
        let pool = test_pool().await;
        let (source_id, group_id, state) = seed_game(&pool).await;
        let manual_id = create_save(&pool, source_id, "回滚门", &state, false)
            .await
            .unwrap()
            .unwrap();

        assert!(delete_game(&pool, source_id).await.unwrap());
        assert_eq!(head(&pool, group_id).await, (manual_id, 3));
    }

    #[tokio::test]
    async fn deleting_save_group_also_deletes_its_head() {
        let pool = test_pool().await;
        let (_, group_id, _) = seed_game(&pool).await;

        assert!(delete_save_group(&pool, group_id).await.unwrap());
        assert!(get_current_game(&pool, group_id).await.unwrap().is_none());
        assert!(!delete_save_group(&pool, group_id).await.unwrap());
    }

    #[tokio::test]
    async fn invalid_state_json_is_reported_instead_of_becoming_a_default_game() {
        let pool = test_pool().await;
        let (id, group_id, _) = seed_game(&pool).await;
        sqlx::query("UPDATE games SET state = '{not valid json' WHERE id = $1")
            .bind(id.to_string())
            .execute(&pool)
            .await
            .unwrap();

        assert!(get_game_with_revision(&pool, id).await.is_err());
        assert!(get_current_game(&pool, group_id).await.is_err());
        assert_eq!(head(&pool, group_id).await, (id, 1));
    }
}
