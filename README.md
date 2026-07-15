# 《掌门日记》— 武侠门派经营模拟器 v2.0

前后端分离架构，Rust 后端 + Vue 3 前端，PostgreSQL 持久化，Tauri 桌面应用打包。

## 玩法简介

- **核心循环**：以月为单位推进时间，每月做决策
- **经营目标**：将三流山寨经营成名震江湖的大派
- **年终论剑**：每年十二月举行，检验门派实力

## 技术架构

| 层 | 技术 |
|----|------|
| 前端 | Vue 3 + Vite + TypeScript + Tailwind CSS |
| 后端 | Rust + axum 0.8 |
| 数据库 | PostgreSQL 18 |
| ORM | sqlx 0.8 |
| 桌面 | Tauri 2.x |

## 快速启动

### 环境准备

#### 1. 安装系统依赖（仅 Tauri 桌面模式需要）

Linux (Debian/Ubuntu)：
```bash
sudo apt install libgtk-3-dev libwebkit2gtk-4.1-dev \
  libayatana-appindicator3-dev librsvg2-dev
```

Windows / macOS：无需额外系统依赖，Tauri 会自动处理。

#### 2. 安装 Node.js 依赖

```bash
# 前端依赖（Vue / Vite / Tailwind）
cd frontend && npm install

# 根目录依赖（Tauri CLI）
cd .. && npm install
```

> `npx tauri` 会自动使用根目录 `node_modules` 中的 `@tauri-apps/cli`。

#### 3. 数据库

```bash
# 在 PostgreSQL 中创建数据库（已建则跳过）
createdb -U ruoruo -h 192.168.50.150 zhangmenriji
```

### 2. 配置环境变量

```bash
# 复制模板并填入实际值
cp .env.template backend/.env
```

编辑 `backend/.env`，填入数据库连接信息：

```env
PG_HOST=192.168.50.150
PG_PORT=5432
PG_USER=ruoruo
PG_PASSWORD=你的密码
PG_DATABASE=zhangmenriji
SERVER_HOST=0.0.0.0
SERVER_PORT=3000
RUST_LOG=info
```

### 3. 启动开发环境

#### 方式 A：Web 开发（推荐调试时使用）

```bash
# 终端 1：启动后端
cd backend && cargo run
# 监听 http://0.0.0.0:3000

# 终端 2：启动前端（热更新）
cd frontend && npm run dev
# 浏览器打开 http://localhost:5173（Vite 代理至后端）
```

#### 方式 B：Tauri 桌面应用

```bash
# 首次需要编译 src-tauri（约 2-5 分钟），后续增量编译很快
npx tauri dev
```

> **常见问题**：
> - 命令是 `tauri` 不是 `tarui`（注意拼写）
> - 如果报 `cargo` 找不到：安装 [Rust](https://rustup.rs)
> - 如果报 `libgtk-3` 找不到：回到「环境准备 → 系统依赖」安装

### 4. 构建

```bash
# 构建前端
cd frontend && npm run build

# 构建桌面应用
npx tauri build
```

构建产物位于 `src-tauri/target/release/bundle/`。

## 项目结构

```
ZhangMenRiJi/
├── frontend/               # Vue 3 前端
│   ├── src/
│   │   ├── App.vue         # 主组件 + 游戏状态管理
│   │   ├── api.ts          # API 调用层
│   │   ├── store.ts        # 响应式状态
│   │   ├── types.ts        # TypeScript 类型
│   │   └── components/     # 15 个 Vue 组件
│   ├── dist/               # 构建产物（npm run build 生成）
│   └── package.json
├── backend/                # Rust axum 后端
│   └── src/
│       ├── lib.rs          # run_server() 入口
│       ├── main.rs         # CLI 入口
│       ├── handlers/       # API 处理器 + 游戏逻辑
│       └── router.rs       # 路由 + 静态文件服务
├── src-tauri/              # Tauri 桌面应用
│   ├── src/lib.rs          # 后台线程启动 axum，然后 Tauri 接管
│   ├── tauri.conf.json     # 窗口配置 (1080×840)
│   └── Cargo.toml          # 依赖 backend crate
├── doc/                    # 设计文档
└── .env.template           # 环境变量模板
```

## API 端点

| 方法 | 路径 | 说明 |
|------|------|------|
| POST | `/api/games` | 创建新游戏 |
| GET | `/api/games` | 列出存档 |
| GET | `/api/games/:id` | 获取游戏状态 |
| DELETE | `/api/games/:id` | 删除存档 |
| POST | `/api/games/:id/decisions/:decision_id` | 执行决策 |
| POST | `/api/games/:id/advance` | 推进月份 |
| GET | `/api/decisions` | 决策静态数据 |
| GET | `/api/martial-arts` | 武学静态数据 |

## 设计文档

- [架构设计](doc/ARCHITECTURE.md)
- [数据库设计](doc/DATABASE.md)
- [完整设计文档](doc/DESIGN.md)

## 📦 仓库地址

| 平台 | 地址 |
|------|------|
| 🏠 Gitea（主） | http://192.168.50.150:33000/RuoRuo/ZhangMenRiJi |
| ☁️ GitHub（镜像） | https://github.com/ZgblKylin/ZhangMenRiJi |

## 版本历史

| 版本 | 日期 | 变更 |
|------|------|------|
| v2.1 | 2026-07 | Tauri 桌面应用迁移，后端拆分为 lib/bin 双 target |
| v2.0 | 2026-07 | Vue 3 + Vite + Tailwind 前端重构，后端添加静态文件服务 |
| v1.1 | 2026-06 | 前后端分离，Rust 后端，PostgreSQL 持久化 |
| v1.0 | 2026-06 | 纯前端 HTML 单文件，LocalStorage 存档 |
