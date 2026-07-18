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
      '新增玩家师徒、NPC 师承谱系与六部门个人职司',
      '七堂等级、完好度、粮铁月修和常设炼药队列形成完整经营链',
      '四国繁荣、治安联动经济、历练、NPC 招募与宋元战争',
      '外派与游历新增旅程卷宗、成败验收、唯一奇遇、物资和私人秘籍',
      '门风接入自主行动、外派奖惩与随机事件权重，邪派牟利伴随声望及反噬代价',
'掌门自创武学：藏经阁中亲定名称、门类与根基，堂效与长老悟性决定武学层次',
      '辈分传承体系：师门辈分链追踪、传承核心武学标记、辈分与忠诚驱动的修炼加成',
      '部门晋升自动化：弟子在六部门任上累积功绩，达阈值即触发品阶擢升',
      'NPC门派深化：自动武学研究、百草堂炼药、建筑修缮与自然磨损',
      '四国人口初值与商路繁荣联动，季令与江湖杂事扩充至六十余条',
      '去除两载封卷限制，年关纪事取代终局，三连三甲+声望500即威震江湖胜利',
      '盟约录新增联巡、援手、论道、易货四项主动署令，可遣使者与列盟互动',
      '初始库银、粮草与声望提高，首月开篇引导纪事助掌门上手',
      '跨门派切磋真实结算双方武学经验、气血与胜者个人声望',
      'NPC 门派可展阅掌门兼任头衔、完整属性、行动旅程与六类武学只读卷宗',
      '长老的部门、资质、修为和功绩会与堂效共同决定堂务产出',
      '旧版督课、推演、传武与委托议事统一写入真实 v3 武学和门派资源',
      '双向盟友可联合巡行、驰援在途门人，低道德势力也可能背盟毁约',
      '年终论剑扩展为25派五轮三阵淘汰赛，保存完整签表并结算真实武学成长',
      '第二届论剑后按真实名次完成两载结卷',
      'SQLite 存档以槽位 revision/CAS 防止并发覆写，状态、事件和自动档裁剪原子提交',
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
