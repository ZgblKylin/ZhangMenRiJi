<script setup lang="ts">
defineProps<{ open: boolean }>()
const emit = defineEmits<{ close: [] }>()

const versions = [
  {
    version: 'v3.1',
    date: '2026-07-18',
    name: '群侠列传',
    changes: [
      '小说群侠加入世界生成体系，拥有专属身份、称号与江湖纪事',
      '数据库迁移至 SQLite，并完善桌面端数据库路径配置',
      '新增版本日志卷宗，可从开始画面的版本副标题随时查阅',
    ],
  },
  {
    version: 'v3.0',
    date: '2026-07-16',
    name: '万象初显',
    changes: [
      '建立四国二十三派同步运转的完整江湖世界',
      '重做侠客属性、武学修炼与弟子并行行动系统',
      '补全山门经营、门派交流、交互事件与月度结算',
    ],
  },
  {
    version: 'v2.1',
    date: '2026-07-16',
    name: '开朝立代',
    changes: [
      '迁移为 Tauri 2 桌面应用，统一前后端启动与打包流程',
      '后端拆分为 lib/bin 双 target，并加入应用配置持久化',
    ],
  },
  {
    version: 'v2.0',
    date: '2026-07-16',
    name: '重筑山门',
    changes: [
      '前端重构为 Vue 3、Vite、TypeScript 与 Tailwind CSS',
      'Rust 后端增加静态文件服务，支持浏览器直接访问',
    ],
  },
  {
    version: 'v1.1',
    date: '2026-06-10',
    name: '分山立派',
    changes: [
      '完成前后端分离，以 Rust、Axum 与 PostgreSQL 持久化游戏进度',
      '游戏数据与操作迁移至 REST API，并完善事件与论剑流程',
    ],
  },
  {
    version: 'v1.0',
    date: '2026-06-09',
    name: '初入江湖',
    changes: [
      '发布纯前端单文件版本，奠定门派经营核心循环',
      '支持月度决策、弟子培养、年终论剑与 LocalStorage 存档',
    ],
  },
]
</script>

<template>
  <Transition name="fade">
    <div v-if="open" class="modal-overlay z-[200]" @click.self="emit('close')">
      <section class="changelog-dialog" role="dialog" aria-modal="true" aria-labelledby="changelog-title">
        <header class="changelog-header">
          <span aria-hidden="true">录</span>
          <div>
            <h2 id="changelog-title">江 湖 纪 版</h2>
            <p>掌门日记 · 历代版本日志</p>
          </div>
          <button class="changelog-close" type="button" aria-label="关闭版本日志" @click="emit('close')">×</button>
        </header>

        <div class="changelog-scroll">
          <article v-for="release in versions" :key="release.version" class="release-entry">
            <div class="release-heading">
              <strong>{{ release.version }}</strong>
              <h3>{{ release.name }}</h3>
              <time :datetime="release.date">{{ release.date }}</time>
            </div>
            <ul>
              <li v-for="change in release.changes" :key="change">{{ change }}</li>
            </ul>
          </article>
        </div>

        <button class="btn changelog-back" type="button" @click="emit('close')">阅毕收卷</button>
      </section>
    </div>
  </Transition>
</template>

<style scoped>
.changelog-dialog {
  display: flex;
  width: min(720px, 92vw);
  max-height: 86vh;
  flex-direction: column;
  padding: 1.25rem 1.5rem 1rem;
  border: 3px solid var(--color-border);
  background: linear-gradient(100deg, #f1dfbc, #fff9e9 48%, #ead3a8);
  box-shadow: 0 10px 40px #0008;
}
.changelog-header {
  display: grid;
  grid-template-columns: 2.25rem 1fr 2.25rem;
  align-items: center;
  padding-bottom: .65rem;
  border-bottom: 2px solid var(--color-cinnabar);
  text-align: center;
}
.changelog-header > span {
  display: grid;
  width: 1.8rem;
  height: 1.8rem;
  place-items: center;
  border: 2px solid var(--color-cinnabar);
  color: var(--color-cinnabar);
  font: 1rem var(--font-title);
  transform: rotate(-4deg);
}
.changelog-header h2 {
  margin: 0;
  font: 1.25rem var(--font-title);
  letter-spacing: .25em;
}
.changelog-header p {
  margin: .15rem 0 0;
  color: var(--color-ink-fade);
  font-size: .68rem;
  letter-spacing: .12em;
}
.changelog-close {
  border: 0;
  background: transparent;
  color: var(--color-ink-fade);
  cursor: pointer;
  font-size: 1.5rem;
  line-height: 1;
}
.changelog-close:hover,
.changelog-close:focus-visible {
  color: var(--color-cinnabar);
  outline: none;
}
.changelog-scroll {
  min-height: 0;
  overflow-y: auto;
  padding: .4rem .25rem .1rem;
}
.release-entry {
  position: relative;
  padding: .7rem .8rem .7rem 1.1rem;
  border-bottom: 1px dotted var(--color-border);
}
.release-entry::before {
  content: '';
  position: absolute;
  top: 1rem;
  bottom: .75rem;
  left: .25rem;
  width: 2px;
  background: var(--color-cinnabar);
  opacity: .55;
}
.release-heading {
  display: grid;
  grid-template-columns: auto 1fr auto;
  align-items: baseline;
  gap: .55rem;
}
.release-heading strong {
  color: var(--color-cinnabar);
  font: 1rem var(--font-title);
}
.release-heading h3 {
  margin: 0;
  font: 1rem var(--font-title);
  letter-spacing: .12em;
}
.release-heading time {
  color: var(--color-ink-fade);
  font-size: .65rem;
}
.release-entry ul {
  margin: .35rem 0 0;
  padding-left: 1.1rem;
  color: var(--color-ink-light);
  font-size: .74rem;
  line-height: 1.65;
}
.release-entry li::marker {
  color: var(--color-jade);
}
.changelog-back {
  align-self: center;
  margin-top: .8rem;
}
</style>
