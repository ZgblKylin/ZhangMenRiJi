# 《掌门日记》数据库现状（v3）

> 本文描述当前代码真正执行的 SQLite 持久化逻辑。运行时事实来源是
> `backend/src/db/mod.rs`，不是 `backend/migrations/` 中保留的 PostgreSQL
> 历史脚本。

---

## 1. 运行方式与设计边界

- 数据库为 SQLite 单文件库，状态字段使用 JSON 字符串存入 `TEXT`，不是
  PostgreSQL `JSONB`。
- 数据库路径来自桌面配置文件、`DATABASE_URL=sqlite://<path>` 或平台默认数据目录。
  有有效配置文件时配置文件优先，其次才是环境变量和默认值。
- 未显式指定 SQLite `mode` 时，后端连接时补上 `mode=rwc`，允许首次启动创建文件。
- 后端每次启动调用 `run_migrations`，用 `CREATE TABLE/INDEX IF NOT EXISTS`
  幂等创建当前所需对象。
- 当前没有外键；关联数据由应用层事务删除。
- `backend/migrations/0003_world_schema.sql` 与 `0004_save_groups.sql` 是旧
  PostgreSQL 架构资料，当前启动流程没有执行这些 SQL。
- `CREATE TABLE IF NOT EXISTS` 只负责建缺失的表，不会为已经存在的旧 SQLite
  表自动补列。本文的 DDL 是全新数据库会得到的当前结构。

---

## 2. 当前四张表的真实 DDL

### 2.1 `games`：存档时间点

```sql
CREATE TABLE IF NOT EXISTS games (
    id             TEXT PRIMARY KEY,
    sect_name      TEXT NOT NULL,
    state          TEXT NOT NULL,
    schema_version INTEGER NOT NULL DEFAULT 3,
    save_group_id  TEXT NOT NULL,
    save_type      TEXT NOT NULL DEFAULT 'auto',
    created_at     TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at     TEXT NOT NULL DEFAULT (datetime('now'))
);
```

```sql
CREATE INDEX IF NOT EXISTS idx_games_updated_at
    ON games (updated_at DESC);

CREATE INDEX IF NOT EXISTS idx_games_save_group
    ON games (save_group_id, updated_at DESC);
```

字段含义：

| 字段 | 当前含义 |
|---|---|
| `id` | 一个存档时间点的 UUID 字符串；同一局的每次新存档都有新 ID |
| `sect_name` | 玩家门派名，也用于旧档水合 |
| `state` | `GameState` 的完整 JSON 字符串 |
| `schema_version` | 行级状态版本元数据；新 v3 存档写入 `3` |
| `save_group_id` | 槽位 UUID；同一局的自动档和手动档共享此值 |
| `save_type` | 当前代码写入 `auto` 或 `manual` |
| `created_at` / `updated_at` | SQLite UTC 时间文本；初建值精度到秒，后续状态写入精度到毫秒 |

SQLite 表本身没有对 UUID 格式、`save_type` 值或门派名长度加 `CHECK` 约束。

### 2.2 `save_group_heads`：槽位权威指针与修订号

```sql
CREATE TABLE IF NOT EXISTS save_group_heads (
    save_group_id  TEXT PRIMARY KEY,
    current_game_id TEXT NOT NULL,
    revision       INTEGER NOT NULL DEFAULT 1 CHECK (revision > 0),
    updated_at     TEXT NOT NULL
        DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now'))
);
```

```sql
CREATE INDEX IF NOT EXISTS idx_save_group_heads_updated
    ON save_group_heads (updated_at DESC);
```

每个存档槽位恰有一个 head：

| 字段 | 当前含义 |
|---|---|
| `save_group_id` | 槽位 UUID，也是 head 的主键 |
| `current_game_id` | 槽位当前权威时间点；读取历史节点不会改变它，成功写入才会推进 |
| `revision` | 槽位级、从 1 开始单调递增的乐观锁版本 |
| `updated_at` | 最近一次 head 变化时间，精度到毫秒 |

