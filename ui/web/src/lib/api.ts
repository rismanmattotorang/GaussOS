import { useAuthStore } from '@/stores/auth-store'

const API_BASE_URL = '/api/v1'

interface RequestOptions extends Omit<RequestInit, 'body'> {
  body?: BodyInit | null
}

interface ApiError {
  message: string
  code: string
  details?: Record<string, unknown>
}

export class ApiClient {
  private baseUrl: string

  constructor(baseUrl: string = API_BASE_URL) {
    this.baseUrl = baseUrl
  }

  private getHeaders(): HeadersInit {
    const headers: HeadersInit = {
      'Content-Type': 'application/json',
    }

    const token = useAuthStore.getState().accessToken
    if (token) {
      headers['Authorization'] = `Bearer ${token}`
    }

    return headers
  }

  private async handleResponse<T>(response: Response): Promise<T> {
    if (!response.ok) {
      const error: ApiError = await response.json().catch(() => ({
        message: response.statusText,
        code: `HTTP_${response.status}`,
      }))

      if (response.status === 401) {
        useAuthStore.getState().logout()
      }

      throw new Error(error.message || 'An error occurred')
    }

    if (response.status === 204) {
      return {} as T
    }

    return response.json()
  }

  async get<T>(endpoint: string, options?: RequestOptions): Promise<T> {
    const response = await fetch(`${this.baseUrl}${endpoint}`, {
      ...options,
      method: 'GET',
      headers: { ...this.getHeaders(), ...options?.headers },
    })
    return this.handleResponse<T>(response)
  }

  async post<T>(endpoint: string, data?: Record<string, unknown>, options?: RequestOptions): Promise<T> {
    const response = await fetch(`${this.baseUrl}${endpoint}`, {
      ...options,
      method: 'POST',
      headers: { ...this.getHeaders(), ...options?.headers },
      body: data ? JSON.stringify(data) : undefined,
    })
    return this.handleResponse<T>(response)
  }

  async put<T>(endpoint: string, data?: Record<string, unknown>, options?: RequestOptions): Promise<T> {
    const response = await fetch(`${this.baseUrl}${endpoint}`, {
      ...options,
      method: 'PUT',
      headers: { ...this.getHeaders(), ...options?.headers },
      body: data ? JSON.stringify(data) : undefined,
    })
    return this.handleResponse<T>(response)
  }

  async patch<T>(endpoint: string, data?: Record<string, unknown>, options?: RequestOptions): Promise<T> {
    const response = await fetch(`${this.baseUrl}${endpoint}`, {
      ...options,
      method: 'PATCH',
      headers: { ...this.getHeaders(), ...options?.headers },
      body: data ? JSON.stringify(data) : undefined,
    })
    return this.handleResponse<T>(response)
  }

  async delete<T>(endpoint: string, options?: RequestOptions): Promise<T> {
    const response = await fetch(`${this.baseUrl}${endpoint}`, {
      ...options,
      method: 'DELETE',
      headers: { ...this.getHeaders(), ...options?.headers },
    })
    return this.handleResponse<T>(response)
  }
}

export const api = new ApiClient()

// WebSocket connection manager
export class WebSocketManager {
  private ws: WebSocket | null = null
  private reconnectAttempts = 0
  private maxReconnectAttempts = 5
  private reconnectDelay = 1000
  private listeners: Map<string, Set<(data: unknown) => void>> = new Map()

  connect(url: string): void {
    const token = useAuthStore.getState().accessToken
    const wsUrl = `${url}${token ? `?token=${token}` : ''}`

    this.ws = new WebSocket(wsUrl)

    this.ws.onopen = () => {
      console.log('WebSocket connected')
      this.reconnectAttempts = 0
    }

    this.ws.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data)
        const { type, payload } = data
        const typeListeners = this.listeners.get(type)
        if (typeListeners) {
          typeListeners.forEach(listener => listener(payload))
        }
      } catch (e) {
        console.error('Failed to parse WebSocket message:', e)
      }
    }

    this.ws.onclose = () => {
      console.log('WebSocket disconnected')
      this.reconnect(url)
    }

    this.ws.onerror = (error) => {
      console.error('WebSocket error:', error)
    }
  }

  private reconnect(url: string): void {
    if (this.reconnectAttempts < this.maxReconnectAttempts) {
      this.reconnectAttempts++
      const delay = this.reconnectDelay * Math.pow(2, this.reconnectAttempts - 1)
      console.log(`Reconnecting in ${delay}ms... (attempt ${this.reconnectAttempts})`)
      setTimeout(() => this.connect(url), delay)
    }
  }

  subscribe(type: string, listener: (data: unknown) => void): () => void {
    if (!this.listeners.has(type)) {
      this.listeners.set(type, new Set())
    }
    this.listeners.get(type)!.add(listener)

    return () => {
      this.listeners.get(type)?.delete(listener)
    }
  }

  send(type: string, payload: unknown): void {
    if (this.ws?.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify({ type, payload }))
    }
  }

  disconnect(): void {
    this.ws?.close()
    this.ws = null
  }
}

export const wsManager = new WebSocketManager()
