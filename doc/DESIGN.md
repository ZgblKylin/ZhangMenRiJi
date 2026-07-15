# 《掌门日记》完整设计文档

> 重构版本 v2.0 — 前后端分离 · Rust 后端 · PostgreSQL 持久化
>
> 文档修改：整合 ARCHITECTURE.md 与 DATABASE.md，并依据 backend/src/ 实际源码完善细节。

---

## 1. 项目概述与重构目标

### 1.1 项目简介

《掌门日记》是一款武侠门派经营模拟器。玩家以掌门身份，按月推进时间，管理门派（招募弟子、修炼武学、处理江湖事件），参加年终论剑，最终将三流山寨经营成名震江湖的大派。

游戏以"月"为基本回合单位，每月可执行若干次决策（默认 3 次），然后推进到下一个月。12 月结束自动触发年终论剑。

### 1.2 重构目标

| 维度 | v1（当前） | v2（目标） |
|------|-----------|----------|
| 架构 | 纯前端 HTML/JS 单文件 | 前后端分离工程 |
| 后端语言 | 无 | Rust + axum 0.8 |
| 运行时 | 浏览器 JavaScript | tokio 异步运行时 |
| 数据存储 | LocalStorage（JSON 序列化） | PostgreSQL 18（JSONB 列） |
| 游戏逻辑 | 全部在浏览器执行 | 核心逻辑移至后端 |
| 配置管理 | 硬编码 | .env 文件 + 环境变量 |
| 数据库连接 | 无 | sqlx 异步连接池（编译期查询校验） |
| 前端框架 | Vanilla HTML/CSS/JS | 保持不变 |
| 并发模型 | 单浏览器 tab | 多游戏存档并行（每个存档 UUID 独立） |

---

## 2. 技术选型

### 2.1 后端

| 技术 | 版本 | 用途 |
|------|------|------|
| **Rust** | 1.85+ (stable) | 主语言 |
| **axum** | 0.8 | HTTP 框架（基于 tower） |
| **sqlx** | 0.8 | 异步 PostgreSQL 驱动，编译期 SQL 校验 |
| **tokio** | 1.x | 异步运行时（multi-thread） |
| **serde / serde_json** | 1.x | JSON 序列化/反序列化 |
| **uuid** | 1.x | 游戏存档唯一标识（UUID v4） |
| **tower-http** | 0.6 | CORS 中间件（开发阶段允许 `*`） |
| **dotenvy** | 0.15 | 从 .env 文件加载环境变量 |
| **tracing / tracing-subscriber** | 0.1 | 结构化日志 |
| **rand** | 0.8 | 伪随机数生成（游戏随机性） |
| **chrono** | — | 时间类型（用于 sqlx::FromRow） |

### 2.2 前端

| 技术 | 版本 | 用途 |
|------|------|------|
| **Vanilla HTML/CSS/JS** | — | 保持现有 UI，仅替换存储层 |
| **Fetch API** | 原生 | HTTP 请求后端 REST API |

> **设计决策**：前端保持纯 HTML/JS 以降低复杂度。武侠宣纸风 UI 已成熟（约 1278 行），无需引入 Vue/React 等重型框架。

### 2.3 数据库

| 技术 | 版本 | 用途 |
|------|------|------|
| **PostgreSQL** | 18（Alpine） | 主数据库 |
| **数据库名** | `zhangmenriji` | 项目专用库 |
| **用户** | `ruoruo` | 数据库所有者 |

---

## 3. 系统架构图