修订号放在槽位而非单个 `games` 行上，是因为完整月结和手动存档会创建新行；
若只比较来源行版本，两个并发请求仍可能各自插入一个合法后继节点。

启动迁移会先删除引用了缺失或跨组节点的悬空 head，再为没有 head 的旧槽位按
`games.updated_at DESC, rowid DESC` 确定性选取最新节点，并以 revision 1
回填。已有合法 head 及其 revision 会被保留。

### 2.3 `game_snapshots`：预留的回合快照表

```sql
CREATE TABLE IF NOT EXISTS game_snapshots (
    game_id   TEXT NOT NULL,
    turn      INTEGER NOT NULL,
    state     TEXT NOT NULL,
    checksum  TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (game_id, turn)
);
```

```sql
CREATE INDEX IF NOT EXISTS idx_snapshots_game_turn
    ON game_snapshots (game_id, turn DESC);
```

**当前状态：仅建表、建索引，并在删档时清理。代码没有任何
`INSERT INTO game_snapshots` 或从该表读取的查询。** 因此它目前不是存档、
回滚或校验功能的一部分；真正的时间点都存为 `games` 行。

### 2.4 `events`：可查询事件副本

```sql
CREATE TABLE IF NOT EXISTS events (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    game_id    TEXT NOT NULL,
    year       INTEGER NOT NULL,
    month      INTEGER NOT NULL,
    mood       TEXT NOT NULL,
    text       TEXT NOT NULL,
    category   TEXT NOT NULL DEFAULT 'world',
    payload    TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
```

```sql
CREATE INDEX IF NOT EXISTS idx_events_game_id
    ON events (game_id);

CREATE INDEX IF NOT EXISTS idx_events_game_time
    ON events (game_id, year DESC, month DESC);

CREATE INDEX IF NOT EXISTS idx_events_category
    ON events (game_id, category, year DESC, month DESC);
```

每次写入事件时，`payload` 当前只保存事件的 `mood` 与 `category`：

```json
{"mood":"good","category":"sect"}
```

当前后端会向 `events` 写入并在删档时清理，但没有从 `events` 读取纪事的 API。
界面使用的是 `games.state.event_log` 以及操作响应中的 `events`。因此
`events` 是独立事件副本，并非当前界面的读取来源。

---

## 3. v3 `state` 快照

`games.state` 是 `GameState` 的完整序列化结果。当前顶层结构如下；嵌套对象省略了
与数据库设计无关的细字段：

```jsonc
{
  "schema_version": 3,
  "autosave": true,
  "year": 1,
  "month": 1,

  // 兼容旧 API 的门派镜像字段
  "prestige": 45,
  "silver": 500,
  "morale": 55,
  "injury": 0,

  "disciples": [
    {
      "id": "...",
      "sect_id": "player",
      "origin_sect_id": "player",
      "is_named_npc": false,
      "npc_position": null,
      "name": "...",
      "talent": 20,
      "inner_power": 30,
      "martial_art": "hunyuan",
      "prepared_skills": {},
      "loyalty": 60,
      "months_in_sect": 0,
      "alive": true,
      "age": 18,
      "aptitudes": {},
      "attributes": {},
      "attribute_bonuses": {},
      "martial_schema_version": 3,
      "condition": "healthy",
      "rank": "outer",
      "merit": 0,
      "department": null,
      "master_id": null,
      "relations": {},
      "skills": [],
      "martial_progress": {},
      "action": null,
      "away_months": 0,
      "personal_silver": 0,
      "personal_rations": 0
    }
  ],
  "martial_arts_learned": ["player_knowledge", "hunyuan"],
  "event_log": [],
  "decisions_used": 0,
  "max_decisions": 3,
  "total_disciples_recruited": 0,
  "game_over": false,
  "game_won": false,
  "game_over_reason": "",
  "tournament_history": [],
  "pending_event": null,

  // v3 的门派与世界状态
  "sect": {
    "id": "player",
    "name": "无名派",
    "description": "...",
    "landmark": "山门",
    "country_id": "song",
    "player_controlled": true,
    "attributes": {},
    "policy": "balanced",
    "moral_direction": "righteous",
    "rank_rules": {},
    "buildings": [],
    "inventory": {},
    "public_books": [],
    "martial_research": {},
    "relations": {},
    "active_orders": [],
    "productions": [],
    "auto_brew_queue": [],
    "auto_brew_index": 0,
    "auto_brew_progress": 0
  },
  "npc_sects": [],
  "npc_disciples": [],
  "countries": [
    { "id": "yuan", "name": "大元", "prosperity": 72, "order": 68 },
    { "id": "song", "name": "大宋", "prosperity": 85, "order": 62 },
    { "id": "dali", "name": "大理", "prosperity": 70, "order": 78 },
    { "id": "xia", "name": "大夏", "prosperity": 58, "order": 55 }
  ],
  "world_seed": 0
}
```

