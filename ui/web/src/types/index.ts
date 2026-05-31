// Simulation Types
export type SimulationStatus = 'running' | 'paused' | 'stopped' | 'completed' | 'error'

export interface Simulation {
  id: string
  name: string
  description: string
  status: SimulationStatus
  agentCount: number
  currentStep: number
  totalSteps: number
  startedAt: string
  updatedAt: string
  config: SimulationConfig
  metrics?: SimulationMetrics
}

export interface SimulationConfig {
  spaceType: 'grid' | 'continuous' | 'graph'
  dimensions: number[]
  agentTypes: AgentTypeConfig[]
  scheduler: 'sequential' | 'parallel' | 'priority'
  maxSteps: number
  timeStep: number
}

export interface SimulationMetrics {
  cpuUsage: number
  memoryUsage: number
  gpuUsage?: number
  stepsPerSecond: number
  eventsPerSecond: number
  elapsedTime: number
}

// Agent Types
export type AgentArchitecture = 'bdi' | 'cognitive' | 'reactive' | 'custom'

export interface AgentType {
  id: string
  name: string
  description: string
  architecture: AgentArchitecture
  parameters: AgentParameter[]
  behaviors: string[]
}

export interface AgentTypeConfig {
  typeId: string
  count: number
  parameters: Record<string, unknown>
}

export interface AgentParameter {
  name: string
  type: 'number' | 'string' | 'boolean' | 'array' | 'object'
  default?: unknown
  min?: number
  max?: number
  description?: string
}

export interface Agent {
  id: string
  typeId: string
  state: Record<string, unknown>
  position: Position
  velocity?: Velocity
  active: boolean
}

// Space Types
export type SpaceType = 'grid' | 'continuous' | 'graph'

export interface Position {
  x: number
  y: number
  z?: number
}

export interface Velocity {
  dx: number
  dy: number
  dz?: number
}

export interface Space {
  id: string
  type: SpaceType
  dimensions: number[]
  config: SpaceConfig
}

export interface SpaceConfig {
  wrapping?: boolean
  maxAgentsPerCell?: number
  spatialIndex?: 'kdtree' | 'gridhash' | 'rtree'
}

// Analytics Types
export interface AnalyticsData {
  timestamp: string
  metrics: MetricPoint[]
}

export interface MetricPoint {
  name: string
  value: number
  unit?: string
}

export interface Anomaly {
  id: string
  type: string
  severity: 'info' | 'warning' | 'error'
  simulationId: string
  simulationName: string
  description: string
  detectedAt: string
}

// API Types
export interface ApiResponse<T> {
  success: boolean
  data: T
  meta?: {
    page?: number
    pageSize?: number
    total?: number
    requestId?: string
  }
}

export interface ApiError {
  code: string
  message: string
  details?: Record<string, unknown>
}

export interface PaginationParams {
  page: number
  pageSize: number
  sortBy?: string
  sortOrder?: 'asc' | 'desc'
}

// User Types
export interface User {
  id: string
  email: string
  name: string
  avatar?: string
  role: 'admin' | 'user' | 'viewer'
  permissions: string[]
  createdAt: string
}

// WebSocket Types
export interface WsMessage<T = unknown> {
  type: string
  payload: T
  timestamp: string
}

export interface SimulationUpdate {
  simulationId: string
  currentStep: number
  agentCount: number
  metrics: SimulationMetrics
}

export interface AgentUpdate {
  agentId: string
  simulationId: string
  state: Record<string, unknown>
  position: Position
}