```
┌─────────────────────────────────────────────────────────────────┐
│                       浏览器 (Frontend)                          │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │  index.html  (武侠宣纸风 UI)                                │  │
│  │  ├─ 内联 CSS  — 保留原有样式                               │  │
│  │  └─ 内联 JS   — 渲染 + 事件处理 + Fetch API 调用            │  │
│  └────────────────────────────┬──────────────────────────────┘  │
│                               │  HTTP REST (JSON)                │
└───────────────────────────────┼──────────────────────────────────┘
                                │
┌───────────────────────────────┼──────────────────────────────────┐
│                        Rust Backend (axum)                        │
│                               ▼                                    │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │  HTTP Layer                                                │  │
│  │  ├─ tower-http CorsLayer  — CORS 中间件（开发: `*`）       │  │
│  │  └─ tracing                — 请求日志                      │  │
│  ├───────────────────────────────────────────────────────────┤  │
│  │  Router (router.rs)                                        │  │
│  │                                                             │  │
│  │  游戏 CRUD:                                                 │  │
│  │  ├─ POST   /api/games                        → 创建新游戏   │  │
│  │  ├─ GET    /api/games                        → 列出存档     │  │
│  │  ├─ GET    /api/games/{id}                   → 获取游戏状态  │  │
│  │  └─ DELETE /api/games/{id}                   → 删除存档     │  │
│  │                                                             │  │
│  │  游戏操作:                                                  │  │
│  │  ├─ POST   /api/games/{id}/decisions/{decision_id}          │  │
│  │  │                      → 执行决策                          │  │
│  │  └─ POST   /api/games/{id}/advance          → 推进月份      │  │
│  │                                                             │  │
│  │  静态数据:                                                  │  │
│  │  ├─ GET    /api/decisions                    → 8 种决策定义 │  │
│  │  └─ GET    /api/martial-arts                 → 5 门武学数据 │  │
│  ├───────────────────────────────────────────────────────────┤  │
│  │  Handlers Layer (handlers/)                                 │  │
│  │  ├─ games.rs     — 游戏 CRUD handler                       │  │
│  │  ├─ decisions.rs — 决策执行 handler                        │  │
│  │  ├─ advance.rs   — 月度推进 handler                        │  │
│  │  └─ static_data.rs — 静态数据接口 handler                  │  │
│  ├───────────────────────────────────────────────────────────┤  │
│  │  Logic Layer (logic/)                                       │  │
│  │  ├─ advance.rs    — 月度推进编排（10 步流水线）             │  │
│  │  ├─ decision.rs   — 8 种决策执行逻辑                        │  │
│  │  ├─ event.rs      — 随机事件池（12+12=24 种） + 触发/应用   │  │
│  │  ├─ disciple.rs   — 弟子生成/命名/成长/叛逃/战力计算       │  │
│  │  └─ tournament.rs — 年终论剑计算                            │  │
│  ├───────────────────────────────────────────────────────────┤  │
│  │  Models Layer (models/)                                     │  │
│  │  ├─ game.rs         — GameState / GameSummary / CreateGame  │  │
│  │  ├─ disciple.rs     — Disciple 结构体                      │  │
│  │  ├─ martial_art.rs  — MartialArt 静态数据（5 门）           │  │
│  │  ├─ event.rs        — GameEvent 结构体                     │  │
│  │  ├─ decision.rs     — DecisionDef 静态数据（8 种）          │  │
│  │  └─ tournament.rs   — TournamentRecord / TournamentResult   │  │
│  ├───────────────────────────────────────────────────────────┤  │
│  │  DB Layer (db/)                                             │  │
│  │  ├─ run_migrations  — 自动建表（幂等 CREATE IF NOT EXISTS） │  │
│  │  ├─ create_game     — INSERT INTO games + RETURNING id      │  │
│  │  ├─ get_game        — SELECT sect_name, state FROM games    │  │
│  │  ├─ list_games      — SELECT ... ORDER BY updated_at DESC   │  │
│  │  ├─ update_game     — UPDATE games SET state = $2 ...       │  │
│  │  ├─ delete_game     — 级联删除 events + games               │  │
│  │  └─ append_events   — INSERT INTO events (逐条)             │  │
│  ├───────────────────────────────────────────────────────────┤  │
│  │  Config (config.rs)                                         │  │
│  │  ├─ 优先读取 DATABASE_URL 环境变量                          │  │
│  │  └─ 否则拼合 PG_HOST / PG_PORT / PG_USER / PG_PASSWORD      │  │
│  └───────────────────────────────────────────────────────────┘  │
└────────────────────────────────┬────────────────────────────────┘
                                 │  TCP :5432
┌────────────────────────────────┼────────────────────────────────┐
│                     PostgreSQL 18 (Alpine)                        │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │  Database: zhangmenriji                                     │  │
│  │  ├─ games  (UUID PK, JSONB state, timestamps)              │  │
│  │  │   └─ 索引: idx_games_updated_at (desc)                   │  │
│  │  └─ events (BIGSERIAL PK, UUID game_id, year, month, mood) │  │
│  │      └─ 索引: idx_events_game_id, idx_events_game_time     │  │
│  └───────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────────┘
```

### 请求处理流程（时序概要）

```
浏览器                    后端 Handler              Logic Layer            DB Layer           PostgreSQL
  │                          │                         │                     │                    │
  │── POST /api/games ──────>│                         │                     │                    │
  │                          │── create_game() ──────────────────────────────────────────────────>│
  │                          │                         │                     │   INSERT+RETURNING │
  │                          │<── (id, state) ───────────────────────────────────────────────────│
  │                          │── generate_starting_disciples()                                    │
  │                          │                         │── generate_disciple() ×2                 │
  │                          │── update_game() ──────────────────────────────────────────────────>│
  │                          │<── game_state ────────────────────────────────────────────────────│
  │<── 201 {id,state} ──────│                         │                     │                    │
  │                          │                         │                     │                    │
  │── POST /advance ───────>│                         │                     │                    │
  │                          │── get_game() ────────────────────────────────────────────────────>│
  │                          │── advance_month() ─────>│                     │                    │
  │                          │                         │── trigger_random_event()                 │
  │                          │                         │── apply_event_effect()                   │
  │                          │                         │── monthly_growth()                       │
  │                          │                         │── check_desertion()                      │
  │                          │                         │── run_tournament() (仅12月)              │
  │                          │<── (events, tournament, game_over)                                │
  │                          │── update_game() ─────────────────────────────────────────────────>│
  │                          │── append_events() ───────────────────────────────────────────────>│
  │<── {events,tournament,..}│                         │                     │                    │
```

---

## 4. 数据模型总览

### 4.1 数据库物理模型

#### `games` 表

