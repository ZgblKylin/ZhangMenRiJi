import type { AdvanceResponse, Decision, DeleteGameResponse, GameResponse, ManageResponse, ManagementRequest, MartialArt, SaveGroup } from './types'

// 浏览器开发环境通过 Vite proxy 访问后端；桌面端/生产构建保留本机后端兼容。
export const API_BASE = import.meta.env.VITE_API_BASE
  || (import.meta.env.DEV ? '/api' : 'http://127.0.0.1:3000/api')

export interface ApiErrorBody {
  code?: string
  error?: string
  message?: string
  current?: GameResponse | null
  [key: string]: unknown
}

export class ApiError<T = unknown> extends Error {
  readonly status: number
  readonly body: T

  constructor(status: number, body: T, message: string) {
    super(message)
    this.name = 'ApiError'
    this.status = status
    this.body = body
  }
}

const errorMessage = (status: number, body: unknown) => {
  if (typeof body === 'string' && body) return body
  if (body && typeof body === 'object') {
    const detail = body as ApiErrorBody
    if (typeof detail.error === 'string' && detail.error) return detail.error
    if (typeof detail.message === 'string' && detail.message) return detail.message
  }
  return `请求失败: ${status}`
}

const revisionHeader = (revision: number | null | undefined) => {
  if (typeof revision !== 'number' || !Number.isSafeInteger(revision) || revision <= 0) {
    const body: ApiErrorBody = {
      code: 'missing_revision',
      error: '后端未提供有效的存档版本标记；为避免覆盖进度，本次操作未发送。请更新或重启后端后重新读档。',
    }
    throw new ApiError(0, body, body.error as string)
  }
  return { 'X-Save-Revision': String(revision) }
}

async function request<T>(
  method: string,
  path: string,
  body?: unknown,
  headers: Record<string, string> = {},
): Promise<T> {
  const response = await fetch(API_BASE + path, {
    method,
    headers: { 'Content-Type': 'application/json', ...headers },
    ...(body === undefined ? {} : { body: JSON.stringify(body) }),
  })
  if (response.status === 204) return null as T
  const text = await response.text()
  const data = text ? (() => { try { return JSON.parse(text) } catch { return text } })() : null
  if (!response.ok) throw new ApiError(response.status, data, errorMessage(response.status, data))
  return data as T
}

const mutate = <T>(path: string, revision: number | null | undefined, body?: unknown) =>
  request<T>('POST', path, body, revisionHeader(revision))

export const isStaleRevisionError = (
  error: unknown,
): error is ApiError<ApiErrorBody> => error instanceof ApiError
  && error.status === 409
  && !!error.body
  && typeof error.body === 'object'
  && (error.body as ApiErrorBody).code === 'stale_revision'

export const isMissingGameError = (
  error: unknown,
): error is ApiError<ApiErrorBody> => error instanceof ApiError
  && error.status === 404
  && !!error.body
  && typeof error.body === 'object'
  && (error.body as ApiErrorBody).code === 'not_found'

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
  list: () => request<{ groups: SaveGroup[] }>('GET', '/games'),
  create: (sectName: string) => request<GameResponse>('POST', '/games', { sect_name: sectName }),
  get: (id: string) => request<GameResponse>('GET', `/games/${id}`),
  save: (id: string, revision: number | null) => mutate<GameResponse>(`/games/${id}/saves`, revision),
  remove: (id: string, revision: number | null) =>
    request<DeleteGameResponse>('DELETE', `/games/${id}`, undefined, revisionHeader(revision)),
  removeGroup: (id: string, revision: number | null) =>
    request<null>('DELETE', `/save-groups/${id}`, undefined, revisionHeader(revision)),
  decide: (id: string, decisionId: string, revision: number | null) =>
    mutate<GameResponse>(`/games/${id}/decisions/${decisionId}`, revision),
  manage: (id: string, command: ManagementRequest, revision: number | null) =>
    mutate<ManageResponse>(`/games/${id}/manage`, revision, command),
  advance: (id: string, revision: number | null) =>
    mutate<AdvanceResponse>(`/games/${id}/advance`, revision),
  resolveEvent: (id: string, optionId: string, revision: number | null) =>
    mutate<AdvanceResponse>(`/games/${id}/events/resolve`, revision, { option_id: optionId }),
}
