import { Suspense, lazy } from 'react'
import { Routes, Route, Navigate } from 'react-router-dom'
import { Toaster } from '@/components/ui/toaster'
import { ThemeProvider } from '@/components/theme-provider'
import { AuthProvider } from '@/features/auth/auth-provider'
import { ProtectedRoute } from '@/features/auth/protected-route'
import { LoadingScreen } from '@/components/loading-screen'
import { ErrorBoundary } from '@/components/error-boundary'
import MainLayout from '@/components/layouts/main-layout'

// Lazy load pages for code splitting
const LoginPage = lazy(() => import('@/pages/auth/login'))
const RegisterPage = lazy(() => import('@/pages/auth/register'))
const DashboardPage = lazy(() => import('@/pages/dashboard'))
const SimulationsPage = lazy(() => import('@/pages/simulations'))
const SimulationDetailPage = lazy(() => import('@/pages/simulations/detail'))
const AgentsPage = lazy(() => import('@/pages/agents'))
const SpacesPage = lazy(() => import('@/pages/spaces'))
const AnalyticsPage = lazy(() => import('@/pages/analytics'))
const SettingsPage = lazy(() => import('@/pages/settings'))
const ApiExplorerPage = lazy(() => import('@/pages/developer/api-explorer'))
const NotFoundPage = lazy(() => import('@/pages/not-found'))

export default function App() {
  return (
    <ErrorBoundary>
      <ThemeProvider defaultTheme="dark" storageKey="gausstwin-theme">
        <AuthProvider>
          <Suspense fallback={<LoadingScreen />}>
            <Routes>
              {/* Public routes */}
              <Route path="/login" element={<LoginPage />} />
              <Route path="/register" element={<RegisterPage />} />

              {/* Protected routes with layout */}
              <Route element={<ProtectedRoute />}>
                <Route element={<MainLayout />}>
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
