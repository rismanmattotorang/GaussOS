import { Suspense, lazy, useEffect } from 'react'
import { Routes, Route, Navigate, useNavigate } from 'react-router-dom'
import { Toaster } from '@web/components/ui/toaster'
import { ThemeProvider } from '@web/components/theme-provider'
import { AuthProvider } from '@web/features/auth/auth-provider'
import { ProtectedRoute } from '@web/features/auth/protected-route'
import { LoadingScreen } from '@web/components/loading-screen'
import { ErrorBoundary } from '@web/components/error-boundary'
import { isTauri, useSettings, useUpdateChecker, useNotifications } from './hooks/use-tauri'

// Lazy load pages (reuse from web UI)
const LoginPage = lazy(() => import('@web/pages/auth/login'))
const RegisterPage = lazy(() => import('@web/pages/auth/register'))
const DashboardPage = lazy(() => import('@web/pages/dashboard'))
const SimulationsPage = lazy(() => import('@web/pages/simulations'))
const SimulationDetailPage = lazy(() => import('@web/pages/simulations/detail'))
const AgentsPage = lazy(() => import('@web/pages/agents'))
const SpacesPage = lazy(() => import('@web/pages/spaces'))
const AnalyticsPage = lazy(() => import('@web/pages/analytics'))
const SettingsPage = lazy(() => import('@web/pages/settings'))
const ApiExplorerPage = lazy(() => import('@web/pages/developer/api-explorer'))
const NotFoundPage = lazy(() => import('@web/pages/not-found'))

// Desktop-specific menu action handler
function DesktopMenuHandler() {
  const navigate = useNavigate()
  const { notify } = useNotifications()

  useEffect(() => {
    const handleMenuAction = (event: CustomEvent<string>) => {
      const action = event.detail

      switch (action) {
        case 'new-simulation':
          navigate('/simulations/new')
          break
        case 'open-file':
          // File dialog will be handled by Tauri
          break
        case 'save-file':
          window.dispatchEvent(new CustomEvent('save-current-file'))
          break
        case 'save-file-as':
          window.dispatchEvent(new CustomEvent('save-current-file-as'))
          break
        case 'start-simulation':
          window.dispatchEvent(new CustomEvent('control-simulation', { detail: 'start' }))
          break
        case 'pause-simulation':
          window.dispatchEvent(new CustomEvent('control-simulation', { detail: 'pause' }))
          break
        case 'stop-simulation':
          window.dispatchEvent(new CustomEvent('control-simulation', { detail: 'stop' }))
          break
        case 'restart-simulation':
          window.dispatchEvent(new CustomEvent('control-simulation', { detail: 'restart' }))
          break
        case 'check-updates':
          notify('Checking for updates...', 'Please wait while we check for updates.')
          break
        default:
          console.log('Unknown menu action:', action)
      }
    }

    window.addEventListener('tauri-menu-action', handleMenuAction as EventListener)
    return () => {
      window.removeEventListener('tauri-menu-action', handleMenuAction as EventListener)
    }
  }, [navigate, notify])

  return null
}

// Startup update checker
function UpdateChecker() {
  const { settings } = useSettings()
  const { checkForUpdates, updateAvailable, updateInfo } = useUpdateChecker()
  const { notify, permissionGranted } = useNotifications()

  useEffect(() => {
    if (!isTauri()) return
    if (!settings?.check_updates_on_start) return

    const check = async () => {
      const result = await checkForUpdates()
      if (result?.available && permissionGranted) {
        notify(
          'Update Available',
          `Version ${result.latest_version} is available. You're currently on ${result.current_version}.`
        )
      }
    }

    // Check after a short delay to not slow down startup
    const timeout = setTimeout(check, 3000)
    return () => clearTimeout(timeout)
  }, [settings, checkForUpdates, notify, permissionGranted])

  return null
}

export default function App() {
  return (
    <ErrorBoundary>
      <ThemeProvider defaultTheme="dark" storageKey="gausstwin-desktop-theme">
        <AuthProvider>
          {isTauri() && (
            <>
              <DesktopMenuHandler />
              <UpdateChecker />
            </>
          )}
          <Suspense fallback={<LoadingScreen />}>
            <Routes>
              {/* Public routes */}
              <Route path="/login" element={<LoginPage />} />
              <Route path="/register" element={<RegisterPage />} />

              {/* Protected routes */}
              <Route element={<ProtectedRoute />}>
                <Route path="/" element={<Navigate to="/dashboard" replace />} />
                <Route path="/dashboard" element={<DashboardPage />} />
                <Route path="/simulations" element={<SimulationsPage />} />
                <Route path="/simulations/:id" element={<SimulationDetailPage />} />
                <Route path="/agents" element={<AgentsPage />} />
                <Route path="/spaces" element={<SpacesPage />} />
                <Route path="/analytics" element={<AnalyticsPage />} />
                <Route path="/settings" element={<SettingsPage />} />
                <Route path="/developer/api" element={<ApiExplorerPage />} />
              </Route>

              {/* 404 */}
              <Route path="*" element={<NotFoundPage />} />
            </Routes>
          </Suspense>
          <Toaster />
        </AuthProvider>
      </ThemeProvider>
    </ErrorBoundary>
  )
}
