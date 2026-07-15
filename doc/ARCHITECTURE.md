# 《掌门日记》架构设计文档

> 重构版本 v2.1 — Vue 3 前端 · Rust 后端 · Tauri 桌面 · PostgreSQL 持久化

---

## 1. 项目概述

### 1.1 项目简介

《掌门日记》是一款武侠门派经营模拟器。玩家以掌门身份，按月推进时间，管理门派（招募弟子、修炼武学、处理江湖事件），参加年终论剑，最终将三流山寨经营成名震江湖的大派。

### 1.2 重构目标

| v1 (当前) | v2.1 (目标) |
|-----------|----------|
| 纯前端 HTML/JS 单文件 | Vue 3 + Vite + TypeScript 组件化前端 |
| 无后端 | Rust 后端 (axum) |
| LocalStorage 存档 | PostgreSQL 持久化 |
| 所有逻辑在浏览器 | 核心逻辑移至后端 |
| 无配置管理 | 双模式配置：桌面 config.json + Web .env |
| 纯 Web | Tauri 2.x 桌面应用 + Web 双部署 |

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
| **tower-http** | 0.6 | CORS 中间件 + 静态文件服务 (ServeDir) |
| **dotenvy** | 0.15 | .env 加载 |

### 2.2 前端

| 技术 | 版本 | 用途 |
|------|------|------|
| **Vue 3** | 3.x | 渐进式前端框架（Composition API） |
| **Vite** | 6.x | 构建工具 + 开发服务器 |
| **TypeScript** | 5.x | 类型安全 |
| **Tailwind CSS** | v4 | 原子化 CSS 框架 |
| **@tauri-apps/api** | 2.x | Tauri 桌面 API 桥接 |

> 设计决策：v2.0 阶段前端保持 Vanilla JS（1278 行单文件），v2.1 迁移至 Vue 3 组件化架构（16 个 .vue 组件），提升代码可维护性和可复用性。通过 tower-http ServeDir 内嵌服务前端 dist/，单端口部署。

### 2.3 数据库

| 技术 | 版本 | 用途 |
|------|------|------|
| **PostgreSQL** | 18 | 主数据库 |
| **数据库名** | `zhangmenriji` | 项目专用库 |

---

## 3. 系统架构

```
┌───────────────────────────────────────────────────────────────┐
│              Tauri 2.x 桌面壳 (src-tauri/)                     │
│  ┌─────────────────────────────────────────────────────────┐  │
│  │  启动时读取系统配置目录 config.json → 注入 env →         │  │
│  │  启动后端线程 → 加载 WebView 展示前端                    │  │
│  └────────────────────────────┬────────────────────────────┘  │
└───────────────────────────────┼────────────────────────────────┘
                                │
┌───────────────────────────────┼────────────────────────────────┐
│                    浏览器 / WebView (Frontend)                  │
│  ┌─────────────────────────────────────────────────────────┐  │
│  │  Vue 3 SPA (前端 /dist/ )                                │  │
│  │  ├─ TypeScript + Tailwind CSS v4                        │  │
│  │  ├─ 16 个 .vue 组件 (StartScreen, GameView, ...)        │  │
│  │  └─ @tauri-apps/api / Fetch API 调用后端                  │  │
│  └───────────────────────┬─────────────────────────────────┘  │
│                          │  HTTP REST (JSON)                   │
└──────────────────────────┼────────────────────────────────────┘
                           │
┌──────────────────────────┼──────────────────────────────┐
│                     Rust Backend (axum)                   │
│                          ▼                                │
│  ┌───────────────────────────────────────────────────┐  │
│  │  HTTP Layer (tower-http CORS + ServeDir 静态文件)       │  │
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
backend/                          # Rust 后端 (lib + bin 双 target)
├── Cargo.toml                    # [lib] + [[bin]]
├── src/
│   ├── lib.rs                   # pub async fn run_server() — 库入口
│   ├── main.rs                  # CLI bin 入口 → run_server(None)
│   ├── config.rs                # Config::load() / AppConfig 结构体
│   ├── router.rs                # 路由定义 + tower-http ServeDir
│   ├── error.rs                 # AppError 统一错误类型
│   ├── handlers/
│   │   ├── mod.rs
│   │   ├── games.rs             # 游戏 CRUD handler
│   │   ├── decisions.rs         # 决策执行 handler
│   │   ├── advance.rs           # 月度推进 handler
│   │   └── static_data.rs       # /api/decisions + /api/martial-arts
│   ├── models/
│   │   ├── mod.rs
│   │   ├── game.rs              # GameState 等数据结构
│   │   ├── disciple.rs          # Disciple 结构
│   │   ├── martial_art.rs       # MartialArt 静态数据
│   │   ├── event.rs             # GameEvent 结构
│   │   ├── decision.rs          # DecisionDef 静态数据
│   │   └── tournament.rs        # Tournament 结构
│   ├── logic/
│   │   ├── mod.rs
│   │   ├── decision.rs          # 8 种决策执行
│   │   ├── event.rs             # 24 种随机事件池
│   │   ├── disciple.rs          # 弟子生成/成长/叛逃
│   │   ├── tournament.rs        # 论剑计算
│   │   └── advance.rs           # 月度推进编排 (11 步)
│   └── db/
│       └── mod.rs               # sqlx 查询函数 + 自动迁移

frontend/                         # Vue 3 前端
├── package.json                  # npm run dev/build
├── vite.config.ts
├── tsconfig.json
├── index.html                    # SPA 入口
└── src/
    ├── main.ts                   # createApp + mount
    ├── App.vue                   # 根组件
    ├── types.ts                  # TypeScript 类型定义 (含 AppConfig)
    ├── style.css                 # Tailwind CSS v4 入口
    └── components/               # 16 个 .vue 组件
        ├── StartScreen.vue       # 开始/存档选择
        ├── SettingsPanel.vue     # 数据库连接配置
        ├── GameView.vue          # 主游戏视图
        ├── TitleBar.vue          # 顶部状态栏
        ├── StatsGrid.vue         # 门派数据指标
        ├── DecisionGrid.vue      # 决策面板
        ├── DiscipleList.vue      # 弟子列表
        ├── MartialArtsPanel.vue  # 武学面板
        ├── TournamentPanel.vue   # 论剑战绩
        ├── ChroniclesBar.vue     # 事件纪事
        ├── EventPopup.vue        # 事件弹窗
        ├── SavePanel.vue         # 存档管理
        ├── AdvanceSection.vue    # 推进月份
        ├── GameOverScreen.vue    # 游戏结束
        ├── ScrollContainer.vue   # 滚动容器
        └── LoadingOverlay.vue    # 加载遮罩

src-tauri/                        # Tauri 2.x 桌面壳
├── Cargo.toml                    # 依赖 backend (path = "../backend")
├── tauri.conf.json               # 窗口 1080×840, targets=["nsis"]
└── src/
    ├── lib.rs                    # run() — setup 钩子 + Tauri 命令
    └── main.rs                   # 桌面入口
```

