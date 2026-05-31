/**
 * Space API Service
 * 
 * Provides methods for interacting with the GaussTwin space/spatial API.
 */

import { api, wsManager } from '../api'
import type { ApiResponse } from './simulation'
import type { Agent, Position } from './agent'

// Types
export type SpaceType = 'grid' | 'continuous' | 'graph'

export interface Space {
  id: string
  simulation_id: string
  space_type: SpaceType
  bounds: Bounds
  agent_count: number
}

export interface Bounds {
  min: Position
  max: Position
}

export type SpatialQueryType = 'radius_search' | 'nearest_neighbors' | 'agents_at'

export interface SpatialQuery {
  query_type: SpatialQueryType
  position?: Position
  radius?: number
  k?: number
}

/**
 * Get space information for a simulation
 */
export async function getSpace(simulationId: string): Promise<Space> {
  const response = await api.get<ApiResponse<Space>>(
    `/simulations/${simulationId}/space`
  )
  if (!response.success || !response.data) {
    throw new Error(response.error || 'Space not found')
  }
  return response.data
}

/**
 * Query agents in space
 */
export async function querySpace(
  simulationId: string,
  query: SpatialQuery
): Promise<Agent[]> {
  const response = await api.post<ApiResponse<Agent[]>>(
    `/simulations/${simulationId}/space/query`,
    query as Record<string, unknown>
  )
  if (!response.success || !response.data) {
    throw new Error(response.error || 'Failed to query space')
  }
  return response.data
}

/**
 * Find agents within a radius of a position
 */
export async function radiusSearch(
  simulationId: string,
  position: Position,
  radius: number
): Promise<Agent[]> {
  return querySpace(simulationId, {
    query_type: 'radius_search',
    position,
    radius,
  })
}

/**
 * Find k nearest neighbors to a position
 */
export async function nearestNeighbors(
  simulationId: string,
  position: Position,
  k: number
): Promise<Agent[]> {
  return querySpace(simulationId, {
    query_type: 'nearest_neighbors',
    position,
    k,
  })
}

/**
 * Find agents at a specific position
 */
export async function agentsAt(
  simulationId: string,
  position: Position
): Promise<Agent[]> {
  return querySpace(simulationId, {
    query_type: 'agents_at',
    position,
  })
}

/**
 * Subscribe to space updates via WebSocket
 */
export function subscribeToSpaceUpdates(
  simulationId: string,
  callback: (space: Space) => void
): () => void {
  const unsubscribe = wsManager.subscribe(`simulation.${simulationId}.space`, (data) => {
    callback(data as Space)
  })

  wsManager.send('subscribe', {
    topics: [`simulation.${simulationId}.space`],
  })

  return () => {
    unsubscribe()
    wsManager.send('unsubscribe', {
      topics: [`simulation.${simulationId}.space`],
    })
  }
}

export const spaceService = {
  get: getSpace,
  query: querySpace,
  radiusSearch,
  nearestNeighbors,
  agentsAt,
  subscribe: subscribeToSpaceUpdates,
}
