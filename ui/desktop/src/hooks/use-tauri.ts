/**
 * Tauri integration hooks for the desktop application
 */

import { useState, useEffect, useCallback } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { listen, UnlistenFn } from '@tauri-apps/api/event'
import { open, save } from '@tauri-apps/plugin-dialog'
import { readTextFile, writeTextFile } from '@tauri-apps/plugin-fs'
import { sendNotification, isPermissionGranted, requestPermission } from '@tauri-apps/plugin-notification'

// Types
export interface Simulation {
  id: string
  name: string
  description: string
  status: string
  agent_count: number
  current_step: number
  total_steps: number
  created_at: string
  updated_at: string
  config: Record<string, unknown>
}

export interface RecentFile {
  path: string
  name: string
  last_opened: string
  pinned: boolean
}

export interface AppSettings {
  theme: string
  language: string
  auto_save: boolean
  auto_save_interval: number
  check_updates_on_start: boolean
  start_with_system: boolean
  minimize_to_tray: boolean
  show_notifications: boolean
  api_endpoint: string
  max_recent_files: number
  editor_font_size: number
  animation_enabled: boolean
}

export interface SystemInfo {
  os: string
  os_version: string
  arch: string
  hostname: string
  cpu_cores: number
  memory_total: number
}

export interface AppPaths {
  data_dir: string
  cache_dir: string
  log_dir: string
  config_dir: string
}

// Check if running in Tauri
export const isTauri = (): boolean => {
  return typeof window !== 'undefined' && '__TAURI__' in window
}