v3 以内嵌的 `sect.attributes` 为门派属性主状态，同时继续维护顶层
`prestige`、`silver`、`morale` 供旧逻辑与旧 API 使用。读取存档水合结束时，
内嵌值会同步到这些顶层字段；仍修改顶层字段的旧逻辑在关键结算点会反向汇入
`sect.attributes`。

`schema_version` 同时存在于 JSON 和 `games` 列中。读取存档时当前查询只取
`sect_name, state`，实际反序列化依据 JSON；新建存档与原地更新都会把
`state.schema_version` 同步写入行级 `schema_version`。

人物部门以 `transmission`、`library`、`apothecary`、`treasury`、
`stewardship`、`external_affairs` 六个枚举值写入 `department`。
`master_id` 保存同一人物名册内的师父 ID，师徒双方关系写在各自
`relations` 中；载入和月结会清理跨派、亡故、非内门、超额或成环引用，
NPC 世界还会确定性补配缺失师承。它们都属于 `state` JSON，不另建师承表。

`countries` 同样直接嵌入状态快照，不另建国家表。水合会补齐固定四国、保留旧档
已有进度并把 `prosperity`、`order` 钳制到 0～100；月结中的经济倍率、NPC
招募、互动反馈和战争支援都读取这份快照，因此国势变化会随自动档完整保存。

---

## 4. 槽位、手动档与自动档

### 4.1 标识层级

```text
一个槽位 save_group_id
├── save_group_heads：current_game_id + revision
└── games
    ├── id：开局初始自动档
    ├── id：月度自动档
    ├── id：手动档
    └── id：后续自动档
```

- `save_group_id` 表示“一局游戏/一个槽位”。
- `games.id` 表示该槽位中的一个存档节点；当前月内操作会原地更新该节点，
  完整跨月后才创建新的时间点 ID。
- `save_group_heads.current_game_id` 表示最近一次成功修改得到的权威节点；
  `revision` 对该槽位的所有可变请求统一排序。
- 载入任一历史时间点仍可继续，但写入必须携带读取时得到的当前槽位 revision；
  成功后 head 会指向更新后的该节点或新建节点，形成显式分支。

### 4.2 开局

开局时生成新的 `id` 和 `save_group_id`，初始行写为：

```text
save_type = "auto"
state.autosave = true
```

处理器先在内存中生成初始弟子、补齐 v3 玩家门派与完整 NPC 世界，再在同一
事务中插入完整 `games` 状态与 revision 1 的槽位 head。开局不再先插入默认
空白状态后原地覆盖。

### 4.3 月内操作

门派管理、月内决策以及“出现待选择事件但月份尚未推进”的情况原地覆盖当前
时间点。所有修改或删除存档的 HTTP 请求必须携带 `X-Save-Revision`。事务的第一条写语句
先在 `save_group_heads` 上比较并交换 revision；只有认领成功的请求才会更新
`games` 并附加该次 `events` 副本。任一后续写入失败，head、状态和事件都会
一起回滚。

### 4.4 月度自动档

月份实际推进后，或交互事件被选择并完成结算后，后端调用带 revision 的
`create_save_with_events_if_revision`：

