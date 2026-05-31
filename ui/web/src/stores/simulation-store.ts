import { create } from 'zustand'
import { Simulation, SimulationStatus } from '@/types'
import { api } from '@/lib/api'

interface SimulationState {
  simulations: Simulation[]
  selectedSimulation: Simulation | null
  isLoading: boolean
  error: string | null
}

interface SimulationActions {
  fetchSimulations: () => Promise<void>
  fetchSimulation: (id: string) => Promise<void>
  createSimulation: (data: Partial<Simulation>) => Promise<Simulation>
  updateSimulation: (id: string, data: Partial<Simulation>) => Promise<void>
  deleteSimulation: (id: string) => Promise<void>
  startSimulation: (id: string) => Promise<void>
  pauseSimulation: (id: string) => Promise<void>
  stopSimulation: (id: string) => Promise<void>
  setSelectedSimulation: (simulation: Simulation | null) => void
  updateSimulationMetrics: (id: string, metrics: Simulation['metrics']) => void
  clearError: () => void
}

export const useSimulationStore = create<SimulationState & SimulationActions>((set, _get) => ({
  // State
  simulations: [],
  selectedSimulation: null,
  isLoading: false,
  error: null,

  // Actions
  fetchSimulations: async () => {
    set({ isLoading: true, error: null })
    try {
      const response = await api.get<{ simulations: Simulation[] }>('/simulations')
      set({ simulations: response.simulations, isLoading: false })
    } catch (error) {
      set({
        isLoading: false,
        error: error instanceof Error ? error.message : 'Failed to fetch simulations',
      })
    }
  },

  fetchSimulation: async (id: string) => {
    set({ isLoading: true, error: null })
    try {
      const response = await api.get<{ simulation: Simulation }>(`/simulations/${id}`)
      set({ selectedSimulation: response.simulation, isLoading: false })
    } catch (error) {
      set({
        isLoading: false,
        error: error instanceof Error ? error.message : 'Failed to fetch simulation',
      })
    }
  },

  createSimulation: async (data) => {
    set({ isLoading: true, error: null })
    try {
      const response = await api.post<{ simulation: Simulation }>('/simulations', data)
      const newSimulation = response.simulation
      set((state) => ({
        simulations: [...state.simulations, newSimulation],
        isLoading: false,
      }))
      return newSimulation
    } catch (error) {
      set({
        isLoading: false,
        error: error instanceof Error ? error.message : 'Failed to create simulation',
      })
      throw error
    }
  },

  updateSimulation: async (id, data) => {
    set({ isLoading: true, error: null })
    try {
      const response = await api.put<{ simulation: Simulation }>(`/simulations/${id}`, data)
      set((state) => ({
        simulations: state.simulations.map((s) =>
          s.id === id ? response.simulation : s
        ),
        selectedSimulation:
          state.selectedSimulation?.id === id
            ? response.simulation
            : state.selectedSimulation,
        isLoading: false,
      }))
    } catch (error) {
      set({
        isLoading: false,
        error: error instanceof Error ? error.message : 'Failed to update simulation',
      })
      throw error
    }
  },

  deleteSimulation: async (id) => {
    set({ isLoading: true, error: null })
    try {
      await api.delete(`/simulations/${id}`)
      set((state) => ({
        simulations: state.simulations.filter((s) => s.id !== id),
        selectedSimulation:
          state.selectedSimulation?.id === id ? null : state.selectedSimulation,
        isLoading: false,
      }))
    } catch (error) {
      set({
        isLoading: false,
        error: error instanceof Error ? error.message : 'Failed to delete simulation',
      })
      throw error
    }
  },

  startSimulation: async (id) => {
    try {
      await api.post(`/simulations/${id}/start`)
      set((state) => ({
        simulations: state.simulations.map((s) =>
          s.id === id ? { ...s, status: 'running' as SimulationStatus } : s
        ),
        selectedSimulation:
          state.selectedSimulation?.id === id
            ? { ...state.selectedSimulation, status: 'running' as SimulationStatus }
            : state.selectedSimulation,
      }))
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : 'Failed to start simulation',
      })
      throw error
    }
  },

  pauseSimulation: async (id) => {
    try {
      await api.post(`/simulations/${id}/pause`)
      set((state) => ({
        simulations: state.simulations.map((s) =>
          s.id === id ? { ...s, status: 'paused' as SimulationStatus } : s
        ),
        selectedSimulation:
          state.selectedSimulation?.id === id
            ? { ...state.selectedSimulation, status: 'paused' as SimulationStatus }
            : state.selectedSimulation,
      }))
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : 'Failed to pause simulation',
      })
      throw error
    }
  },

  stopSimulation: async (id) => {
    try {
      await api.post(`/simulations/${id}/stop`)
      set((state) => ({
        simulations: state.simulations.map((s) =>
          s.id === id ? { ...s, status: 'stopped' as SimulationStatus } : s
        ),
        selectedSimulation:
          state.selectedSimulation?.id === id
            ? { ...state.selectedSimulation, status: 'stopped' as SimulationStatus }
            : state.selectedSimulation,
      }))
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : 'Failed to stop simulation',
      })
      throw error
    }
  },

  setSelectedSimulation: (simulation) => {
    set({ selectedSimulation: simulation })
  },

  updateSimulationMetrics: (id, metrics) => {
    set((state) => ({
      simulations: state.simulations.map((s) =>
        s.id === id ? { ...s, metrics } : s
      ),
      selectedSimulation:
        state.selectedSimulation?.id === id
          ? { ...state.selectedSimulation, metrics }
          : state.selectedSimulation,
    }))
  },

  clearError: () => set({ error: null }),
}))