// Simulation hooks
export function useSimulations() {
  const [simulations, setSimulations] = useState<Simulation[]>([])
  const [isLoading, setIsLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const fetchSimulations = useCallback(async () => {
    if (!isTauri()) return
    setIsLoading(true)
    try {
      const result = await invoke<Simulation[]>('list_simulations')
      setSimulations(result)
      setError(null)
    } catch (e) {
      setError(e as string)
    } finally {
      setIsLoading(false)
    }
  }, [])

  const createSimulation = useCallback(async (data: { name: string; description?: string; config: Record<string, unknown> }) => {
    if (!isTauri()) return null
    const result = await invoke<Simulation>('create_simulation', { request: data })
    setSimulations(prev => [...prev, result])
    return result
  }, [])

  const updateSimulation = useCallback(async (id: string, data: Partial<Simulation>) => {
    if (!isTauri()) return null
    const result = await invoke<Simulation>('update_simulation', { id, request: data })
    setSimulations(prev => prev.map(s => s.id === id ? result : s))
    return result
  }, [])

  const deleteSimulation = useCallback(async (id: string) => {
    if (!isTauri()) return
    await invoke('delete_simulation', { id })
    setSimulations(prev => prev.filter(s => s.id !== id))
  }, [])

  const startSimulation = useCallback(async (id: string) => {
    if (!isTauri()) return
    await invoke('start_simulation', { id })
    setSimulations(prev => prev.map(s => s.id === id ? { ...s, status: 'running' } : s))
  }, [])

  const pauseSimulation = useCallback(async (id: string) => {
    if (!isTauri()) return
    await invoke('pause_simulation', { id })
    setSimulations(prev => prev.map(s => s.id === id ? { ...s, status: 'paused' } : s))
  }, [])

  const stopSimulation = useCallback(async (id: string) => {
    if (!isTauri()) return
    await invoke('stop_simulation', { id })
    setSimulations(prev => prev.map(s => s.id === id ? { ...s, status: 'stopped' } : s))
  }, [])

  useEffect(() => {
    fetchSimulations()
  }, [fetchSimulations])

  return {
    simulations,
    isLoading,
    error,
    fetchSimulations,
    createSimulation,
    updateSimulation,
    deleteSimulation,
    startSimulation,
    pauseSimulation,
    stopSimulation,
  }
}

// File operations hook
export function useFileOperations() {
  const [recentFiles, setRecentFiles] = useState<RecentFile[]>([])

  const fetchRecentFiles = useCallback(async () => {
    if (!isTauri()) return
    const files = await invoke<RecentFile[]>('get_recent_files')
    setRecentFiles(files)
  }, [])

  const openFileDialog = useCallback(async () => {
    if (!isTauri()) return null
    
    const selected = await open({
      multiple: false,
      filters: [{
        name: 'GaussTwin Simulation',
        extensions: ['gausstwin', 'gts', 'json']
      }]
    })

    if (selected) {
      const content = await readTextFile(selected as string)
      await invoke('open_file', { path: selected })
      await fetchRecentFiles()
      return { path: selected as string, content }
    }
    return null
  }, [fetchRecentFiles])

  const saveFileDialog = useCallback(async (content: string, defaultName?: string) => {
    if (!isTauri()) return null

    const path = await save({
      defaultPath: defaultName,
      filters: [{
        name: 'GaussTwin Simulation',
        extensions: ['gausstwin', 'gts', 'json']
      }]
    })

    if (path) {
      await writeTextFile(path, content)
      await invoke('save_file', { path, content })
      await fetchRecentFiles()
      return path
    }
    return null
  }, [fetchRecentFiles])

  const clearRecentFiles = useCallback(async () => {
    if (!isTauri()) return
    await invoke('clear_recent_files')
    setRecentFiles([])
  }, [])

  useEffect(() => {
    fetchRecentFiles()
  }, [fetchRecentFiles])

  return {
    recentFiles,
    openFileDialog,
    saveFileDialog,
    clearRecentFiles,
    fetchRecentFiles,
  }
}

// Settings hook
export function useSettings() {
  const [settings, setSettings] = useState<AppSettings | null>(null)
  const [isLoading, setIsLoading] = useState(false)

  const fetchSettings = useCallback(async () => {
    if (!isTauri()) return
    setIsLoading(true)
    try {
      const result = await invoke<AppSettings>('get_settings')
      setSettings(result)
    } finally {
      setIsLoading(false)
    }
  }, [])

  const updateSettings = useCallback(async (updates: Partial<AppSettings>) => {
    if (!isTauri()) return null
    const result = await invoke<AppSettings>('update_settings', { request: updates })
    setSettings(result)
    return result
  }, [])

  const resetSettings = useCallback(async () => {
    if (!isTauri()) return null
    const result = await invoke<AppSettings>('reset_settings')
    setSettings(result)
    return result
  }, [])

  useEffect(() => {
    fetchSettings()
  }, [fetchSettings])

  return {
    settings,
    isLoading,
    updateSettings,
    resetSettings,
  }
}

// System info hook
export function useSystemInfo() {
  const [systemInfo, setSystemInfo] = useState<SystemInfo | null>(null)
  const [appPaths, setAppPaths] = useState<AppPaths | null>(null)

  useEffect(() => {
    if (!isTauri()) return

    const fetchInfo = async () => {
      const info = await invoke<SystemInfo>('get_system_info')
      setSystemInfo(info)

      const paths = await invoke<AppPaths>('get_app_paths')
      setAppPaths(paths)
    }

    fetchInfo()
  }, [])

  return { systemInfo, appPaths }
}

// Notifications hook
export function useNotifications() {
  const [permissionGranted, setPermissionGranted] = useState(false)

  useEffect(() => {
    if (!isTauri()) return

    const checkPermission = async () => {
      const granted = await isPermissionGranted()
      setPermissionGranted(granted)
    }

    checkPermission()
  }, [])

  const requestNotificationPermission = useCallback(async () => {
    if (!isTauri()) return false
    const permission = await requestPermission()
    const granted = permission === 'granted'
    setPermissionGranted(granted)
    return granted
  }, [])

  const notify = useCallback(async (title: string, body?: string) => {
    if (!isTauri() || !permissionGranted) return
    await sendNotification({ title, body })
  }, [permissionGranted])

  return {
    permissionGranted,
    requestNotificationPermission,
    notify,
  }
}

// Menu events hook
export function useMenuEvents() {
  useEffect(() => {
    if (!isTauri()) return

    let unlisten: UnlistenFn

    const setup = async () => {
      unlisten = await listen<string>('menu-action', (event) => {
        const action = event.payload
        window.dispatchEvent(new CustomEvent('tauri-menu-action', { detail: action }))
      })
    }

    setup()

    return () => {
      unlisten?.()
    }
  }, [])
}

// File watcher hook
export function useFileWatcher(path: string | null) {
  const [changes, setChanges] = useState<{ paths: string[]; kind: string }[]>([])

  useEffect(() => {
    if (!isTauri() || !path) return

    let unlisten: UnlistenFn

    const setup = async () => {
      await invoke('watch_directory', { path })

      unlisten = await listen<{ paths: string[]; kind: string }>('file-changed', (event) => {
        setChanges(prev => [...prev, event.payload])
      })
    }

    setup()

    return () => {
      unlisten?.()
      invoke('unwatch_directory', { path })
    }
  }, [path])

  const clearChanges = useCallback(() => {
    setChanges([])
  }, [])

  return { changes, clearChanges }
}

// Update checker hook
export function useUpdateChecker() {
  const [updateAvailable, setUpdateAvailable] = useState(false)
  const [updateInfo, setUpdateInfo] = useState<{
    currentVersion: string
    latestVersion?: string
    releaseNotes?: string
  } | null>(null)

  const checkForUpdates = useCallback(async () => {
    if (!isTauri()) return null

    const result = await invoke<{
      available: boolean
      current_version: string
      latest_version?: string
      release_notes?: string
    }>('check_for_updates')

    setUpdateAvailable(result.available)
    setUpdateInfo({
      currentVersion: result.current_version,
      latestVersion: result.latest_version,
      releaseNotes: result.release_notes,
    })

    return result
  }, [])

  return {
    updateAvailable,
    updateInfo,
    checkForUpdates,
  }
}