1. 校验来源 `id` 属于请求的 `save_group_id`，并以预期 revision 认领槽位
   head；head 在事务内先指向将要创建的新 UUID；
2. 克隆当前状态并设置 `state.autosave = true`；
3. 以该 UUID 插入一条 `save_type = 'auto'` 的 `games` 行；
4. 把本次完整月结事件写入以新 ID 关联的 `events`；
5. 查出并删除第 11 条起的自动档，同时清理其 `events` 与
   `game_snapshots` 记录；
6. 提交事务并返回新存档 ID 与递增后的 revision，前端原子接管新会话。

每个槽位最多保留最近 **10 条** `save_type = 'auto'` 的行。插入自动档后，
查询第 11 条起的自动档并逐条删除；手动档不计入该上限。新自动档、事件副本
与超限清理属于同一事务，且当前 head 永远不会被淘汰；事件插入或清理失败会
令 head 认领、新档创建和清理全部回滚。

### 4.5 手动档

手动存档同样参与槽位 CAS，并在事务中创建新 `games` 行、推进 head，只是：

```text
save_type = "manual"
state.autosave = false
```

手动档当前没有数量上限。

`save_type` 是数据库行类型，`state.autosave` 是 JSON 内的同义展示标记。
当前新写入流程会同步二者。存档列表实际同时返回 `save_type`，并从 JSON
提取 `autosave`。

---

## 5. 当前代码的真实查询

SQLx 对 SQLite 使用 `$1`、`$2` 形式的绑定参数。主要查询如下。

### 5.1 创建与读取

```sql
INSERT INTO games
    (id, sect_name, state, schema_version, save_group_id, save_type)
VALUES ($1, $2, $3, $4, $5, $6);
```

```sql
SELECT game.id, game.sect_name, game.state, game.save_group_id,
       head.current_game_id, head.revision
FROM games AS game
JOIN save_group_heads AS head
  ON head.save_group_id = game.save_group_id
WHERE game.id = $1;
```

新游戏会在同一事务中插入 `games` 和 revision 1 的
`save_group_heads`。读取任一历史节点时，返回的是该节点状态以及槽位当前的
`current_game_id`/`revision`；按槽位读取当前权威状态时则从 head 反向连接
`games`。

### 5.2 存档列表

```sql
SELECT
    game.id,
    game.save_group_id,
    game.save_type,
    game.sect_name,
    COALESCE(CAST(json_extract(game.state, '$.autosave') AS INTEGER), 1) AS autosave,
    COALESCE(CAST(json_extract(game.state, '$.year') AS INTEGER), 1) AS year,
    COALESCE(CAST(json_extract(game.state, '$.month') AS INTEGER), 1) AS month,
    game.updated_at,
    head.current_game_id,
    head.revision,
    head.updated_at
FROM save_group_heads AS head
JOIN games AS current
  ON current.id = head.current_game_id
 AND current.save_group_id = head.save_group_id
JOIN games AS game ON game.save_group_id = head.save_group_id
ORDER BY head.updated_at DESC, head.save_group_id DESC,
         CASE WHEN game.id = head.current_game_id THEN 0 ELSE 1 END,
         game.updated_at DESC, game.rowid DESC;
```

该查询以单条 `games JOIN save_group_heads` 的 SQLite 快照读取节点与 head，
不反序列化完整状态，只从 JSON 提取列表所需字段。处理器按 `save_group_id`
在内存中分组，并在每组给出 `current_id` 与
`revision`；当前权威节点固定排在组内首位，其余节点最近优先。槽位按 head
更新时间最近优先。
旧 JSON 缺少 `autosave`、`year` 或 `month` 时，列表查询分别回退为
`true`、`1`、`1`。

### 5.3 revision CAS、原地更新与自动档淘汰

每次原地状态写入或新建存档先执行以下等价条件更新；这是事务的第一条写语句：

