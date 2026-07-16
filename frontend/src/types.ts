export type Mood = 'good' | 'bad' | 'neutral'
export type ActionKind = 'read' | 'practice' | 'teach' | 'spar' | 'temper_body' | 'cultivate_neili' | 'meditate' | 'sect_mission' | 'wander' | 'recover'
export type SectPolicy = 'balanced' | 'martial' | 'scholarly' | 'chivalrous' | 'mercantile' | 'reclusive'
export type DiscipleRank = 'chore' | 'outer' | 'inner' | 'elder'
export type Department = 'transmission' | 'library' | 'apothecary' | 'treasury' | 'stewardship' | 'external_affairs'

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
  desc: string
  atk: number
  def: number
  spd: number
  req_talent?: number
  sect_id?: string | null
  basic_skill?: string
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
}
export interface SkillProgress { level: number; experience: number }
export interface MartialProgress {
  proficiencies: Record<string, SkillProgress>
  specialties: string[]
  private_books: string[]
}
export interface Disciple {
  id: string
  sect_id?: string | null
  name: string
  alive: boolean
  age: number
  talent: number
  inner_power: number
  loyalty: number
  martial_art: string
  aptitudes: Aptitudes
  attributes: AcquiredAttributes
  condition: 'healthy' | 'exhausted' | 'unconscious' | 'seriously_injured' | 'dead'
  rank: DiscipleRank
  merit: number
  department?: Department | null
  martial_progress: MartialProgress
  action?: ActionPlan | null
  away_months: number
}

export interface Building { id: string; name: string; level: number; condition: number; upgrading_months: number }
export interface SectOrder { id: string; name: string; remaining_months: number; silver_cost: number; effect: Record<string, number> }
export interface SectAttributes { prestige: number; silver: number; morality: number; morale: number }
export interface SectState {
  id: string
  name: string
  country_id: string
  player_controlled: boolean
  attributes: SectAttributes
  policy: SectPolicy
  buildings: Building[]
  inventory: Record<string, number>
  public_books: string[]
  martial_research: Record<string, number>
  relations: Record<string, number>
  active_orders: SectOrder[]
}
export interface Country { id: string; name: string; prosperity: number; order: number }

export interface EventEffect { [key: string]: unknown }
export interface EventChoice { id: string; label: string; result_text: string; effect: EventEffect; good: boolean }
export interface PendingWorldEvent { id: string; category: string; title: string; text: string; choices: EventChoice[] }
export interface ChronicleEvent { year?: number; month?: number; text: string; mood: Mood }
export interface Tournament { year?: number; rank: number; total_sects: number; power: number; desc_text?: string }

export interface GameState {
  sect_name: string
  schema_version: number
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
export interface GameResponse { id: string; sect_name?: string; state: GameState; updated_at?: string }
export interface AdvanceResponse { state: GameState; events?: ChronicleEvent[]; tournament?: Tournament; game_over?: boolean }
export interface ManageResponse { state: GameState; events?: ChronicleEvent[] }

export interface SaveSlot {
  id: string
  sect_name: string
  year: number
  month: number
  prestige: number
  silver: number
  disciples: number
  updated_at?: string
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
