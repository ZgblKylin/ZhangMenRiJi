import { MedicineType } from './types'

export type MedicineRate = '常速' | '低速' | '极低速'

export interface MedicineDefinition {
  recipeId: string
  name: MedicineType
  description: string
  herbCost: number
  quantity: number
  months: number
  rate: MedicineRate
}

export const medicines: MedicineDefinition[] = [
  { recipeId: 'wound', name: MedicineType.Wound, description: '治疗气血，恢复 30 点气血。', herbCost: 4, quantity: 2, months: 1, rate: '常速' },
  { recipeId: 'qi', name: MedicineType.Qi, description: '调息养气，恢复 25 点内力。', herbCost: 6, quantity: 1, months: 2, rate: '常速' },
  { recipeId: 'spirit', name: MedicineType.Spirit, description: '澄心清神，恢复 30 点精神。', herbCost: 5, quantity: 2, months: 1, rate: '常速' },
  { recipeId: 'energy', name: MedicineType.Energy, description: '补益元精，恢复 25 点精力。', herbCost: 6, quantity: 1, months: 2, rate: '常速' },
  { recipeId: 'foundation', name: MedicineType.Foundation, description: '培本固元，永久提升 10 点气血上限。', herbCost: 12, quantity: 1, months: 6, rate: '低速' },
  { recipeId: 'gather_qi', name: MedicineType.GatherQi, description: '聚纳真气，永久提升 5 点内力上限。', herbCost: 12, quantity: 1, months: 6, rate: '低速' },
  { recipeId: 'calm_spirit', name: MedicineType.CalmSpirit, description: '安魂宁神，永久提升 10 点精神上限。', herbCost: 12, quantity: 1, months: 6, rate: '低速' },
  { recipeId: 'restore_origin', name: MedicineType.RestoreOrigin, description: '补全元精，永久提升 5 点精力上限。', herbCost: 12, quantity: 1, months: 6, rate: '低速' },
  { recipeId: 'marrow', name: MedicineType.Marrow, description: '洗髓易骨，永久提升 1 点根骨。', herbCost: 20, quantity: 1, months: 12, rate: '极低速' },
  { recipeId: 'sinew', name: MedicineType.Sinew, description: '强筋壮骨，永久提升 1 点膂力。', herbCost: 20, quantity: 1, months: 12, rate: '极低速' },
  { recipeId: 'awaken', name: MedicineType.Awaken, description: '灵台开窍，永久提升 1 点悟性。', herbCost: 20, quantity: 1, months: 12, rate: '极低速' },
  { recipeId: 'lightness', name: MedicineType.Lightness, description: '身轻步健，永久提升 1 点身法。', herbCost: 20, quantity: 1, months: 12, rate: '极低速' },
  { recipeId: 'longevity', name: MedicineType.Longevity, description: '延年益寿，使结算年龄永久降低 1 岁（最低 15 岁）。', herbCost: 24, quantity: 1, months: 12, rate: '极低速' },
]

const medicineByName = new Map<string, MedicineDefinition>(medicines.map(medicine => [medicine.name, medicine]))

export const medicineDescription = (name: string) => medicineByName.get(name)?.description || ''
export const issuableItems = ['草药', ...medicines.map(medicine => medicine.name)]
