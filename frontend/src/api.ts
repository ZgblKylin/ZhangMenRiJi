import type { AdvanceResponse, Decision, GameResponse, ManageResponse, ManagementRequest, MartialArt } from './types'

// Tauri 桌面端内嵌的后端固定监听本机 3000 端口。
export const API_BASE = 'http://127.0.0.1:3000/api'

async function request<T>(method: string, path: string, body?: unknown): Promise<T> {
  const response = await fetch(API_BASE + path, {
    method,
    headers: { 'Content-Type': 'application/json' },
    ...(body === undefined ? {} : { body: JSON.stringify(body) }),
  })
  if (response.status === 204) return null as T
  const text = await response.text()
  const data = text ? (() => { try { return JSON.parse(text) } catch { return text } })() : null
  if (!response.ok) throw new Error(typeof data === 'string' ? data : data?.error || `请求失败: ${response.status}`)
  return data as T
}

export const gameApi = {
  staticData: async () => {
    const [decisions, arts] = await Promise.all([
      request<{ decisions: Decision[] }>('GET', '/decisions'),
      request<{ arts: MartialArt[] }>('GET', '/martial-arts'),
    ])
    return {
      decisions: (decisions.decisions || []).map(d => ({ ...d, costType: d.cost_type || d.costType })),
      arts: (arts.arts || []).map(a => ({ ...a, type: a.art_type || a.type })),
    }
  },
  list: () => request<{ games: GameResponse[] }>('GET', '/games'),
  create: (sectName: string) => request<GameResponse>('POST', '/games', { sect_name: sectName }),
  get: (id: string) => request<GameResponse>('GET', `/games/${id}`),
  remove: (id: string) => request<null>('DELETE', `/games/${id}`),
  decide: (id: string, decisionId: string) => request<{ state: GameResponse['state'] }>('POST', `/games/${id}/decisions/${decisionId}`),
  manage: (id: string, command: ManagementRequest) => request<ManageResponse>('POST', `/games/${id}/manage`, command),
  advance: (id: string) => request<AdvanceResponse>('POST', `/games/${id}/advance`),
  resolveEvent: (id: string, optionId: string) => request<AdvanceResponse>('POST', `/games/${id}/events/resolve`, { option_id: optionId }),
}
