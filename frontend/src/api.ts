import type { AdvanceResponse, Decision, GameResponse, MartialArt } from './types'

export const API_BASE = '/api'

async function request<T>(method: string, path: string, body?: unknown): Promise<T> {
  const response = await fetch(API_BASE + path, {
    method,
    headers: { 'Content-Type': 'application/json' },
    ...(body === undefined ? {} : { body: JSON.stringify(body) }),
  })
  if (!response.ok) {
    const data = await response.json().catch(() => ({ error: `HTTP ${response.status}` }))
    throw new Error(data.error || `请求失败: ${response.status}`)
  }
  return (response.status === 204 ? null : response.json()) as Promise<T>
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
  advance: (id: string) => request<AdvanceResponse>('POST', `/games/${id}/advance`),
}

