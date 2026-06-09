# 《掌门日记》— 武侠门派经营模拟器 v2.0

前后端分离架构，Rust 后端 + 纯 HTML/CSS/JS 前端，PostgreSQL 持久化。

## 玩法简介

- **核心循环**：以月为单位推进时间，每月做决策
- **经营目标**：将三流山寨经营成名震江湖的大派
- **年终论剑**：每年十二月举行，检验门派实力

## 技术架构

| 层 | 技术 |
|----|------|
| 前端 | 纯 HTML/CSS/JS（武侠宣纸风） |
| 后端 | Rust + axum 0.8 |
| 数据库 | PostgreSQL 18 |
| ORM | sqlx 0.8 |

## 快速启动

### 1. 数据库

```bash
# 在 PostgreSQL 中创建数据库（已建则跳过）
createdb -U ruoruo -h 192.168.50.150 zhangmenriji
```

### 2. 后端

```bash
cd backend
cp ../.env.template .env
# 编辑 .env 填入实际数据库连接信息
cargo run
# 监听 http://0.0.0.0:3000
```

### 3. 前端

浏览器直接打开 `index.html`，API 自动指向 `localhost:3000`。

## API 端点

| 方法 | 路径 | 说明 |
|------|------|------|
| POST | `/api/games` | 创建新游戏 |
| GET | `/api/games` | 列出存档 |
| GET | `/api/games/:id` | 获取游戏状态 |
| DELETE | `/api/games/:id` | 删除存档 |
| POST | `/api/games/:id/decisions/:decision_id` | 执行决策 |
| POST | `/api/games/:id/advance` | 推进月份 |

## 设计文档

- [架构设计](doc/ARCHITECTURE.md)
- [数据库设计](doc/DATABASE.md)

## 版本历史

| 版本 | 日期 | 变更 |
|------|------|------|
| v2.0 | 2026-06 | 前后端分离，Rust 后端，PostgreSQL 持久化 |
| v1.0 | 2026-06 | 纯前端 HTML 单文件，LocalStorage 存档 |