| 字段 | 类型 | 约束 | 说明 |
|------|------|------|------|
| `id` | `UUID` | `PRIMARY KEY DEFAULT gen_random_uuid()` | 游戏存档唯一标识 |
| `sect_name` | `TEXT` | `NOT NULL, CHECK(1..10 字符)` | 门派名称 |
| `state` | `JSONB` | `NOT NULL` | 完整游戏状态快照 |
| `created_at` | `TIMESTAMPTZ` | `NOT NULL DEFAULT now()` | 创建时间 |
| `updated_at` | `TIMESTAMPTZ` | `NOT NULL DEFAULT now()` | 最后更新时间 |

**索引**：`idx_games_updated_at` on `(updated_at DESC)`

#### `events` 表

| 字段 | 类型 | 约束 | 说明 |
|------|------|------|------|
| `id` | `BIGSERIAL` | `PRIMARY KEY` | 自增主键 |
| `game_id` | `UUID` | `NOT NULL` | 所属游戏（无 FK 约束） |
| `year` | `INT` | `NOT NULL` | 游戏年 |
| `month` | `INT` | `NOT NULL` | 游戏月 |
| `mood` | `TEXT` | `NOT NULL, CHECK(good\|bad\|neutral)` | 事件情绪 |
| `text` | `TEXT` | `NOT NULL` | 事件描述文本 |
| `created_at` | `TIMESTAMPTZ` | `NOT NULL DEFAULT now()` | 记录时间 |

**索引**：`idx_events_game_id`, `idx_events_game_time` on `(game_id, year DESC, month DESC)`

#### 级联删除策略

删除游戏时，应用层手动执行两步删除：
```sql
DELETE FROM events WHERE game_id = $1;
DELETE FROM games WHERE id = $1;
```

### 4.2 JSONB `state` 结构详解

```jsonc
{
  "year": 1,                       // int: 当前年（≥1）
  "month": 1,                      // int: 当前月（1..12）
  "prestige": 45,                  // int: 江湖声望（0..100）
  "silver": 500,                   // int: 库银（≥0）
  "morale": 55,                    // int: 门人志气（0..100）
  "injury": 0,                     // int: 掌门伤势（0..100，值越大越重）
  "disciples": [                   // Disciple[]
    {
      "id": "d1717920000_1234",    // string: 唯一ID（时间戳+随机数）
      "name": "风清扬",            // string: 姓名（随机组合 20姓×20男名×20女名）
      "talent": 65,                // int: 资质（10..95，决定可学武学）
      "inner_power": 28,           // int: 内力（10..100）
      "martial_art": "taixu",      // string: 当前修炼武学ID
      "loyalty": 62,               // int: 忠诚度（0..100，<15 可能叛逃）
      "months_in_sect": 5,         // int: 入门月数
      "alive": true                // bool: 是否在世（叛逃后移除，非标死亡）
    }
  ],
  "martial_arts_learned": [        // string[]: 已掌握武学ID
    "hunyuan", "taixu"
  ],
  "decisions_used": 0,             // int: 本月已用决策次数
  "max_decisions": 3,              // int: 每月决策上限
  "total_disciples_recruited": 5,  // int: 历史累计招募弟子数
  "game_over": false,              // bool: 游戏是否结束
  "game_over_reason": "",          // string: 结束原因（失败文本）
  "tournament_history": [          // TournamentRecord[]: 历届论剑成绩
    {
      "year": 1,
      "rank": 3,                   // 名次（1 为魁首）
      "total_sects": 8,            // 参赛门派数（8 + year/2）
      "power": 55                  // 本派战力值
    }
  ],
  "pending_event": null            // object|null: 当前待处理事件（预留字段）
}
```

### 4.3 武学静态数据（5 门）

| ID | 名称 | 类型 | 描述 | ATK | DEF | SPD | 资质要求 |
|----|------|------|------|-----|-----|-----|---------|
| `hunyuan` | 混元功 | 内功 | 浑厚绵长，根基扎实 | 3 | 3 | 2 | 20 |
| `taixu` | 太虚剑法 | 剑法 | 攻守兼备，虚实相生 | 3 | 3 | 2 | 30 |
| `xuanbing` | 玄冰心经 | 内功 | 以柔克刚，心如寒冰 | 1 | 5 | 2 | 35 |
| `jiuyang` | 九阳烈掌 | 掌法 | 至刚至猛，金石俱裂 | 5 | 1 | 2 | 40 |
| `zhuifeng` | 追风步 | 轻功 | 踏雪无痕，追风逐电 | 2 | 2 | 5 | 25 |

### 4.4 决策静态数据（8 种）

| ID | 标题 | 消耗 | 前置条件 | 效果摘要 |
|----|------|------|---------|---------|
| `recruit` | 张贴招贤榜 | 50 银 | 银≥50 | 招募 1~2 名随机弟子 |
| `train` | 闭关练功 | 无 | 伤势<30 | 全员内力↑，忠诚↑，伤势微增 |
| `mission` | 遣弟子行侠 | 无 | 弟子≥1 | 随机弟子行侠：胜则得银/声望，败则受伤 |
| `repair` | 修缮山门 | 60 银 | 银≥60 | 志气+5~10 |
| `diplomacy` | 拜会邻派 | 40 银 | 银≥40 | 声望+2~6，可能获赠银两 |
| `rest` | 静养疗伤 | 无 | 无 | 伤势-15~30 |
| `study` | 研习武功 | 30 银 | 银≥30，有未学武学 | 50% 概率习得新武学 |
| `teach` | 传功授艺 | 20 银 | 银≥20，弟子≥1 | 最多 3 名弟子内力/忠诚↑，30% 换武学 |

