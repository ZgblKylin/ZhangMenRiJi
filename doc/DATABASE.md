# 《掌门日记》数据库设计文档

> 数据库: PostgreSQL 18 · 库名: `zhangmenriji` · 用户: `ruoruo`

---

## 1. 设计原则

- **JSONB 优先**：游戏状态是紧密耦合的快照，使用 JSONB 列存储完整状态，避免过度范式化
- **事件独立**：事件日志虽可内嵌于 JSONB，但独立一张表便于按时间查询和分析
- **无外键**：`events.game_id` 用 UUID 关联 `games`，但不用 FK 约束（游戏删除时级联由应用层处理）

---

## 2. 表设计

### 2.1 `games` — 游戏存档主表

```sql
CREATE TABLE games (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    sect_name   TEXT NOT NULL CHECK (char_length(sect_name) BETWEEN 1 AND 10),
    state       JSONB NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- 索引
CREATE INDEX idx_games_updated_at ON games (updated_at DESC);
```

#### `state` JSONB 结构

```jsonc
{
  "year": 1,                    // int: 当前年 (1..n)
  "month": 1,                   // int: 当前月 (1..12)
  "prestige": 45,               // int: 江湖声望 (0..100)
  "silver": 500,                // int: 库银 (≥0)
  "morale": 55,                 // int: 门人志气 (0..100)
  "injury": 0,                  // int: 掌门伤势 (0..100)
  "disciples": [                // array: 弟子列表
    {
      "id": "d1717920000_1234", // string: 唯一ID
      "name": "风清扬",         // string: 姓名
      "talent": 65,             // int: 资质 (1..100)
      "inner_power": 28,        // int: 内力 (1..100)
      "martial_art": "taixu",   // string: 当前武学ID
      "loyalty": 62,            // int: 忠诚度 (0..100)
      "months_in_sect": 5,      // int: 入门月数
      "alive": true             // bool: 是否在世
    }
  ],
  "martial_arts_learned": [     // array<string>: 已掌握武学ID
    "hunyuan", "taixu"
  ],
  "decisions_used": 0,          // int: 本月已用决策数
  "max_decisions": 3,           // int: 每月决策上限
  "total_disciples_recruited": 5, // int: 历史累计招募数
  "game_over": false,           // bool
  "game_over_reason": "",       // string: 结束原因
  "tournament_history": [       // array: 历届论剑记录
    {
      "year": 1,
      "rank": 3,
      "total_sects": 8,
      "power": 55
    }
  ],
  "pending_event": null         // object|null: 当前待处理事件
}
```

---

### 2.2 `events` — 事件日志表

```sql
CREATE TABLE events (
    id          BIGSERIAL PRIMARY KEY,
    game_id     UUID NOT NULL,
    year        INT NOT NULL,
    month       INT NOT NULL,
    mood        TEXT NOT NULL CHECK (mood IN ('good', 'bad', 'neutral')),
    text        TEXT NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- 索引
CREATE INDEX idx_events_game_id ON events (game_id);
CREATE INDEX idx_events_game_time ON events (game_id, year DESC, month DESC);
```

| 字段 | 类型 | 说明 |
|------|------|------|
| `id` | BIGSERIAL | 自增主键 |
| `game_id` | UUID | 所属游戏 |
| `year` | INT | 游戏年 |
| `month` | INT | 游戏月 |
| `mood` | TEXT | 事件情绪: good/bad/neutral |
| `text` | TEXT | 事件描述 |
| `created_at` | TIMESTAMPTZ | 记录时间 |

---

## 3. 数据库初始化脚本

