export type Mood = 'good' | 'bad' | 'neutral'

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
  atk: number
  def: number
  spd: number
}

export interface Disciple {
  name: string
  alive: boolean
  talent: number
  inner_power: number
  loyalty: number
  martial_art: string
}

export interface ChronicleEvent {
  year?: number
  month?: number
  text: string
  mood: Mood
}

export interface Tournament {
  year?: number
  rank: number
  total_sects: number
  power: number
  desc_text?: string
}

export interface GameState {
  sect_name: string
  year: number
  month: number
  prestige: number
  silver: number
  morale: number
  injury: number
  max_decisions: number
  disciples: Disciple[]
  martial_arts_learned: string[]
  event_log: ChronicleEvent[]
  tournament_history: Tournament[]
  game_over: boolean
  game_over_reason?: string
}

export interface GameResponse { id: string; sect_name?: string; state: GameState; updated_at?: string }
export interface AdvanceResponse { state: GameState; events?: ChronicleEvent[]; tournament?: Tournament }

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

/** Tauri 桌面端 — 应用配置（与 Rust AppConfig 对应） */
export interface AppConfig {
  pg_host: string
  pg_port: string
  pg_user: string
  pg_password: string
  pg_database: string
  server_host: string
  server_port: string
}

