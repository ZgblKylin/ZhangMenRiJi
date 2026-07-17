export type Mood = 'good' | 'bad' | 'neutral'
export type ActionKind = 'read' | 'practice' | 'teach' | 'spar' | 'temper_body' | 'cultivate_neili' | 'meditate' | 'sect_mission' | 'wander' | 'recover' | 'maintain' | 'construct' | 'produce' | 'business' | 'gather'
export type SectPolicy = 'balanced' | 'martial' | 'scholarly' | 'chivalrous' | 'mercantile' | 'reclusive'
export type MoralDirection = 'righteous' | 'neutral' | 'villainous'
export type DiscipleRank = 'chore' | 'outer' | 'inner'
export type BuildingKind = 'practice' | 'scripture' | 'warehouse' | 'herb_hall' | 'intelligence' | 'affairs' | 'logistics'
export type Department = 'transmission' | 'library' | 'apothecary' | 'treasury' | 'stewardship' | 'external_affairs'
export const MedicineType = {
  Wound: '金疮药',
  Qi: '养气丹',
  Spirit: '清神散',
  Energy: '回精丸',
  Foundation: '培元丹',
  GatherQi: '聚气丹',
  CalmSpirit: '宁神丹',
  RestoreOrigin: '回天丹',
  Marrow: '洗髓丹',
  Sinew: '强筋丹',
  Awaken: '开窍丹',
  Lightness: '轻身丹',
  Longevity: '延寿丹',
} as const
export type MedicineType = typeof MedicineType[keyof typeof MedicineType]
export const SkillCategory = {
  Unarmed: 'unarmed',
  Parry: 'parry',
  Dodge: 'dodge',
  Force: 'force',
  Weapon: 'weapon',
  Knowledge: 'knowledge',
} as const
export type SkillCategory = typeof SkillCategory[keyof typeof SkillCategory]
export const MartialTier = {
  Basic: 'basic',
  Chore: 'chore',
  Outer: 'outer',
  Inner: 'inner',
} as const
export type MartialTier = typeof MartialTier[keyof typeof MartialTier]

export interface Decision {
  id: string
  title: string
  desc: string
  cost: number
  cost_type?: string
  costType: string
}

export interface MartialArt {
  id: string
  name: string
  type: string
  art_type?: string
  category: SkillCategory
  tier: MartialTier
  is_combat: boolean
  desc: string
  atk: number
  def: number
  spd: number
  req_talent?: number
  sect_id?: string | null
  basic_skill?: string
  usable_for_parry?: boolean
  difficulty?: number
}

export interface ResourcePool { current: number; maximum: number }
export interface Aptitudes { strength: number; intelligence: number; constitution: number; agility: number; fortune: number }
export interface AcquiredAttributes {
  qi: ResourcePool
  spirit: ResourcePool
  neili: ResourcePool
  energy: ResourcePool
  attainment: number
  reputation: number
  morality: number
  sect_loyalty: number
}
export interface ActionPlan {
  kind: ActionKind
  target_id?: string | null
  martial_art_id?: string | null
  assigned_by?: string | null
  remaining_months: number
  rations_claimed?: boolean
}
export interface SkillProgress { level: number; experience: number }
export interface SkillEntry { martial_art_id: string; level: number; experience: number }
export interface MartialProgress {
  proficiencies: Record<string, SkillProgress>
  specialties: string[]
  private_books: string[]
}
export interface Disciple {
  id: string
  sect_id?: string | null
  origin_sect_id?: string | null
  name: string
  alive: boolean
  age: number
  talent: number
  inner_power: number
  loyalty: number
  martial_art: string
  prepared_skills?: Record<string, string>
  aptitudes: Aptitudes
  attributes: AcquiredAttributes
  attribute_bonuses?: { qi: number; spirit: number; neili: number; energy: number }
  martial_schema_version?: number
  condition: 'healthy' | 'exhausted' | 'unconscious' | 'seriously_injured' | 'dead'
  rank: DiscipleRank
  merit: number
  department?: Department | null
  skills: SkillEntry[]
  martial_progress: MartialProgress
  action?: ActionPlan | null
  away_months: number
  personal_silver: number
  personal_rations: number
}

export interface Building {
  id: string
  name: string
  kind: BuildingKind
  level: number
  condition: number
  upgrading_months: number
  elder_id?: string | null
  elder_title: string
  selected_duty?: string | null
  duty_target?: string | null
  elder_action_used: boolean
  work_required: number
  work_invested: number
}
export interface RankRules { outer_ratio: number; inner_ratio: number }
export interface SectOrder { id: string; name: string; remaining_months: number; silver_cost: number; effect: Record<string, number> }
export interface ProductionTask { id: string; name: string; output_item: string; quantity: number; remaining_months: number }
export interface SectAttributes { prestige: number; silver: number; morality: number; morale: number }
export interface SectState {
  id: string
  name: string
  description: string
  landmark: string
  country_id: string
  player_controlled: boolean
  attributes: SectAttributes
  policy: SectPolicy
  moral_direction: MoralDirection
  rank_rules: RankRules
  buildings: Building[]
  inventory: Record<string, number>
  public_books: string[]
  martial_research: Record<string, number>
  relations: Record<string, number>
  active_orders: SectOrder[]
  productions: ProductionTask[]
  auto_brew_index: number
  auto_brew_progress: number
}
export interface Country { id: string; name: string; prosperity: number; order: number }

export interface EventEffect { [key: string]: unknown }
export interface EventChoice { id: string; label: string; result_text: string; effect: EventEffect; good: boolean }
export interface PendingWorldEvent { id: string; category: string; title: string; text: string; choices: EventChoice[] }
export interface ChronicleEvent { year?: number; month?: number; text: string; mood: Mood; category?: 'sect' | 'world' }
export interface Tournament { year?: number; rank: number; total_sects: number; power: number; desc_text?: string }

export interface GameState {
  sect_name: string
  schema_version: number
  autosave?: boolean
  year: number
  month: number
  prestige: number
  silver: number
  morale: number
  injury: number
  decisions_used: number
  max_decisions: number
  disciples: Disciple[]
  martial_arts_learned: string[]
  event_log: ChronicleEvent[]
  tournament_history: Tournament[]
  game_over: boolean
  game_over_reason?: string
  pending_event?: PendingWorldEvent | null
  sect: SectState
  npc_sects: SectState[]
  npc_disciples: Disciple[]
  countries: Country[]
}

export interface ManagementRequest { action: string; [key: string]: unknown }
export interface GameResponse { id: string; save_group_id: string; sect_name?: string; state: GameState; updated_at?: string }
export interface AdvanceResponse { id: string; state: GameState; events?: ChronicleEvent[]; sect_events?: ChronicleEvent[]; world_events?: ChronicleEvent[]; tournament?: Tournament; game_over?: boolean }
export interface ManageResponse { state: GameState; events?: ChronicleEvent[] }

export interface SaveRecord {
  id: string
  save_group_id: string
  save_type: 'auto' | 'manual'
  autosave: boolean
  year: number
  month: number
  updated_at: string
}

export interface SaveGroup {
  save_group_id: string
  sect_name: string
  year: number
  month: number
  updated_at: string
  saves: SaveRecord[]
}

export interface AppConfig {
  pg_host: string
  pg_port: string
  pg_user: string
  pg_password: string
  pg_database: string
  server_host: string
  server_port: string
}
