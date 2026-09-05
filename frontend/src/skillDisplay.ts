import { SkillCategory } from './types'
import type { MartialArt, SkillEntry } from './types'

export const skillCategories: Array<{ id: SkillCategory; label: string; hint: string }> = [
  { id: SkillCategory.Unarmed, label: '拳脚', hint: '拳掌指爪' },
  { id: SkillCategory.Parry, label: '招架', hint: '可择拳脚兵器为招架式' },
  { id: SkillCategory.Dodge, label: '轻功', hint: '身法步法' },
  { id: SkillCategory.Force, label: '内功', hint: '内力根基' },
  { id: SkillCategory.Weapon, label: '兵器', hint: '本门器械' },
  { id: SkillCategory.Knowledge, label: '知识', hint: '悟性与精力' },
]

export const skillsInCategory = (
  skills: SkillEntry[] | undefined,
  arts: MartialArt[],
  category: SkillCategory,
) => (skills || [])
  .filter(skill => arts.find(art => art.id === skill.martial_art_id)?.category === category)
  .sort((a, b) => b.level - a.level || a.martial_art_id.localeCompare(b.martial_art_id))

export const artName = (arts: MartialArt[], id: string) => arts.find(art => art.id === id)?.name || id

/**
 * Static martial arts describe the rules shared by every game. Player-created
 * arts belong to the current save, so merge them at the view boundary and let
 * the save copy win if an id ever overlaps.
 */
export const mergeMartialArts = (arts: MartialArt[], created: MartialArt[] = []) => {
  const merged = new Map(arts.map(art => [art.id, art]))
  for (const art of created) merged.set(art.id, art)
  return [...merged.values()]
}
