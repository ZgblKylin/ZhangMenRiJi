# 《掌门日记》架构设计文档

> 重构版本 v2.0 — 前后端分离 · Rust 后端 · PostgreSQL 持久化

---

## 1. 项目概述

### 1.1 项目简介

《掌门日记》是一款武侠门派经营模拟器。玩家以掌门身份，按月推进时间，管理门派（招募弟子、修炼武学、处理江湖事件），参加年终论剑，最终将三流山寨经营成名震江湖的大派。

### 1.2 重构目标

| v1 (当前) | v2 (目标) |
|-----------|----------|
| 纯前端 HTML/JS 单文件 | 前后端分离工程 |
| 无后端 | Rust 后端 (axum) |
| LocalStorage 存档 | PostgreSQL 持久化 |
| 所有逻辑在浏览器 | 核心逻辑移至后端 |
| 无配置管理 | .env 配置 + 模板 |

---

## 2. 技术选型

### 2.1 后端

| 技术 | 版本 | 用途 |
|------|------|------|
| **Rust** | 1.85+ (stable) | 主语言 |
| **axum** | 0.8 | HTTP 框架 |
| **sqlx** | 0.8 | 异步 PostgreSQL 驱动 (compile-time checked queries) |
| **tokio** | 1.x | 异步运行时 |
| **serde / serde_json** | 1.x | JSON 序列化 |
| **uuid** | 1.x | 游戏存档唯一标识 |
| **tower-http** | 0.6 | CORS 中间件 |
| **dotenvy** | 0.15 | .env 加载 |

### 2.2 前端

| 技术 | 版本 | 用途 |
|------|------|------|
| **Vanilla HTML/CSS/JS** | — | 保持现有 UI，仅替换存储层 |
| **Fetch API** | 原生 | HTTP 请求后端 |

> 设计决策：前端保持纯 HTML/JS 以降低复杂度。武侠宣纸风 UI 已成熟，无需引入重型框架。

### 2.3 数据库

| 技术 | 版本 | 用途 |
|------|------|------|
| **PostgreSQL** | 18 | 主数据库 |
| **数据库名** | `zhangmenriji` | 项目专用库 |

---

## 3. 系统架构

```
┌─────────────────────────────────────────────────────────┐
│                    浏览器 (Frontend)                      │
│  ┌───────────────────────────────────────────────────┐  │
│  │  index.html  (武侠宣纸风 UI)                        │  │
│  │  ├─ CSS: 内联 (保留原有样式)                        │  │
│  │  └─ JS:  渲染 + 事件处理 + API 调用                  │  │
│  └───────────────────────┬───────────────────────────┘  │
│                          │  HTTP REST (JSON)             │
└──────────────────────────┼──────────────────────────────┘
                           │
┌──────────────────────────┼──────────────────────────────┐
│                     Rust Backend (axum)                   │
│                          ▼                                │
│  ┌───────────────────────────────────────────────────┐  │
│  │  HTTP Layer (tower-http CORS)                      │  │
│  ├───────────────────────────────────────────────────┤  │
│  │  Router                                             │  │
│  │  ├─ POST   /api/games            → 创建新游戏      │  │
│  │  ├─ GET    /api/games            → 列出存档        │  │
│  │  ├─ GET    /api/games/:id        → 获取游戏状态    │  │
│  │  ├─ DELETE /api/games/:id        → 删除存档        │  │
│  │  ├─ POST   /api/games/:id/decisions/:decision_id   │  │
│  │  │                                → 执行决策        │  │
│  │  └─ POST   /api/games/:id/advance                  │  │
│  │                                   → 推进月份        │  │
│  ├───────────────────────────────────────────────────┤  │
│  │  Game Logic Layer                                   │  │
│  │  ├─ decision.rs  — 决策系统 (8种决策)               │  │
│  │  ├─ event.rs     — 随机事件 (24种)                  │  │
│  │  ├─ disciple.rs  — 弟子系统 (成长/叛逃)             │  │
│  │  ├─ tournament.rs — 年终论剑                         │  │
│  │  └─ advance.rs   — 月度推进 (整合以上)              │  │
│  ├───────────────────────────────────────────────────┤  │
│  │  Data Layer (sqlx)                                  │  │
│  │  ├─ games table         — 游戏主状态 (JSONB)       │  │
│  │  └─ events table        — 事件日志 (JSONB[])       │  │
│  └───────────────────────────────────────────────────┘  │
└──────────────────────────┬──────────────────────────────┘
                           │  TCP :5432
┌──────────────────────────┼──────────────────────────────┐
│                  PostgreSQL 18 (Alpine)                   │
│  ┌───────────────────────────────────────────────────┐  │
│  │  Database: zhangmenriji                             │  │
│  │  ├─ games     — 游戏存档                            │  │
│  │  └─ events    — 事件日志                            │  │
│  └───────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

---

## 4. API 设计

### 4.1 通用约定

- 所有请求/响应均为 `Content-Type: application/json`
- 成功响应 HTTP 2xx，失败响应 4xx/5xx 并附带 `{ "error": "消息" }`

### 4.2 端点详情

#### `POST /api/games` — 创建新游戏

```json
// Request
{
  "sect_name": "青云门"
}

