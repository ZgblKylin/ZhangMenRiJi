# 《掌门日记》— 武侠门派经营模拟器 v3.1

《掌门日记》是一款按月推进的单机武侠门派经营游戏。当前版本采用 Vue 3 前端、Rust/axum 后端与 SQLite 单文件存档；桌面端由 Tauri 2 打包，浏览器开发模式复用同一套真实 HTTP API，不在前端模拟业务逻辑。

## 当前玩法

- 每月最多执行三项掌门决策；七类旧议事入口已经统一使用 v3 库银、人物武学、师承与修习上限，并可另行安排弟子读书、练功、传授、切磋、营造、游历与任务。
- 外派与游历采用一次性旅程卷宗：出发固定任务、目的地、难度和唯一奇遇，
  归山才按成功、部分成功或失败结算赏银、物资、声望及私人秘籍。
- 管理七座职能建筑、长老堂务、门人位阶、门派令、库存、藏经与外交；建筑等级和完好度形成真实堂效，长老的部门、资质、修为和功绩会进一步改变堂务产出，粮铁欠供会加剧月度损耗。
- 为正式门人安排六部门职司和同门师承；直系师徒优先授业，各部门会为对应修行、经营或外派行动提供 10%～15% 加成。
- 百草堂支持可排序、可暂停的常设炼药队列，按堂效、长老、草药和库银逐月循环开炉。
- 四国、24 个 NPC 势力与固定小说群侠持续经营和行动；繁荣、治安会改变市况、行路历练、NPC 招募与战争支援，贸易、冲突和战争又会反向改写国势。天枢阁可展阅 NPC 掌门及门人的完整只读卷宗。
- 每月另有 3–5 条切磋、叛逃、联盟、朝廷征召与掌门继任等世界交互；跨派切磋会落到真实人物的武学经验、气血和个人声名。双向盟友拥有独立互动池，会以真实人物联合巡行、驰援在途门人，低道德盟友也可能背盟；宋元全国战争每月最多发生一次。
- 门风和经营方针会共同改变自主行动及随机事件权重；邪派牟利更快，但声望损失
  和追缉反噬也更常见。
- 交互大事会暂停当月结算，待掌门选择后继续；普通事件则直接进入月结。
- 每年十二月举行 25 派五轮三阵淘汰制论剑：冻结健康在门的最强三人，保存完整
  签表并为真实交手结算受上限约束的武学经验；当月弹窗、天枢阁历届论剑谱与
  两载结卷均可回看逐场结果。第二届后按真实名次正向结卷并设置 `game_won`，
  封卷状态不可继续经营或推进。

## 技术栈

| 层 | 技术 |
|---|---|
| 前端 | Vue 3、TypeScript、Vite 8、Tailwind CSS 4 |
| 后端 | Rust、axum 0.8、tokio、rayon |
| 数据 | SQLite、sqlx 0.8，完整游戏状态以 JSON 文本存储 |
| 桌面 | Tauri 2 |
| UI 基准 | 1600×900 初始桌面窗口，卷册式三栏布局 |

## 快速开始

