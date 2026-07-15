# 《掌门日记》— 武侠门派经营模拟器 v3.0

前后端分离架构，Rust 后端 + Vue 3 前端，PostgreSQL 持久化，Tauri 桌面应用打包。

## 玩法简介

- **核心循环**：以月为单位推进时间，每月做决策
- **经营目标**：将三流山寨经营成名震江湖的大派
- **年终论剑**：每年十二月举行，检验门派实力
- **侠客行式修行**：膂力、悟性、根骨、身法、福源，以及气血、精神、内力、精力与造诣
- **江湖大势**：四国二十三派同步行动，弟子每月依快照并行推演
- **山门经营**：营造、人事、门派令、物资、藏经研究、门派交流皆可由掌门定夺
- **交互事件**：江湖、生计、门内、朝廷与奇遇事件会暂缓月令，待掌门选策后续行

## 技术架构

| 层 | 技术 |
|----|------|
| 前端 | Vue 3 + Vite + TypeScript + Tailwind CSS |
| 后端 | Rust + axum 0.8 |
| 数据库 | PostgreSQL 18 |
| ORM | sqlx 0.8 |
| 桌面 | Tauri 2.x |

## 快速启动

Linux 桌面端须先备齐 [Tauri 2 官方系统依赖](https://v2.tauri.app/zh-cn/start/prerequisites/)。Debian/Ubuntu 可执行：

```bash
sudo apt update
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file \
  libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
```

只需两条命令：

```bash
# 1. 安装所有依赖（根目录 + 前端，约 30 秒）
npm run setup

# 2. 启动桌面应用（自动编译前端 + Rust 后端 + 打开窗口）
npm run dev
```

> `npm run dev` 等同于 `npx tauri dev`，会自动：前端构建 → Rust 编译 → 后端启动 → 打开桌面窗口。
> 首次编译约 2-5 分钟，后续增量编译很快。

### 配置数据库

**桌面应用**：启动后点击开始画面的 ⚙️ 按钮，在弹出的设置面板中填入数据库连接信息。

**Web 开发模式**（浏览器调试，无需 Tauri）：
```bash
cp .env.template backend/.env   # 编辑填入数据库信息
npm run web-backend             # 终端 1：后端 → http://localhost:3000
npm run web-frontend            # 终端 2：前端 → http://localhost:5173
```

### 构建安装包

```bash
npm run build   # 等同于 npx tauri build
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
| POST | `/api/games/:id/manage` | 执行门派经营 |
| POST | `/api/games/:id/events/resolve` | 处置交互事件并续行月份 |
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
| v3.0 | 2026-07 | 侠客行式属性、二十三派世界、并行行动、完整经营与交互事件 |
| v2.1 | 2026-07 | Tauri 桌面应用迁移，后端拆分为 lib/bin 双 target |
| v2.0 | 2026-07 | Vue 3 + Vite + Tailwind 前端重构，后端添加静态文件服务 |
| v1.1 | 2026-06 | 前后端分离，Rust 后端，PostgreSQL 持久化 |
| v1.0 | 2026-06 | 纯前端 HTML 单文件，LocalStorage 存档 |