### 4.5 命名映射表（前/后端兼容）

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

## 5. 完整 API 设计

### 5.1 通用约定

- 所有请求/响应均为 `Content-Type: application/json`
- 成功响应 HTTP 2xx，失败响应 4xx/5xx 并附带错误文本
- 路径参数 `{id}` 为 UUID v4 格式
- 静态数据接口 (`/api/decisions`, `/api/martial-arts`) 无状态，无需认证

### 5.2 端点详情

#### 5.2.1 `POST /api/games` — 创建新游戏

创建后自动生成 2 名初始弟子。

```
Request:  POST /api/games
Body:     { "sect_name": "青云门" }

Response: 201 Created
{
  "id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
  "sect_name": "青云门",
  "state": {
    "year": 1,
    "month": 1,
    "prestige": 45,
    "silver": 500,
    "morale": 55,
    "injury": 0,
    "disciples": [ /* 2名初始弟子 */ ],
    "martial_arts_learned": ["hunyuan"],
    "decisions_used": 0,
    "max_decisions": 3,
    "total_disciples_recruited": 0,
    "game_over": false,
    "game_over_reason": "",
    "tournament_history": [],
    "pending_event": null
  }
}
```

#### 5.2.2 `GET /api/games` — 列出所有存档

```
Request:  GET /api/games

Response: 200 OK
{
  "games": [
    {
      "id": "a1b2c3d4-...",
      "sect_name": "青云门",
      "state": { /* 完整 state JSONB */ },
      "updated_at": "2026-06-16T12:34:56+00:00"
    }
  ]
}
```

#### 5.2.3 `GET /api/games/{id}` — 获取单个游戏状态

```
Response: 200 OK
{
  "id": "a1b2c3d4-...",
  "sect_name": "青云门",
  "state": { /* 完整游戏状态 */ }
}

Errors:
  404 存档不存在
  500 内部错误
```

#### 5.2.4 `DELETE /api/games/{id}` — 删除存档

级联删除关联的 events 记录。

```
Response: 204 No Content

Errors:
  404 存档不存在
  500 内部错误
```

#### 5.2.5 `POST /api/games/{id}/decisions/{decision_id}` — 执行决策

`decision_id` 为 8 种之一：`recruit` | `train` | `mission` | `repair` | `diplomacy` | `rest` | `study` | `teach`

```
Request:  POST /api/games/{id}/decisions/recruit

Response: 200 OK
{
  "ok": true,
  "events": [
    {
      "text": "招贤榜贴出，1人前来拜山投师。",
      "mood": "good",
      "year": 1,
      "month": 3
    }
  ],
  "state": { /* 更新后的完整游戏状态 */ }
}

Errors:
  400 本月决策次数已用完（decisions_used >= max_decisions）
  404 存档不存在
  500 内部错误
```

#### 5.2.6 `POST /api/games/{id}/advance` — 推进月份

推进月份时后端执行完整 10 步流水线（详见 §6.5）。

```
Request:  POST /api/games/{id}/advance

Response: 200 OK
{
  "events": [
    { "text": "邻派遣使来谒...", "mood": "good", "year": 1, "month": 3 }
  ],
  "tournament": {           // 仅12月返回，否则 null
    "rank": 2,
    "total_sects": 8,
    "power": 55,
    "desc_text": "本派位列三甲...",
    "reward_silver": 200,
    "reward_prestige": 10
  },
  "game_over": false,
  "state": { /* 更新后的完整游戏状态 */ }
}

Errors:
  404 存档不存在
  500 内部错误
```

#### 5.2.7 `GET /api/decisions` — 获取决策定义列表

返回前端渲染决策面板所需的静态数据。

```
Response: 200 OK
{
  "decisions": [
    {
      "id": "recruit",
      "title": "张贴招贤榜",
      "desc": "遣人在山下城镇张贴招贤榜文...",
      "cost": 50,
      "cost_type": "silver",
      "req_injury_max": null,
      "req_silver_min": 50,
      "req_disciples_min": null,
      "req_unlearned_arts": false
    }
    // ... 共 8 条
  ]
}
```

#### 5.2.8 `GET /api/martial-arts` — 获取武学列表

```
Response: 200 OK
{
  "arts": [
    {
      "id": "taixu",
      "name": "太虚剑法",
      "type": "剑法",
      "desc": "攻守兼备，虚实相生...",
      "atk": 3,
      "def": 3,
      "spd": 2,
      "req_talent": 30
    }
    // ... 共 5 条
  ]
}
```

---

## 6. 核心业务逻辑流程

### 6.1 决策系统（`logic/decision.rs`）

**入口**：`execute_decision(rng, state, decision_id) -> Vec<GameEvent>`

**前置校验**（在 handler 层完成）：
- 存档存在性检查
- 决策次数余量检查（`decisions_used < max_decisions`）

**8 种决策详细逻辑**：

