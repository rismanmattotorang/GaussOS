/**
 * GraphQL Client Service
 * 
 * Provides a lightweight GraphQL client for the GaussTwin API.
 */

import { useAuthStore } from '@/stores/auth-store'

const GRAPHQL_ENDPOINT = '/graphql'

interface GraphQLResponse<T> {
  data?: T
  errors?: Array<{
    message: string
    path?: string[]
    extensions?: Record<string, unknown>
  }>
}

/**
 * Execute a GraphQL query or mutation
 */
export async function graphql<T = unknown>(
  query: string,
  variables?: Record<string, unknown>
): Promise<T> {
  const token = useAuthStore.getState().accessToken

  const response = await fetch(GRAPHQL_ENDPOINT, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      ...(token ? { Authorization: `Bearer ${token}` } : {}),
    },
    body: JSON.stringify({
      query,
      variables,
    }),
  })

  const result: GraphQLResponse<T> = await response.json()

  if (result.errors && result.errors.length > 0) {
    throw new Error(result.errors[0].message)
  }

  if (!result.data) {
    throw new Error('No data returned from GraphQL')
  }

  return result.data
}

// ============================================================================
// Twin Queries
// ============================================================================

export interface Twin {
  id: string
  name: string
  description?: string
  created_at: string
  updated_at: string
  status: 'Active' | 'Inactive' | 'Maintenance' | 'Error'
}

export interface Metric {
  name: string
  value: number
  unit?: string
}

export interface SimulationResult {
  id: string
  twin_id: string
  timestamp: string
  status: string
  metrics: Metric[]
}

/**
 * Get health status
 */
export async function getHealth(): Promise<string> {
  const data = await graphql<{ health: string }>(`
    query Health {
      health
    }
  `)
  return data.health
}

/**
 * Get server version
 */
export async function getVersion(): Promise<string> {
  const data = await graphql<{ version: string }>(`
    query Version {
      version
    }
  `)
  return data.version
}

/**
 * Get a twin by ID
 */
export async function getTwin(id: string): Promise<Twin | null> {
  const data = await graphql<{ twin: Twin | null }>(`
    query GetTwin($id: ID!) {
      twin(id: $id) {
        id
        name
        description
        created_at
        updated_at
        status
      }
    }
  `, { id })
  return data.twin
}

/**
 * List twins with pagination
 */
export async function listTwins(
  limit: number = 10,
  offset: number = 0,
  status?: Twin['status']
): Promise<Twin[]> {
  const data = await graphql<{ twins: Twin[] }>(`
    query ListTwins($limit: Int, $offset: Int, $status: TwinStatus) {
      twins(limit: $limit, offset: $offset, status: $status) {
        id
        name
        description
        created_at
        updated_at
        status
      }
    }
  `, { limit, offset, status })
  return data.twins
}

/**
 * Search twins by name
 */
export async function searchTwins(query: string, limit: number = 10): Promise<Twin[]> {
  const data = await graphql<{ searchTwins: Twin[] }>(`
    query SearchTwins($query: String!, $limit: Int) {
      searchTwins(query: $query, limit: $limit) {
        id
        name
        description
        created_at
        updated_at
        status
      }
    }
  `, { query, limit })
  return data.searchTwins
}

/**
 * Get simulation results for a twin
 */
export async function getSimulationResults(
  twinId: string,
  limit: number = 10
): Promise<SimulationResult[]> {
  const data = await graphql<{ simulationResults: SimulationResult[] }>(`
    query GetSimulationResults($twinId: ID!, $limit: Int) {
      simulationResults(twinId: $twinId, limit: $limit) {
        id
        twin_id
        timestamp
        status
        metrics {
          name
          value
          unit
        }
      }
    }
  `, { twinId, limit })
  return data.simulationResults
}

// ============================================================================
// Twin Mutations
// ============================================================================

export interface CreateTwinInput {
  name: string
  description?: string
}

export interface UpdateTwinInput {
  id: string
  name?: string
  description?: string
  status?: Twin['status']
}

/**
 * Create a new twin
 */
export async function createTwin(input: CreateTwinInput): Promise<Twin> {
  const data = await graphql<{ createTwin: Twin }>(`
    mutation CreateTwin($input: CreateTwinInput!) {
      createTwin(input: $input) {
        id
        name
        description
        created_at
        updated_at
        status
      }
    }
  `, { input })
  return data.createTwin
}

/**
 * Update a twin
 */
export async function updateTwin(input: UpdateTwinInput): Promise<Twin> {
  const data = await graphql<{ updateTwin: Twin }>(`
    mutation UpdateTwin($input: UpdateTwinInput!) {
      updateTwin(input: $input) {
        id
        name
        description
        created_at
        updated_at
        status
      }
    }
  `, { input })
  return data.updateTwin
}

/**
 * Delete a twin
 */
export async function deleteTwin(id: string): Promise<boolean> {
  const data = await graphql<{ deleteTwin: boolean }>(`
    mutation DeleteTwin($id: ID!) {
      deleteTwin(id: $id)
    }
  `, { id })
  return data.deleteTwin
}

/**
 * Start a simulation for a twin
 */
export async function startSimulation(twinId: string): Promise<SimulationResult> {
  const data = await graphql<{ startSimulation: SimulationResult }>(`
    mutation StartSimulation($twinId: ID!) {
      startSimulation(twinId: $twinId) {
        id
        twin_id
        timestamp
        status
        metrics {
          name
          value
          unit
        }
      }
    }
  `, { twinId })
  return data.startSimulation
}

export const graphqlService = {
  query: graphql,
  getHealth,
  getVersion,
  getTwin,
  listTwins,
  searchTwins,
  getSimulationResults,
  createTwin,
  updateTwin,
  deleteTwin,
  startSimulation,
}