```sql
UPDATE save_group_heads
SET current_game_id = $target_id,
    revision = revision + 1,
    updated_at = strftime('%Y-%m-%d %H:%M:%f', 'now')
WHERE save_group_id = $save_group_id
  AND revision = $expected_revision
  AND EXISTS (
      SELECT 1 FROM games
      WHERE games.id = $source_id
        AND games.save_group_id = save_group_heads.save_group_id
  );
```

影响一行才可继续写入状态。影响零行时，后端重新读取当前 head：revision 已
前进则返回 stale conflict 和赢家的完整当前状态；来源缺失或不属于该槽位则
返回不存在。规则预校验前也会比较请求 revision，但最终仍以事务内 CAS 防止
检查和写入之间的竞态。

```sql
UPDATE games
SET sect_name = $2,
    state = $3,
    schema_version = $4,
    updated_at = strftime('%Y-%m-%d %H:%M:%f', 'now')
WHERE id = $1;
```

```sql
SELECT id
FROM games
WHERE save_group_id = $1
  AND save_type = 'auto'
ORDER BY updated_at DESC, rowid DESC
LIMIT -1 OFFSET $2;
```

当前淘汰查询的 offset 固定绑定为 `10`。同一秒产生多条记录时以 `rowid`
作为次级顺序，避免自动档上限的保留对象不确定。

### 5.4 事件写入

```sql
INSERT INTO events
    (game_id, year, month, mood, text, category, payload)
VALUES ($1, $2, $3, $4, $5, $6, $7);
```

事件仍逐条插入，但这些插入不会在事务之外单独提交：

- `update_game_with_events_if_revision` 把 head CAS、原地 `UPDATE games` 与全部
  事件插入放入同一事务；
- `create_save_with_events_if_revision` 把 head CAS、新 `games` 行、全部事件插入
  和超限自动档清理放入同一事务；
- 任一事件插入失败会回滚该事务中的状态更新、新档插入和清理。

这里的“原子”同时覆盖状态、事件、head 和自动档清理；槽位级 revision/CAS
还保证同一版本的并发写只有一个请求提交。系统不会尝试自动合并两个分支，落败
请求收到当前权威状态后由前端接管。

### 5.5 删除单档与整个槽位

删除单档同样先以请求 revision 条件更新槽位 head；认领成功后在事务中执行：

```sql
DELETE FROM events WHERE game_id = $1;
DELETE FROM game_snapshots WHERE game_id = $1;
DELETE FROM games WHERE id = $1;
```

删除当前节点时，代码从剩余同组节点中确定性选择最近一条作为新 head；若组内
已空则删除 head。认领步骤已经把 revision 递增一次，因此重指 head 不会重复
递增。删除历史节点也只递增一次，使已打开页面的下一次请求不会在过期列表基础上
静默提交。成功删除单档返回替代的当前完整状态，组内已空则返回 `current: null`。

删除整个槽位也必须匹配该组 revision。事务首写认领成功后删除组内所有行并
通过 `RETURNING id` 收集关联清理目标：

```sql
DELETE FROM games
WHERE save_group_id = $1
RETURNING id;
```

随后在同一事务中逐个清理其 `events`、`game_snapshots`，最后执行：

```sql
DELETE FROM save_group_heads WHERE save_group_id = $1;
```

---

## 6. 旧档反序列化与水合

旧档兼容不是通过逐版修改数据库 JSON 完成，而是在每次 `get_game` 后于内存中
执行。流程如下。

### 6.1 Serde 默认值

`GameState`、`Disciple`、`SectState` 及主要嵌套结构使用
`#[serde(default)]`。旧 JSON 缺少新字段时，以当前 Rust `Default` 补齐，
例如门派世界、人物属性、行动、旅程卷宗、个人银两与口粮等。

若整段 `state` 不是合法的 `GameState` JSON，或既有字段类型无法兼容当前模型，
读取会返回解码错误，绝不会生成默认局并在下一次写入时覆盖损坏存档；正常缺失
字段仍由 `#[serde(default)]` 补齐。

### 6.2 玩家门派水合

`hydrate_player_sect(state, sect_name)` 会：

1. 若内嵌门派仍是“无名派”或空名，用 `games.sect_name` 命名，并把旧顶层
   声望、库银、士气迁入 `sect.attributes`；