// Response (201)
{
  "id": "a1b2c3d4-...",
  "sect_name": "青云门",
  "year": 1,
  "month": 1,
  "prestige": 45,
  "silver": 500,
  "morale": 55,
  "injury": 0,
  "disciples": [...],
  "martial_arts_learned": ["hunyuan"],
  "event_log": [...],
  "decisions_used": 0,
  "tournament_history": []
}
```

#### `GET /api/games` — 列出所有存档

```json
// Response (200)
{
  "games": [
    { "id": "...", "sect_name": "青云门", "year": 3, "month": 6, "updated_at": "..." },
    ...
  ]
}
```

#### `GET /api/games/:id` — 获取游戏状态

```json
// Response (200)
{ /* 完整游戏状态，同上 */ }
```

#### `DELETE /api/games/:id` — 删除存档

```json
// Response (204) — No Content
```

#### `POST /api/games/:id/decisions/:decision_id` — 执行决策

```json
// Response (200)
{
  "ok": true,
  "event": { "text": "...", "mood": "good" },
  "game": { /* 更新后的完整游戏状态 */ }
}
```

#### `POST /api/games/:id/advance` — 推进月份

推进月份时后端执行完整流程：
1. 触发随机事件（江湖/门中）
2. 弟子月度成长
3. 被动收支结算
4. 忠诚度检查（叛逃）
5. 库银枯竭检查
6. 游戏结束检查
7. 十二月自动论剑

```json
// Response (200)
{
  "events": [{ "text": "...", "mood": "good" }, ...],
  "tournament": { /* 论剑结果，仅12月返回 */ },
  "game_over": false,
  "game": { /* 更新后的完整游戏状态 */ }
}
```

---

## 5. 模块划分

```
src/
├── main.rs              # 入口：启动 HTTP server
├── config.rs            # 配置加载 (从 .env / 环境变量)
├── router.rs            # 路由定义
├── handlers/
│   ├── mod.rs
│   ├── games.rs         # CRUD 游戏存档
│   ├── decisions.rs     # 执行决策
│   └── advance.rs       # 推进月份
├── models/
│   ├── mod.rs
│   ├── game.rs          # GameState 数据结构
│   ├── disciple.rs      # Disciple 结构
│   ├── martial_art.rs   # MartialArt 结构 (静态数据)
│   ├── event.rs         # GameEvent 结构
│   ├── decision.rs      # Decision 结构 (静态数据)
│   └── tournament.rs    # Tournament 结构
├── logic/
│   ├── mod.rs
│   ├── decision.rs      # 决策执行逻辑
│   ├── event.rs         # 随机事件池 + 触发
│   ├── disciple.rs      # 弟子生成/成长/叛逃
│   ├── tournament.rs    # 论剑计算
│   └── advance.rs       # 月度推进 (编排层)
├── db/
│   ├── mod.rs
│   └── queries.rs       # sqlx 查询函数
└── error.rs             # 错误类型定义
```

---

## 6. 配置管理

### 6.1 环境变量

| 变量 | 说明 | 默认值 |
|------|------|--------|
| `PG_HOST` | PostgreSQL 地址 | `192.168.50.150` |
| `PG_PORT` | PostgreSQL 端口 | `5432` |
| `PG_USER` | 数据库用户 | `ruoruo` |
| `PG_PASSWORD` | 数据库密码 | — |
| `PG_DATABASE` | 数据库名 | `zhangmenriji` |
| `SERVER_HOST` | 后端监听地址 | `0.0.0.0` |
| `SERVER_PORT` | 后端监听端口 | `3000` |
| `DATABASE_URL` | 完整连接串 (优先) | 由上述 PG_* 拼合 |

### 6.2 加载优先级

1. `DATABASE_URL` 环境变量（如设置则直接使用）
2. 各 `PG_*` 环境变量拼合为 `postgres://${PG_USER}:${PG_PASSWORD}@${PG_HOST}:${PG_PORT}/${PG_DATABASE}`
3. `.env` 文件（开发环境）

---

## 7. 部署架构

```
docker-compose.yml (未来)
├── zhangmenriji-backend  (Rust binary, port 3000)
├── zhangmenriji-frontend (Nginx serving static files, port 8080)
└── postgres              (已有，共享实例)
```

前端通过 Nginx 反向代理 `/api/*` 到后端，避免跨域问题。

当前阶段（开发）：
- 后端：`cargo run` 监听 `0.0.0.0:3000`
- 前端：浏览器直接打开 `index.html`，API 指向 `http://localhost:3000`
- CORS：tower-http 允许 `*` origin

---

## 8. 关键设计决策

### ADR-001: 游戏状态存储为 JSONB

**决策**：`games` 表使用单个 `JSONB` 列存储完整游戏状态，而非拆成多表。

**理由**：
- 游戏状态是一个紧密耦合的整体快照，极少需要按字段查询
- JSONB 支持索引，足以覆盖按 id / sect_name 的查询
- 避免 ORM 复杂度和 N+1 问题
- 弟子、事件等数据随游戏状态一同读写，无独立查询场景

### ADR-002: 核心逻辑移至后端

**决策**：决策执行、随机事件、月度推进、论剑计算全部在后端完成。

**理由**：
- 防止前端作弊（虽然这是单人游戏，但保持架构正确性）
- 前端只需渲染，逻辑集中便于维护
- 未来支持多端（手机/桌面）共享同一逻辑

### ADR-003: 前端保持 Vanilla JS

**决策**：不引入 Vue/React 框架。

**理由**：
- 现有 UI 已完成（1278 行），迁移框架成本高收益低
- 纯 HTML 打开即用，零构建步骤
- 前端仅负责渲染 + API 调用，复杂度可控

---

## 9. 安全考量

- SQL 注入防护：sqlx 编译期查询检查 + 参数绑定
- CORS：开发阶段允许所有 origin，生产环境限制为前端域名
- 无用户认证系统：单人游戏，无鉴权需求
- 输入验证：sect_name 限制 1-10 字符，decision_id 白名单校验

---

> 文档版本: v1.0  
> 最后更新: 2026-06-09  
> 架构师: 若若 (RuoRuo) 🐱