```
recruit（招募）
├── 扣银 50
├── 招募 1~2 名弟子（声望 >50 时资质额外 +10）
├── total_disciples_recruited += 招募数
└── 事件：「招贤榜贴出，N人前来拜山投师。」

train（练功）
├── 前置：伤势 < 30（否则无效）
├── 全员：内力 +(2~6 - injury/20)，至少 +1
├── 全员：忠诚 +(1~4)
├── 掌门：伤势 +(3~8)
├── 志气 +2
└── 事件：「掌门率众苦练一月...」

mission（行侠）
├── 前置：有在世弟子（否则无效）
├── 随机选 1 名弟子
├── 战力 check = 资质×0.3 + 内力×0.3 + (武学ATK+DEF+SPD)×3 + 忠诚×0.1
├── score > 25 → 成功: 银 +(20~80 + score/2), 声望 +(1~4), 忠诚 +(1~5)
├── score ≤ 25 → 失败: 内力 -5, 忠诚 -(3~8), 掌门伤势 +(3~8)
└── 分别生成对应文本

repair（修缮）
├── 扣银 60
├── 志气 +(5~10)
└── 事件：「山门修缮一新...」

diplomacy（外交）
├── 扣银 40
├── 声望 +(2~6)
├── 可能获赠银两 (0~30)
├── 志气 +1
└── 事件：「掌门携礼拜访邻派...」

rest（疗伤）
├── 伤势 -(15~30)
├── 志气 -1
└── 事件：「掌门闭门静养月余...」

study（研习）
├── 前置：有未学武学
├── 扣银 30
├── 伤势 +(5~12)
├── 50% 概率：随机习得一门未学武学，声望 +3
├── 50% 概率：失败（neutral 事件）
└── 相应事件文本

teach（传功）
├── 前置：有在世弟子
├── 扣银 20
├── 伤势 +(3~7)
├── 取最多 3 名弟子：内力 +(3~10), 忠诚 +(2~6)
├── 每名弟子 30% 概率切换到已掌握的武学
└── 事件：「掌门亲自为XX、XX传功...」
```

**后处理**：`decisions_used += 1`，事件写入 `state.event_log`（保留最近 50 条）。

### 6.2 随机事件系统（`logic/event.rs`）

**事件池结构**：24 种事件，分为两组

**江湖风云（12 种）**：

| ID | 事件 | 效果类型 | 影响 |
|----|------|---------|------|
| jh_01 | 邻派遣使来谒 | good | 声望+3, 志气+2 |
| jh_02 | 邻派率众来犯 | bad | 声望-2, 志气-3, 银-30, 伤势+8 |
| jh_03 | 朝廷赐匾 | good | 声望+8, 银+100, 志气+5 |
| jh_04 | 豪杰慕名来投 | good | 声望+2, 志气+3, 免费招募2人 |
| jh_05 | 剿灭山贼 | good | 声望+5, 银+40, 志气+2 |
| jh_06 | 秘笈谣言 | bad | 声望-1, 志气-2, 伤势+5 |
| jh_07 | 云游僧赠心法 | good | 声望+1, 志气+1, 习得未学武学 |
| jh_08 | 共商讨魔 | good | 声望+6, 志气+3, 银-20, 伤势+10 |
| jh_09 | 弟子伤人赔偿 | bad | 声望-3, 银-50, 志气-1 |
| jh_10 | 缉拿大盗 | good | 声望+4, 银+80, 志气+4, 伤势+8 |
| jh_11 | 邻派掌门暴毙 | bad | 声望-5, 志气-3 |
| jh_12 | 西域异人赠药 | good | 声望+2, 志气+1, 银+60 |

**门中生息（12 种）**：

| ID | 事件 | 效果类型 | 影响 |
|----|------|---------|------|
| ms_01 | 米价飞涨 | bad | 银-40 |
| ms_02 | 掌门扭了腰 | bad | 志气-1, 伤势+5 |
| ms_03 | 暴雨毁练功场 | bad | 志气-2, 银-30 |
| ms_04 | 掌门观星悟道 | good | 声望+1, 志气+3, 随机弟子内力+8~20 |
| ms_05 | 厨子手艺极差 | bad | 志气-2, 伤势+3 |
| ms_06 | 发现温泉 | good | 志气+5, 伤势-5 |
| ms_07 | 弟子打架 | bad | 志气-3, 30%弟子忠诚↓ |
| ms_08 | 掌门生日 | good | 志气+4, 银-10 |
| ms_09 | 库房鼠患 | bad | 志气+1, 银-20 |
| ms_10 | 老郎中调理 | good | 志气+3, 银-25, 伤势-8 |
| ms_11 | 弟子探亲 | bad | 志气-1, 全员忠诚随机± |
| ms_12 | 师父遗物 | good | 声望+1, 志气+6, 银+150 |

**触发机制**：
- 每月推进时 50% 概率触发江湖事件，50% 概率触发门中事件
- 在各组内均匀随机选取

**Effect 字段解析**：