2. 水合全部玩家弟子；
3. 把早期五库/药库建筑布局规范为当前七座职能建筑，并保留可映射的等级、
   完好度、工程进度、长老与事务；
4. 规范物资名，兼容旧金疮药名称，并为缺失的“粮秣”“精铁”库存补零；
5. 移除无资格或重复的长老任命；
6. 清理缺人、跨派、亡故、品阶不符、超额或成环的玩家师承；
7. 确保门派公库和旧顶层已掌握武学都含 `player_knowledge`；
8. 水合 NPC 世界；
9. 将 v3 门派属性同步回旧顶层镜像字段。

### 6.3 弟子武学水合

`hydrate_v2_disciple` 会合并并规范旧武学数据：

- 当 `martial_progress` 为空而旧 `skills` 有值时，迁入新进度表；
- 把旧武学 ID 与准备槽 ID 规范为当前 ID；
- 兼容旧字段名 `equipped_skills`，序列化时统一写为 `prepared_skills`；
- 推断并保存武学来源门派，补齐必要基础技能；
- 对旧 `martial_schema_version` 补齐内力、精力和造诣安全余量，再更新为当前
  人物武学版本 `3`，不会倒扣既有技能等级；
- 重新计算属性上限、人物状态和旧展示字段；
- 旧 `rank = "elder"` 读取为 `inner`，是否担任长老以建筑负责人字段为准。
- 旧 `ActionPlan` 缺少 `journey` 时保持为 `None`；若人物正处于外派或游历，
  下一次月结会以稳定种子补建任务模板、目的地、难度和唯一奇遇，不改数据库表。

### 6.4 NPC 世界水合

`hydrate_world` 会：

- NPC 门派少于 18 个或 NPC 弟子为空时，用 `world_seed` 重建标准世界；
- 否则只迁移朝廷模板等已知旧结构，避免重置旧世界经营进度；
- 补齐国家资料；
- 水合所有 NPC 弟子；
- 规范 NPC 门派建筑与长老任命；
- 保留合法旧师承，清理异常引用，并按稳定顺序为 NPC 确定性补配师父；
- 补齐缺失的门派关系。

### 6.5 何时写回

水合首先只发生在内存中。单纯 `GET /api/games/:id` 不会把水合结果写回数据库；
之后执行管理、决策、原地事件保存或创建新存档时，才会随完整 `state` 一起持久化。

旧 JSON 缺少 `autosave` 时有一个刻意保留的现状差异：列表 SQL 回退为
`true`，而完整状态反序列化采用 `GameState::default()` 中的 `false`。新创建或
重新保存的存档会由当前保存流程写入明确值。

---

## 7. 一致性与当前限制

- `games.state.event_log` 与 `events` 仍是两份数据；当前保存路径以事务保证同次
  状态和事件副本共同提交，但前端仍以 `state.event_log` 为权威，尚未形成
  单一事件查询来源。
- `save_group_heads.revision` 与事务内 CAS 会检测槽位级并发修改；HTTP 层把
  过期请求报告为 `stale_revision`，并附带最新权威状态。系统刻意不自动合并
  两份游戏状态。
- `events.game_id` 和 `game_snapshots.game_id` 没有外键，完整性依靠删档代码维护。
- `game_snapshots` 当前未启用，不能据此承诺回滚、断点恢复或校验。
- `schema_version = 3` 表示当前状态模型，但旧档升级主要依靠 Serde 默认值和水合
  函数，并非一套按 `schema_version` 顺序执行的数据库迁移链。
- `CREATE TABLE IF NOT EXISTS` 仍不会为结构更早、缺少 `games` 既有列的数据库
  自动补列；head 表可以自动回填，但这不等于完整的逐版本 DDL 迁移框架。
- SQLite JSON1 的 `json_extract` 用于快速生成存档列表；完整游戏读取仍解析
  `state` JSON。

---

> 文档版本：v3.2（按当前 SQLite 实现核对）
> 最后核对：2026-07-19
