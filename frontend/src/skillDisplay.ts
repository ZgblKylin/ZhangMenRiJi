import type { MartialArt, SkillCategory, SkillEntry } from './types'

export const skillCategories: Array<{ id: SkillCategory; label: string; hint: string }> = [
  { id: 'unarmed', label: '拳脚', hint: '拳掌指爪' },
  { id: 'parry', label: '招架', hint: '拆解守御' },
  { id: 'dodge', label: '轻功', hint: '身法步法' },
  { id: 'force', label: '内功', hint: '内力根基' },
  { id: 'weapon', label: '兵器', hint: '本门器械' },
  { id: 'knowledge', label: '知识', hint: '悟性与精力' },
]

export const skillsInCategory = (
  skills: SkillEntry[] | undefined,
  arts: MartialArt[],
  category: SkillCategory,
) => (skills || [])
  .filter(skill => arts.find(art => art.id === skill.martial_art_id)?.category === category)
  .sort((a, b) => b.level - a.level || a.martial_art_id.localeCompare(b.martial_art_id))

export const artName = (arts: MartialArt[], id: string) => arts.find(art => art.id === id)?.name || id