| 字段 | 类型 | 作用 |
|------|------|------|
| `prestige` | `Option<i32>` | 声望变化量 |
| `silver` | `Option<i32>` | 库银变化量 |
| `morale` | `Option<i32>` | 志气变化量 |
| `injury` | `Option<i32>` | 伤势变化量 |
| `free_recruit` | `Option<i32>` | 免费招募数 |
| `special` | `Option<String>` | `"manual"`(习得武学) / `"epiphany"`(弟子顿悟) |
| `loyalty_loss` | `Option<bool>` | 30% 概率部分弟子忠诚 ↓ |
| `loyalty_change` | `Option<bool>` | 全员忠诚随机浮动 |

### 6.3 弟子系统（`logic/disciple.rs`）

#### 6.3.1 姓名生成

从 20 个复姓（风、云、萧、慕容、上官……）和 20 个男女名中随机组合。

#### 6.3.2 弟子生成 `generate_disciple(rng, talent_bonus)`

```
资质 = clamp(rand(20..70) + bonus, 10, 95)
内力 = rand(10..30)
武学 = 从 资质+10 ≥ req_talent 的武学中随机选取（至少混元功）
忠诚 = rand(40..70)
ID = "d{timestamp}_{random_4_digit}"
```

#### 6.3.3 初始弟子 `generate_starting_disciples(rng)`

创建游戏时生成 2 名弟子（资质 bonus +15 和 +10），忠诚和内力略高于普通招募。

#### 6.3.4 战力计算

**单名弟子战力**（行侠和论剑用）：
```
combat_score = talent * 0.3  +  inner_power * 0.3  +  (art.atk + art.def + art.spd) * 3  +  loyalty * 0.1
```

**门派总战力**（论剑用）：
```
sect_power = avg(combat_score) + alive_count * 5
```
无弟子时默认战力 = 10。

#### 6.3.5 月度成长 `monthly_growth(rng, disciples, morale)`

```
每名在世弟子：
  months_in_sect += 1
  内力 += rand(0..3) + floor(morale/100 * 2)
  忠诚 += (morale - 50)/20 + rand(-2..2)（都有 clamp）
```

#### 6.3.6 叛逃检查 `check_desertion(rng, disciples)`

```
忠诚 < 15 AND 25% 概率 → 叛逃（从列表中移除）
每叛逃 1 人 → 志气 -5
```

### 6.4 年终论剑（`logic/tournament.rs`）

**触发条件**：12 月推进时自动触发。

**算法**：

```
门派战力 = get_sect_combat_power(disciples)
参赛门派数 = 8 + year / 2
基础名次 = total_sects × max(1 - power/200 - prestige/200, 1/total_sects)
最终名次 = clamp(base_rank + rand(-2..2), 1, total_sects)

奖励：
  第1名: 银+300, 声望+15, 全员内力+2~6
  前3名: 银+200, 声望+10, 全员内力+2~6
  前50%: 银+100, 声望+5,  全员内力+2~6
  末流: 银+30,  声望不变,  志气-5, 全员内力+2~6
```

论剑后记录到 `tournament_history`。

### 6.5 月度推进完整流程（`logic/advance.rs`）

**入口**：`advance_month(rng, state) -> (events, tournament_result?, game_over)`

每次推进执行 11 个步骤：

```
步骤 1: 触发随机事件
   ├─ 50% 江湖事件 / 50% 门中事件
   └─ apply_event_effect() 应用效果 + 生成额外事件

步骤 2: 弟子月度成长
   └─ monthly_growth() 全员内力/忠诚自然增长

步骤 3: 被动收支
   ├─ 收入 = prestige * 3/10 + alive_count * 3
   ├─ 支出 = alive_count * 5 + 20
   └─ silver = max(silver + income - expense, 0)

步骤 4: 志气自然浮动
   └─ morale += rand(-3..3)

步骤 5: 掌门伤势恢复
   └─ injury -= rand(2..5)

步骤 6: 叛逃检查
   ├─ check_desertion() 低忠诚弟子逃亡
   └─ 每逃1人: 志气-5

步骤 7: 库银枯竭处理
   ├─ 若 silver ≤ 0 且 在世弟子 >2
   ├─ 志气-10
   └─ 1/3 弟子离开

步骤 8: 游戏结束检查
   ├─ 条件1: 无弟子 AND 银<20 → game_over
   └─ 条件2: 声望≤0 AND 志气≤0 → game_over

步骤 9: 年终论剑（仅12月）
   ├─ run_tournament() 计算名次、奖励
   └─ 将论剑事件写入事件列表

步骤10: 推进时间
   ├─ month += 1
   ├─ month > 12 → month=1, year+=1
   └─ 新元年写入分隔事件

步骤11: 后处理
   ├─ 事件追加到 state.event_log（保留最近50条）
   ├─ decisions_used = 0
   └─ pending_event = null
```

---

## 7. 模块代码结构

