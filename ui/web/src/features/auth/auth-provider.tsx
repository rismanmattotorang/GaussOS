import { createContext, useContext, useEffect, ReactNode } from 'react'
import { useAuthStore, User } from '@/stores/auth-store'

interface AuthContextType {
  user: User | null
  isAuthenticated: boolean
  isLoading: boolean
  error: string | null
  login: (credentials: { email: string; password: string }) => Promise<void>
  register: (data: { email: string; password: string; name: string }) => Promise<void>
  logout: () => void
}

const AuthContext = createContext<AuthContextType | null>(null)

export function AuthProvider({ children }: { children: ReactNode }) {
  const {
    user,
    isAuthenticated,
    isLoading,
    error,
    login,
    register,
    logout,
    refreshAccessToken,
  } = useAuthStore()

  // Set up token refresh interval
  useEffect(() => {
    if (!isAuthenticated) return

    // Refresh token every 14 minutes (assuming 15 min expiry)
    const interval = setInterval(() => {
      refreshAccessToken()
    }, 14 * 60 * 1000)

    return () => clearInterval(interval)
  }, [isAuthenticated, refreshAccessToken])

  return (
    <AuthContext.Provider
      value={{
        user,
        isAuthenticated,
        isLoading,
        error,
        login,
        register,
        logout,
      }}
    >
      {children}
    </AuthContext.Provider>
  )
}

export function useAuth() {
  const context = useContext(AuthContext)
  if (!context) {
    throw new Error('useAuth must be used within an AuthProvider')
  }
  return context
}
