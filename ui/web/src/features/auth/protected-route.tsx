import { Navigate, Outlet, useLocation } from 'react-router-dom'
import { useAuth } from './auth-provider'
import { LoadingScreen } from '@/components/loading-screen'

export function ProtectedRoute() {
  const { isAuthenticated, isLoading } = useAuth()
  const location = useLocation()

  if (isLoading) {
    return <LoadingScreen />
  }

  if (!isAuthenticated) {
    return <Navigate to="/login" state={{ from: location }} replace />
  }

  return <Outlet />
}