```
backend/
├── Cargo.toml                    # 依赖声明
├── .env                          # 环境配置（不入 git）
└── src/
    ├── main.rs                   # 入口：加载 .env → 连接DB → 迁移 → 路由 → 启动
    ├── config.rs                 # Config 结构体，from_env() 加载
    ├── router.rs                 # Router 定义（8 个路由 + CORS 中间件）
    ├── error.rs                  # AppError 统一错误类型（impl IntoResponse）
    │
    ├── models/                   # —— 数据模型层 ——
    │   ├── mod.rs                # 模块导出
    │   ├── game.rs               # GameState（JSONB 对应体）、GameSummary、CreateGameRequest
    │   ├── disciple.rs           # Disciple 结构体（7 字段）
    │   ├── martial_art.rs        # MartialArt 结构体 + all_martial_arts() 静态数据
    │   ├── event.rs              # GameEvent 结构体（text/mood/year/month）
    │   ├── decision.rs           # DecisionDef 结构体 + all_decisions() 静态数据
    │   └── tournament.rs         # TournamentRecord、TournamentResult
    │
    ├── logic/                    # —— 核心逻辑层 ——
    │   ├── mod.rs                # 模块导出
    │   ├── advance.rs            # advance_month() 月度推进编排（11 步）
    │   ├── decision.rs           # execute_decision() 8 种决策处理
    │   ├── event.rs              # 随机事件池（24种）+ trigger_random_event() + apply_event_effect()
    │   ├── disciple.rs           # generate_disciple() / monthly_growth() / check_desertion() / 战力计算
    │   └── tournament.rs         # run_tournament() 论剑计算
    │
    ├── handlers/                 # —— HTTP 请求处理层 ——
    │   ├── mod.rs                # 模块导出
    │   ├── games.rs              # AppState + create/list/get/delete 4 个 handler
    │   ├── decisions.rs          # execute_decision handler
    │   ├── advance.rs            # advance_month handler
    │   └── static_data.rs        # list_decisions / list_martial_arts
    │
    └── db/                       # —— 数据访问层 ——
        └── mod.rs                # run_migrations + CRUD 查询函数（7 个）
```

**分层依赖关系**（自顶向下）：

```
main.rs
  └── router.rs
        └── handlers/  ──────────────────────────────────┐
              │                                           │
              ├── logic/  ←── 决策/事件/弟子/论剑逻辑     │
              │                                           │
              ├── db/     ←── PostgreSQL 读写             │
              │                                           │
              └── models/ ←── 所有 handler 和 logic 都用   │
```

### 各模块代码量统计

| 模块 | 文件 | 行数 | 职责 |
|------|------|------|------|
| `main.rs` | 1 | 45 | 启动引导 |
| `config.rs` | 1 | 51 | 环境变量加载 |
| `router.rs` | 1 | 23 | 路由注册 |
| `error.rs` | 1 | 16 | 错误处理 |
| `models/` | 6 | 150 | 数据结构定义 |
| `logic/` | 5 | 500+ | 业务逻辑 |
| `handlers/` | 4 | 170 | HTTP 请求处理 |
| `db/` | 1 | 129 | 数据库操作 |

---

## 8. 部署方案

### 8.1 开发环境

```
依赖：
├── PostgreSQL 18（已有共享实例 192.168.50.150:5432）
├── Rust 1.85+ toolchain
└── .env 文件配置数据库连接

启动流程：
1. cargo run             # 后端监听 0.0.0.0:3000
2. 浏览器打开 index.html # API 指向 http://localhost:3000
3. CORS 自动允许跨域请求
```

### 8.2 生产环境（Docker Compose）

```yaml
# docker-compose.yml（规划）
services:
  zhangmenriji-backend:
    build: ./backend
    ports: ["3000:3000"]
    environment:
      - DATABASE_URL=postgres://ruoruo:${PG_PASSWORD}@postgres:5432/zhangmenriji
    depends_on: [postgres]

  zhangmenriji-frontend:
    image: nginx:alpine
    ports: ["8080:80"]
    volumes:
      - ./frontend:/usr/share/nginx/html
      - ./nginx.conf:/etc/nginx/conf.d/default.conf
    # Nginx 反代 /api/* → backend:3000

  postgres:
    image: postgres:18-alpine
    environment:
      POSTGRES_DB: zhangmenriji
      POSTGRES_USER: ruoruo
      POSTGRES_PASSWORD: ${PG_PASSWORD}
    volumes:
      - pgdata:/var/lib/postgresql/data
```

前端通过 Nginx 反向代理 `/api/*` 到后端，避免跨域问题。

### 8.3 数据库自动迁移

启动时 `db::run_migrations()` 自动执行 `CREATE TABLE IF NOT EXISTS`，无需手动跑 SQL。索引同样自动创建。

### 8.4 配置管理

| 环境变量 | 说明 | 默认值 |
|----------|------|--------|
| `DATABASE_URL` | 完整连接串（**优先级最高**） | — |
| `PG_HOST` | PostgreSQL 地址 | `192.168.50.150` |
| `PG_PORT` | PostgreSQL 端口 | `5432` |
| `PG_USER` | 数据库用户 | `ruoruo` |
| `PG_PASSWORD` | 数据库密码 | — |
| `PG_DATABASE` | 数据库名 | `zhangmenriji` |
| `SERVER_HOST` | 后端监听地址 | `0.0.0.0` |
| `SERVER_PORT` | 后端监听端口 | `3000` |

**加载优先级**：`DATABASE_URL` > 各 `PG_*` 拼合 > `.env` 文件

---

## 9. 关键设计决策（ADR）

### ADR-001：游戏状态存储为 JSONB

