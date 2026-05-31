/**
 * Simulation API Service
 * 
 * Provides methods for interacting with the GaussTwin simulation API.
 */

import { api, wsManager } from '../api'

// Types
export interface Simulation {
  id: string
  name: string
  description?: string
  status: SimulationStatus
  config: SimulationConfig
  metrics: SimulationMetrics
  created_at: string
  updated_at: string
}

export type SimulationStatus = 'idle' | 'running' | 'paused' | 'stopped' | 'error'

export interface SimulationConfig {
  max_steps?: number
  time_step: number
  scheduler: string
  seed?: number
}

export interface SimulationMetrics {
  current_step: number
  elapsed_time: number
  agent_count: number
  events_processed: number
  steps_per_second: number
}

export interface CreateSimulationRequest {
  name: string
  description?: string
  config?: Partial<SimulationConfig>
}

export interface UpdateSimulationRequest {
  name?: string
  description?: string
  config?: Partial<SimulationConfig>
}

export interface PaginatedResponse<T> {
  data: T[]
  pagination: {
    page: number
    per_page: number
    total: number
    total_pages: number
  }
}

export interface ApiResponse<T> {
  success: boolean
  data?: T
  error?: string
}

// API endpoints
const SIMULATIONS_ENDPOINT = '/simulations'

/**
 * List all simulations with pagination
 */
export async function listSimulations(
  page: number = 1,
  perPage: number = 10
): Promise<PaginatedResponse<Simulation>> {
  const params = new URLSearchParams({
    page: page.toString(),
    per_page: perPage.toString(),
  })
  const response = await api.get<ApiResponse<PaginatedResponse<Simulation>>>(
    `${SIMULATIONS_ENDPOINT}?${params}`
  )
  if (!response.success || !response.data) {
    throw new Error(response.error || 'Failed to list simulations')
  }
  return response.data
}

/**
 * Get a simulation by ID
 */
export async function getSimulation(id: string): Promise<Simulation> {
  const response = await api.get<ApiResponse<Simulation>>(
    `${SIMULATIONS_ENDPOINT}/${id}`
  )
  if (!response.success || !response.data) {
    throw new Error(response.error || 'Simulation not found')
  }
  return response.data
}

/**
 * Create a new simulation
 */
export async function createSimulation(
  data: CreateSimulationRequest
): Promise<Simulation> {
  const response = await api.post<ApiResponse<Simulation>>(
    SIMULATIONS_ENDPOINT,
    data as Record<string, unknown>
  )
  if (!response.success || !response.data) {
    throw new Error(response.error || 'Failed to create simulation')
  }
  return response.data
}

/**
 * Update a simulation
 */
export async function updateSimulation(
  id: string,
  data: UpdateSimulationRequest
): Promise<Simulation> {
  const response = await api.put<ApiResponse<Simulation>>(
    `${SIMULATIONS_ENDPOINT}/${id}`,
    data as Record<string, unknown>
  )
  if (!response.success || !response.data) {
    throw new Error(response.error || 'Failed to update simulation')
  }
  return response.data
}

/**
 * Delete a simulation
 */
export async function deleteSimulation(id: string): Promise<void> {
  const response = await api.delete<ApiResponse<void>>(
    `${SIMULATIONS_ENDPOINT}/${id}`
  )
  if (!response.success) {
    throw new Error(response.error || 'Failed to delete simulation')
  }
}

/**
 * Start a simulation
 */
export async function startSimulation(id: string): Promise<void> {
  const response = await api.post<ApiResponse<unknown>>(
    `${SIMULATIONS_ENDPOINT}/${id}/start`
  )
  if (!response.success) {
    throw new Error(response.error || 'Failed to start simulation')
  }
}

/**
 * Pause a simulation
 */
export async function pauseSimulation(id: string): Promise<void> {
  const response = await api.post<ApiResponse<unknown>>(
    `${SIMULATIONS_ENDPOINT}/${id}/pause`
  )
  if (!response.success) {
    throw new Error(response.error || 'Failed to pause simulation')
  }
}

/**
 * Stop a simulation
 */
export async function stopSimulation(id: string): Promise<void> {
  const response = await api.post<ApiResponse<unknown>>(
    `${SIMULATIONS_ENDPOINT}/${id}/stop`
  )
  if (!response.success) {
    throw new Error(response.error || 'Failed to stop simulation')
  }
}

/**
 * Execute a single simulation step
 */
export async function stepSimulation(id: string): Promise<{ step: number }> {
  const response = await api.post<ApiResponse<{ step: number }>>(
    `${SIMULATIONS_ENDPOINT}/${id}/step`
  )
  if (!response.success || !response.data) {
    throw new Error(response.error || 'Failed to step simulation')
  }
  return response.data
}

/**
 * Get simulation metrics
 */
export async function getSimulationMetrics(id: string): Promise<SimulationMetrics> {
  const response = await api.get<ApiResponse<SimulationMetrics>>(
    `${SIMULATIONS_ENDPOINT}/${id}/metrics`
  )
  if (!response.success || !response.data) {
    throw new Error(response.error || 'Failed to get metrics')
  }
  return response.data
}

/**
 * Subscribe to real-time simulation updates via WebSocket
 */
export function subscribeToSimulation(
  id: string,
  callbacks: {
    onStatusChange?: (status: SimulationStatus) => void
    onMetricsUpdate?: (metrics: SimulationMetrics) => void
    onError?: (error: string) => void
  }
): () => void {
  const unsubscribers: (() => void)[] = []

  // Subscribe to status changes
  if (callbacks.onStatusChange) {
    unsubscribers.push(
      wsManager.subscribe(`simulation.${id}.status`, (data) => {
        callbacks.onStatusChange!(data as SimulationStatus)
      })
    )
  }

  // Subscribe to metrics updates
  if (callbacks.onMetricsUpdate) {
    unsubscribers.push(
      wsManager.subscribe(`simulation.${id}.metrics`, (data) => {
        callbacks.onMetricsUpdate!(data as SimulationMetrics)
      })
    )
  }

  // Subscribe to errors
  if (callbacks.onError) {
    unsubscribers.push(
      wsManager.subscribe(`simulation.${id}.error`, (data) => {
        callbacks.onError!(data as string)
      })
    )
  }

  // Send subscription request
  wsManager.send('subscribe', {
    topics: [
      `simulation.${id}.status`,
      `simulation.${id}.metrics`,
      `simulation.${id}.error`,
    ],
  })

  // Return cleanup function
  return () => {
    unsubscribers.forEach((unsub) => unsub())
    wsManager.send('unsubscribe', {
      topics: [
        `simulation.${id}.status`,
        `simulation.${id}.metrics`,
        `simulation.${id}.error`,
      ],
    })
  }
}

export const simulationService = {
  list: listSimulations,
  get: getSimulation,
  create: createSimulation,
  update: updateSimulation,
  delete: deleteSimulation,
  start: startSimulation,
  pause: pauseSimulation,
  stop: stopSimulation,
  step: stepSimulation,
  getMetrics: getSimulationMetrics,
  subscribe: subscribeToSimulation,
}