需要 Node.js/npm、Rust stable。Linux 构建桌面端还须安装 [Tauri 2 系统依赖](https://v2.tauri.app/zh-cn/start/prerequisites/)；Debian/Ubuntu 可执行：

```bash
sudo apt update
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file \
  libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
```

在仓库根目录安装依赖并启动桌面开发版：

```bash
npm run setup
npm run dev
```

`npm run dev` 会启动 Vite、编译 Tauri/Rust，并打开 1600×900 的桌面窗口。需要用 release 配置观察性能时可运行：

```bash
npm run dev:release
```

### 浏览器开发模式

分别打开两个终端：

```bash
npm run web-backend
npm run web-frontend
```

- 后端与 API：`http://127.0.0.1:3000`
- Vite 开发页面：`http://localhost:5173`

也可以先执行 `npm run build`，再运行 `npm run web-backend`，由 axum 在 3000 端口直接提供 `frontend/dist/`。

## 配置与存档

桌面端和后端 CLI 都从系统用户配置目录的 `com.zhangmenriji.desktop/config.json` 读取配置。首次启动时会自动创建配置，并默认把数据库放到系统用户本地数据目录的 `com.zhangmenriji.desktop/zhangmenriji.db`。

开发环境可在 `backend/.env` 或进程环境中设置：

```dotenv
DATABASE_URL=sqlite:///absolute/path/to/zhangmenriji.db
SERVER_HOST=0.0.0.0
SERVER_PORT=3000
RUST_LOG=info
```

可从 `.env.template` 复制模板。已有 `config.json` 时以文件内容为准；设置页面保存后需重启应用。当前前端游戏 API 固定访问本机 3000 端口，因此桌面/Web 联调应保持 `SERVER_PORT=3000`。

SQLite 启动时自动建表。存档按槽位分组，手动存档和每月自动存档都是独立记录；
每个槽位最多保留最近 10 个自动存档。槽位 head 以单调 revision 统一保护月内
修改、手动存档和完整月结；状态、事件副本、head 与自动档淘汰在同一 SQLite
事务中提交。并发请求只有一个能提交，前端收到 `stale_revision` 后直接接管
服务端权威状态，不重放可能含随机结果的命令。

## 构建与验证

```bash
# TypeScript 类型检查 + 前端生产构建
npm run build

# Tauri 桌面安装包
npm run build:desktop

# Rust 全部目标测试
cd backend
cargo test --all-targets
```

桌面安装包位于 `src-tauri/target/release/bundle/`。

## 项目结构

```text
ZhangMenRiJi/
├── backend/
│   └── src/
│       ├── handlers/       # HTTP 请求/响应与持久化编排
│       ├── logic/          # 月结、行动、经营、世界、事件与论剑规则
│       ├── models/         # 游戏状态、弟子、门派、武学、药物等模型
│       ├── db/             # SQLite 建表、存档与事件查询
│       ├── config.rs       # config.json、环境变量与默认路径
│       ├── router.rs       # REST 路由、CORS 与前端静态文件
│       ├── lib.rs          # Tauri/CLI 共用的 run_server()
│       └── main.rs         # 独立 Web 后端入口
├── frontend/
│   ├── src/
│   │   ├── api.ts          # 真实 REST API 客户端
│   │   ├── store.ts        # Vue 响应式会话状态
│   │   ├── types.ts        # API 与界面类型
│   │   └── components/     # 游戏、经营、门派、事件与存档组件
│   └── dist/               # npm run build 产物
├── src-tauri/              # 桌面壳、窗口配置与后端线程启动
└── doc/                    # 架构、数据库、设计与开发目标
```

## HTTP API

| 方法 | 路径 | 用途 |
|---|---|---|
| `POST` | `/api/games` | 创建门派、槽位和初始世界 |
| `GET` | `/api/games` | 按槽位列出存档，最近优先 |
| `GET` / `DELETE` | `/api/games/:id` | 读取或删除单个存档 |
| `POST` | `/api/games/:id/saves` | 新建手动存档 |
| `DELETE` | `/api/save-groups/:id` | 删除槽位及其全部存档 |
| `POST` | `/api/games/:id/decisions/:decision_id` | 执行掌门决策 |
| `POST` | `/api/games/:id/manage` | 执行经营、人物和外交指令 |
| `POST` | `/api/games/:id/advance` | 推进月份；月结、论剑或两载结卷后自动存档 |
| `POST` | `/api/games/:id/events/resolve` | 处置交互事件并续行当月 |
| `GET` | `/api/decisions` | 获取决策定义 |
| `GET` | `/api/martial-arts` | 获取武学定义 |
| `GET` / `POST` | `/api/config` | 读取或保存后端配置 |

所有修改存档的请求（手动存档、决策、经营、推进、事件续决及两类删除）必须
携带最近响应中的 `X-Save-Revision`；过期请求返回 409 `stale_revision` 及当前
权威状态。单档删除成功时也会返回替代 head，避免其他页面停留在失效会话。

详见 [架构设计](doc/ARCHITECTURE.md)、[数据库设计](doc/DATABASE.md) 与 [完整设计](doc/DESIGN.md)。

## 版本

| 版本 | 日期 | 主要变更 |
|---|---|---|
| v3.1 | 2026-07 | 群侠列传、师徒与六部门、四国国势联动、完整两载结卷、堂效资源链、世界交互及原子存档 |
| v3.0 | 2026-07 | 侠客行式属性、完整门派经营、世界并行行动与交互事件 |
| v2.1 | 2026-07 | Tauri 桌面化，后端拆分为 lib/bin 双入口 |
| v2.0 | 2026-07 | Vue 3 + Vite + TypeScript 组件化前端 |
| v1.x | 2026-06 | 原型期 LocalStorage 与 PostgreSQL 版本 |