- **决策**：`games` 表使用单个 `JSONB` 列存储完整游戏状态，而非拆成多表范式化。
- **理由**：
  1. 游戏状态是紧密耦合的整体快照，极少需要按字段查询
  2. JSONB 支持索引，足以覆盖按 id / sect_name 的查询需求
  3. 避免 ORM 复杂度和 N+1 查询问题
  4. 弟子、事件等数据随游戏状态一同读写，无独立查询场景
  5. 单局 JSONB 约 2KB，极轻量
- **权衡**：牺牲了部分字段级校验能力（如 CHECK 约束无法作用于 JSONB 内部字段）

### ADR-002：核心逻辑移至后端

- **决策**：决策执行、随机事件、月度推进、论剑计算全部在后端完成。
- **理由**：
  1. 防止前端作弊（虽然这是单人游戏，但保持架构正确性）
  2. 前端只需渲染，逻辑集中便于维护和测试
  3. 未来支持多端（手机/桌面）共享同一逻辑
- **权衡**：增加了网络往返开销（每次决策/推进需一次 HTTP 请求）

### ADR-003：前端保持 Vanilla JS

- **决策**：不引入 Vue/React 等前端框架。
- **理由**：
  1. 现有 UI 已完成（约 1278 行），迁移框架成本高收益低
  2. 纯 HTML 打开即用，零构建步骤，无打包器依赖
  3. 前端仅负责渲染 + API 调用，复杂度完全可控
  4. 武侠宣纸风 UI 是静态风格，不需要响应式框架
- **权衡**：大型 UI 更新时缺少组件化抽象，维护成本随规模线性增长

### ADR-004：事件独立存储表

- **决策**：事件日志从 `state.event_log`（JSONB 内嵌）独立为 `events` 表。
- **理由**：
  1. `event_log` 仅保留最近 50 条（内存优化），但数据库可永久归档
  2. 独立表支持按时间范围查询历史叙事
  3. 独立存储便于未来数据分析（统计事件类型分布等）
- **权衡**：需要维护双写一致性（state.event_log + events 表）

### ADR-005：无用户认证系统

- **决策**：不引入任何用户认证/授权机制。
- **理由**：
  1. 这是单人游戏，无多用户场景
  2. 所有存档通过 UUID 访问，本质上不可猜测
  3. 开发阶段可通过 CORS 和网络隔离控制访问
- **权衡**：生产部署时如需公网访问，需在前端 Nginx 层加 HTTP Basic Auth

### ADR-006：自动数据库迁移

- **决策**：应用启动时通过 `run_migrations()` 自动执行 `CREATE TABLE IF NOT EXISTS`，不依赖外部迁移工具。
- **理由**：
  1. 表结构极简（仅 2 张表），无需复杂的版本化迁移
  2. 降低运维门槛，启动即用
  3. `IF NOT EXISTS` 保证幂等性
- **权衡**：未来表结构变更需手动编写兼容 SQL

### ADR-007：RNG 与 async 隔离

- **决策**：所有 `rand::thread_rng()` 的创建和使用局限在同步代码块内，在调用 `await` 前完成并 drop。
- **理由**：
  1. `thread_rng()` 非 `Send`，不能跨越 `.await` 边界
  2. 在每个 handler 中显式创建 RNG 块，确保编译器满意
- **实现**：在 `handlers/games.rs`、`handlers/decisions.rs`、`handlers/advance.rs` 中使用 `{ let mut rng = ...; ... }` 代码块模式

### ADR-008：前端后端字段命名约定

- **决策**：前端使用 camelCase，后端/数据库统一使用 snake_case。
- **理由**：
  1. 前端 JS 生态惯例为 camelCase
  2. Rust 和 PostgreSQL 惯例为 snake_case
  3. serde 的 `#[serde(rename)]` 注解处理字段映射
  4. JSONB 内部保持 snake_case 以匹配 Rust 结构体
- **权衡**：前端需要知道后端字段名或做转换

---

## 附录 A：数据量估算

| 指标 | 估算 |
|------|------|
| 单局游戏时长 | 24~60 个月（2~5 年） |
| 单局决策次数 | 每月最多 3 次 × 24~60 = 72~180 |
| 单局事件数（state.event_log） | 保留最近 50 条 |
| 单局事件数（events 表） | 每月 1~2 条 ≈ 24~120 条 |
| state JSONB 大小 | ~2KB（含 5~15 名弟子） |
| 单局总存储 | ~5KB（games 1行）+ ~3KB（events 行汇总）≈ 8KB |
| 100 局总存储 | ~800KB — 极小，无需分区 |

## 附录 B：游戏结束条件

| 条件 | 描述 |
|------|------|
| 弟子归零 + 库银 < 20 | "门中无弟子，库银枯竭。山门冷落，掌门黯然隐退……" |
| 声望 ≤ 0 + 志气 ≤ 0 | "江湖声望尽失，门人志气消沉。本派终究未能撑过难关。" |

---

> 文档版本：v2.0
> 最后更新：2026-06-16
> 基于：ARCHITECTURE.md v1.0、DATABASE.md v1.0、backend/src/ 全部源码
> 架构师：若若 (RuoRuo)
