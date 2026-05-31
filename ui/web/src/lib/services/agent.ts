/**
 * Agent API Service
 * 
 * Provides methods for interacting with the GaussTwin agent API.
 */

import { api, wsManager } from '../api'
import type { ApiResponse, PaginatedResponse } from './simulation'

// Types
export interface Agent {
  id: string
  simulation_id: string
  agent_type: string
  state: Record<string, unknown>
  position?: Position
  created_at: string
}

export interface Position {
  x: number
  y: number
  z?: number
}

export interface CreateAgentRequest {
  agent_type: string
  state?: Record<string, unknown>
  position?: Position
}

export interface AgentQueryParams {
  agent_type?: string
  page?: number
  per_page?: number
}

/**
 * List agents in a simulation
 */
export async function listAgents(
  simulationId: string,
  params: AgentQueryParams = {}
): Promise<PaginatedResponse<Agent>> {
  const searchParams = new URLSearchParams()
  if (params.page) searchParams.set('page', params.page.toString())
  if (params.per_page) searchParams.set('per_page', params.per_page.toString())
  if (params.agent_type) searchParams.set('agent_type', params.agent_type)

  const response = await api.get<ApiResponse<PaginatedResponse<Agent>>>(
    `/simulations/${simulationId}/agents?${searchParams}`
  )
  if (!response.success || !response.data) {
    throw new Error(response.error || 'Failed to list agents')
  }
  return response.data
}

/**
 * Get an agent by ID
 */
export async function getAgent(
  simulationId: string,
  agentId: string
): Promise<Agent> {
  const response = await api.get<ApiResponse<Agent>>(
    `/simulations/${simulationId}/agents/${agentId}`
  )
  if (!response.success || !response.data) {
    throw new Error(response.error || 'Agent not found')
  }
  return response.data
}

/**
 * Create an agent in a simulation
 */
export async function createAgent(
  simulationId: string,
  data: CreateAgentRequest
): Promise<Agent> {
  const response = await api.post<ApiResponse<Agent>>(
    `/simulations/${simulationId}/agents`,
    data as Record<string, unknown>
  )
  if (!response.success || !response.data) {
    throw new Error(response.error || 'Failed to create agent')
  }
  return response.data
}

/**
 * Delete an agent
 */
export async function deleteAgent(
  simulationId: string,
  agentId: string
): Promise<void> {
  const response = await api.delete<ApiResponse<void>>(
    `/simulations/${simulationId}/agents/${agentId}`
  )
  if (!response.success) {
    throw new Error(response.error || 'Failed to delete agent')
  }
}

/**
 * Subscribe to agent updates via WebSocket
 */
export function subscribeToAgentUpdates(
  simulationId: string,
  callback: (agents: Agent[]) => void
): () => void {
  const unsubscribe = wsManager.subscribe(`simulation.${simulationId}.agents`, (data) => {
    callback(data as Agent[])
  })

  wsManager.send('subscribe', {
    topics: [`simulation.${simulationId}.agents`],
  })

  return () => {
    unsubscribe()
    wsManager.send('unsubscribe', {
      topics: [`simulation.${simulationId}.agents`],
    })
  }
}

export const agentService = {
  list: listAgents,
  get: getAgent,
  create: createAgent,
  delete: deleteAgent,
  subscribe: subscribeToAgentUpdates,
}