---

## 6. 配置管理

### 6.1 双模式概述

项目支持两种配置加载模式，取决于运行环境：

| 模式 | 配置来源 | 适用场景 |
|------|---------|---------|
| **桌面模式** | 系统配置目录 `config.json`（如 `~/.config/zhangmenriji-desktop/config.json`） | Tauri 桌面应用 |
| **Web 模式** | `.env` 文件 + 环境变量 | `cargo run` 开发 / Docker 部署 |

### 6.2 桌面端配置流程

```
Tauri 启动
  ↓
读取系统配置目录 config.json（不存在则创建默认值）
  ↓
AppConfig.apply_to_env() 注入环境变量
  ↓
后端线程通过 env::var() 读取
```

SettingsPanel 组件通过 Tauri 命令 `get_config` / `save_config` 提供 UI 编辑。

### 6.3 环境变量

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

### 6.4 加载优先级

1. `Config::load(config_path)` — JSON 配置文件优先（桌面端）
2. `DATABASE_URL` 环境变量（如设置则直接使用）
3. 各 `PG_*` 环境变量拼合为 `postgres://${PG_USER}:***@${PG_HOST}:${PG_PORT}/${PG_DATABASE}`
4. `.env` 文件（开发环境回退）

---

## 7. 部署架构

### 7.1 Tauri 桌面应用

```
cargo tauri build
  → src-tauri/target/release/bundle/nsis/*.exe (Windows)
  → 单文件安装包，内嵌 WebView + 后端进程
  → 窗口 1080×840，系统托盘可选
```

### 7.2 Web 部署 (Docker Compose)

```
docker-compose.yml
├── zhangmenriji-backend  (Rust binary, port 3000，内嵌前端 dist/)
└── postgres              (已有，共享实例)
```

后端通过 tower-http ServeDir 服务前端静态文件，无需独立 Nginx。`localhost:3000` 直接访问完整应用。

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

### ADR-003: Vanilla JS → Vue 3 组件化迁移

**决策**：将前端从纯 HTML/JS 单文件（1278 行）迁移至 Vue 3 + Vite + TypeScript + Tailwind CSS v4 组件化架构（16 个 .vue 组件）。

**理由**：
- 原单文件 UI 维护成本随功能增长线性上升，缺乏组件级复用
- Vue 3 Composition API + TypeScript 提供类型安全和更好的代码组织
- Vite 提供极快的 HMR 开发体验和优化的生产构建
- Tailwind CSS v4 原子化样式与 Vue 单文件组件天然契合
- 迁移后前端通过 `npm run build` 产出 `frontend/dist/`，由后端 tower-http ServeDir 内嵌服务
- 为 Tauri 桌面应用提供现代化 WebView 前端基础

**权衡**：
- 引入 Node.js 构建工具链（npm + Vite）
- root `package.json` 提供统一脚本入口：`npm run setup/dev/build`

### ADR-004: Tauri 2.x 桌面集成

**决策**：引入 Tauri 2.x 桌面壳（`src-tauri/`），通过 `path = "../backend"` 引用后端 lib 作为依赖，在 setup 钩子中读取系统配置目录 `config.json`、注入环境变量、启动后端线程。

**理由**：
- 提供原生桌面应用体验（安装包、窗口管理）
- 后端 lib crate 可被 Tauri 和 CLI bin 复用，无需重复代码
- 配置持久化到系统标准目录，用户可通过 SettingsPanel 编辑
- NSIS 安装器支持 Windows 分发

**权衡**：
- 增加构建复杂度（Rust 编译 + Tauri 打包）
- 桌面模式下后端作为子线程运行，生命周期受 Tauri 管控

---

## 9. 安全考量

- SQL 注入防护：sqlx 编译期查询检查 + 参数绑定
- CORS：开发阶段允许所有 origin，生产环境限制为前端域名
- 无用户认证系统：单人游戏，无鉴权需求
- 输入验证：sect_name 限制 1-10 字符，decision_id 白名单校验

---

> 文档版本: v2.1  
> 最后更新: 2026-07-16  
> 架构师: 若若 (RuoRuo) 🐱
