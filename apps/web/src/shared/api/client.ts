const API_BASE_URL = import.meta.env.VITE_API_BASE_URL ?? 'http://localhost:8080'

interface ErrorBody {
  error?: string
  message?: string
}

export class ApiError extends Error {
  readonly status: number
  /** The backend's typed error code (`DroplyError` variant, snake_case) when available. */
  readonly code?: string

  constructor(message: string, status: number, code?: string) {
    super(message)
    this.name = 'ApiError'
    this.status = status
    this.code = code
  }
}

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(`${API_BASE_URL}${path}`, init)

  if (!response.ok) {
    const body = (await response.json().catch(() => null)) as ErrorBody | null
    throw new ApiError(
      body?.message ?? `${init?.method ?? 'GET'} ${path} failed with ${response.status}`,
      response.status,
      body?.error,
    )
  }

  if (response.status === 204) {
    return undefined as T
  }

  return (await response.json()) as T
}

export function apiGet<T>(path: string): Promise<T> {
  return request<T>(path)
}

export function apiPost<T>(path: string, body?: unknown): Promise<T> {
  return request<T>(path, {
    method: 'POST',
    headers: body === undefined ? undefined : { 'content-type': 'application/json' },
    body: body === undefined ? undefined : JSON.stringify(body),
  })
}

/** For building direct navigation/download URLs (e.g. content links) that
 * aren't `fetch` calls the app makes itself. */
export function apiUrl(path: string): string {
  return `${API_BASE_URL}${path}`
}