```sql
-- 01_create_database.sql (以 postgres 超级用户执行)
CREATE DATABASE zhangmenriji
    OWNER ruoruo
    ENCODING 'UTF8'
    LC_COLLATE = 'en_US.UTF-8'
    LC_CTYPE = 'en_US.UTF-8';

-- 02_create_tables.sql (以 ruoruo 用户执行，连接 zhangmenriji)
CREATE TABLE IF NOT EXISTS games (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    sect_name   TEXT NOT NULL CHECK (char_length(sect_name) BETWEEN 1 AND 10),
    state       JSONB NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS events (
    id          BIGSERIAL PRIMARY KEY,
    game_id     UUID NOT NULL,
    year        INT NOT NULL,
    month       INT NOT NULL,
    mood        TEXT NOT NULL CHECK (mood IN ('good', 'bad', 'neutral')),
    text        TEXT NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_games_updated_at ON games (updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_events_game_id ON events (game_id);
CREATE INDEX IF NOT EXISTS idx_events_game_time ON events (game_id, year DESC, month DESC);
```

---

## 4. 查询模式

### 4.1 高频查询

| 查询 | SQL | 用途 |
|------|-----|------|
| 创建游戏 | `INSERT INTO games (sect_name, state) VALUES ($1, $2) RETURNING id` | 新游戏 |
| 获取游戏 | `SELECT * FROM games WHERE id = $1` | 加载存档 |
| 更新状态 | `UPDATE games SET state = $2, updated_at = now() WHERE id = $1` | 每月存盘 |
| 列出存档 | `SELECT id, sect_name, state->>'year', state->>'month', updated_at FROM games ORDER BY updated_at DESC` | 存档列表 |
| 删除存档 | `DELETE FROM games WHERE id = $1` | 删档 |
| 写事件 | `INSERT INTO events (game_id, year, month, mood, text) VALUES ($1,$2,$3,$4,$5)` | 事件日志 |
| 读事件 | `SELECT * FROM events WHERE game_id = $1 ORDER BY year DESC, month DESC LIMIT 15` | 显示纪事 |

### 4.2 级联删除

删除游戏时同时删除关联事件：

```sql
DELETE FROM events WHERE game_id = $1;
DELETE FROM games WHERE id = $1;
```

---

## 5. 数据迁移 (v1 → v2)

从旧版 LocalStorage JSON 迁移到 PostgreSQL：

```
1. 用户在旧版导出 JSON 文件
2. 新版提供导入 API: POST /api/games/import
   - 接收 JSON，解析 DEFAULT_STATE 格式
   - 映射字段到 state JSONB (key 命名 snake_case)
   - 写入 games + events 表
3. 映射规则：
   - G.sectName   → games.sect_name
   - G.*          → games.state
   - G.eventLog[] → events 表逐条 INSERT
   - 前端字段名 camelCase，后端/DB 统一 snake_case
```

### 命名映射表

| 前端 (camelCase) | 后端/DB (snake_case) |
|-------------------|---------------------|
| `sectName` | `sect_name` |
| `innerPower` | `inner_power` |
| `martialArt` | `martial_art` |
| `martialArtsLearned` | `martial_arts_learned` |
| `monthsInSect` | `months_in_sect` |
| `decisionsUsed` | `decisions_used` |
| `maxDecisions` | `max_decisions` |
| `totalDisciplesRecruited` | `total_disciples_recruited` |
| `gameOver` | `game_over` |
| `gameOverReason` | `game_over_reason` |
| `eventLog` | `event_log` |
| `tournamentHistory` | `tournament_history` |
| `pendingEvent` | `pending_event` |
| `totalSects` | `total_sects` |

---

## 6. 数据量估算

| 指标 | 估算 |
|------|------|
| 单局游戏时长 | 24~60 个月 (2~5 年) |
| 单局事件数 | 每月 1~2 条 ≈ 24~120 条 |
| state JSONB 大小 | ~2KB (含 5~15 名弟子) |
| 单局总存储 | ~5KB |
| 100 局总存储 | ~500KB — 极小，无需分区 |

---

> 文档版本: v1.0  
> 最后更新: 2026-06-09  
> 架构师: 若若 (RuoRuo) 🐱
