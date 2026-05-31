import { useState } from 'react'
import { Link, Outlet, useLocation, useNavigate } from 'react-router-dom'
import {
  LayoutDashboard,
  Boxes,
  Users,
  Compass,
  BarChart3,
  Settings,
  Code,
  Menu,
  X,
  LogOut,
  Moon,
  Sun,
  Bell,
  Search,
  ChevronDown,
} from 'lucide-react'
import { motion, AnimatePresence } from 'framer-motion'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Avatar, AvatarFallback, AvatarImage } from '@/components/ui/avatar'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { Badge } from '@/components/ui/badge'
import { cn } from '@/lib/utils'
import { useTheme } from '@/components/theme-provider'

const navigation = [
  { name: 'Dashboard', href: '/dashboard', icon: LayoutDashboard },
  { name: 'Simulations', href: '/simulations', icon: Boxes },
  { name: 'Agents', href: '/agents', icon: Users },
  { name: 'Spaces', href: '/spaces', icon: Compass },
  { name: 'Analytics', href: '/analytics', icon: BarChart3 },
  { name: 'Settings', href: '/settings', icon: Settings },
  { name: 'API Explorer', href: '/developer/api', icon: Code },
]

function MainLayout() {
  const [sidebarOpen, setSidebarOpen] = useState(true)
  const [mobileMenuOpen, setMobileMenuOpen] = useState(false)
  const location = useLocation()
  const navigate = useNavigate()
  const { theme, setTheme } = useTheme()

  const handleLogout = () => {
    // Handle logout logic
    navigate('/login')
  }

  return (
    <div className="min-h-screen bg-background">
      {/* Desktop Sidebar */}
      <aside
        className={cn(
          'fixed left-0 top-0 z-40 h-screen border-r border-border bg-card transition-all duration-300 hidden lg:block',
          sidebarOpen ? 'w-64' : 'w-20'
        )}
      >
        <div className="flex h-full flex-col">
          {/* Logo */}
          <div className="flex h-16 items-center justify-between border-b border-border px-4">
            {sidebarOpen ? (
              <>
                <Link to="/dashboard" className="flex items-center gap-2">
                  <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-gradient-to-br from-twin-500 to-cyber-500">
                    <span className="text-lg font-bold text-white">G</span>
                  </div>
                  <span className="text-lg font-bold bg-gradient-to-r from-twin-500 to-cyber-500 bg-clip-text text-transparent">
                    GaussTwin
                  </span>
                </Link>
                <Button
                  variant="ghost"
                  size="icon-sm"
                  onClick={() => setSidebarOpen(false)}
                >
                  <X className="h-4 w-4" />
                </Button>
              </>
            ) : (
              <Button
                variant="ghost"
                size="icon-sm"
                onClick={() => setSidebarOpen(true)}
                className="mx-auto"
              >
                <Menu className="h-4 w-4" />
              </Button>
            )}
          </div>

          {/* Navigation */}
          <nav className="flex-1 space-y-1 p-3 overflow-y-auto">
            {navigation.map((item) => {
              const isActive = location.pathname.startsWith(item.href)
              return (
                <Link key={item.name} to={item.href}>
                  <motion.div
                    whileHover={{ x: 4 }}
                    className={cn(
                      'flex items-center gap-3 rounded-lg px-3 py-2.5 text-sm font-medium transition-colors',
                      isActive
                        ? 'bg-primary text-primary-foreground'
                        : 'text-muted-foreground hover:bg-accent hover:text-accent-foreground'
                    )}
                  >
                    <item.icon className="h-5 w-5 shrink-0" />
                    {sidebarOpen && <span>{item.name}</span>}
                  </motion.div>
                </Link>
              )
            })}
          </nav>

          {/* Status Card */}
          {sidebarOpen && (
            <div className="m-3 rounded-lg border border-border bg-accent/50 p-3">
              <div className="flex items-center gap-2 text-sm">
                <div className="h-2 w-2 rounded-full bg-gauss-500 animate-pulse" />
                <span className="font-medium">System Online</span>
              </div>
              <div className="mt-2 space-y-1 text-xs text-muted-foreground">
                <div className="flex justify-between">
                  <span>CPU</span>
                  <span>47%</span>
                </div>
                <div className="flex justify-between">
                  <span>Memory</span>
                  <span>62%</span>
                </div>
              </div>
            </div>
          )}
        </div>
      </aside>

      {/* Mobile Menu */}
      <AnimatePresence>
        {mobileMenuOpen && (
          <>
            <motion.div
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              exit={{ opacity: 0 }}
              className="fixed inset-0 z-40 bg-background/80 backdrop-blur-sm lg:hidden"
              onClick={() => setMobileMenuOpen(false)}
            />
            <motion.aside
              initial={{ x: -300 }}
              animate={{ x: 0 }}
              exit={{ x: -300 }}
              className="fixed left-0 top-0 z-50 h-screen w-64 border-r border-border bg-card lg:hidden"
            >
              <div className="flex h-full flex-col">
                <div className="flex h-16 items-center justify-between border-b border-border px-4">
                  <Link to="/dashboard" className="flex items-center gap-2">
                    <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-gradient-to-br from-twin-500 to-cyber-500">
                      <span className="text-lg font-bold text-white">G</span>
                    </div>
                    <span className="text-lg font-bold">GaussTwin</span>
                  </Link>
                  <Button
                    variant="ghost"
                    size="icon-sm"
                    onClick={() => setMobileMenuOpen(false)}
                  >
                    <X className="h-4 w-4" />
                  </Button>
                </div>

                <nav className="flex-1 space-y-1 p-3">
                  {navigation.map((item) => {
                    const isActive = location.pathname.startsWith(item.href)
                    return (
                      <Link
                        key={item.name}
                        to={item.href}
                        onClick={() => setMobileMenuOpen(false)}
                      >
                        <div
                          className={cn(
                            'flex items-center gap-3 rounded-lg px-3 py-2.5 text-sm font-medium transition-colors',
                            isActive
                              ? 'bg-primary text-primary-foreground'
                              : 'text-muted-foreground hover:bg-accent hover:text-accent-foreground'
                          )}
                        >
                          <item.icon className="h-5 w-5" />
                          <span>{item.name}</span>
                        </div>
                      </Link>
                    )
                  })}
                </nav>
              </div>
            </motion.aside>
          </>
        )}
      </AnimatePresence>

      {/* Main Content */}
      <div
        className={cn(
          'transition-all duration-300',
          sidebarOpen ? 'lg:pl-64' : 'lg:pl-20'
        )}
      >
        {/* Top Bar */}
        <header className="sticky top-0 z-30 flex h-16 items-center gap-4 border-b border-border bg-card/95 backdrop-blur supports-[backdrop-filter]:bg-card/60 px-4 lg:px-6">
          <Button
            variant="ghost"
            size="icon"
            className="lg:hidden"
            onClick={() => setMobileMenuOpen(true)}
          >
            <Menu className="h-5 w-5" />
          </Button>

          {/* Search */}
          <div className="flex-1 max-w-md">
            <div className="relative">
              <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
              <Input
                type="search"
                placeholder="Search simulations, agents..."
                className="pl-9 w-full"
              />
            </div>
          </div>

          <div className="flex items-center gap-2">
            {/* Theme Toggle */}
            <Button
              variant="ghost"
              size="icon"
              onClick={() => setTheme(theme === 'dark' ? 'light' : 'dark')}
            >
              {theme === 'dark' ? (
                <Sun className="h-5 w-5" />
              ) : (
                <Moon className="h-5 w-5" />
              )}
            </Button>

            {/* Notifications */}
            <Button variant="ghost" size="icon" className="relative">
              <Bell className="h-5 w-5" />
              <span className="absolute top-1 right-1 h-2 w-2 rounded-full bg-destructive" />
            </Button>

            {/* User Menu */}
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button variant="ghost" className="gap-2">
                  <Avatar className="h-8 w-8">
                    <AvatarImage src="" />
                    <AvatarFallback className="bg-gradient-to-br from-twin-500 to-cyber-500 text-white">
                      JD
                    </AvatarFallback>
                  </Avatar>
                  <span className="hidden sm:inline-block">John Doe</span>
                  <ChevronDown className="h-4 w-4 opacity-50" />
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end" className="w-56">
                <DropdownMenuLabel>
                  <div className="flex flex-col space-y-1">
                    <p className="text-sm font-medium">John Doe</p>
                    <p className="text-xs text-muted-foreground">john.doe@example.com</p>
                  </div>
                </DropdownMenuLabel>
                <DropdownMenuSeparator />
                <DropdownMenuItem>
                  <Settings className="mr-2 h-4 w-4" />
                  Settings
                </DropdownMenuItem>
                <DropdownMenuItem>
                  <Code className="mr-2 h-4 w-4" />
                  API Keys
                </DropdownMenuItem>
                <DropdownMenuSeparator />
                <DropdownMenuItem onClick={handleLogout} className="text-destructive">
                  <LogOut className="mr-2 h-4 w-4" />
                  Log out
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
          </div>
        </header>

        {/* Page Content */}
        <main className="p-4 lg:p-6">
          <Outlet />
        </main>

        {/* Footer */}
        <footer className="border-t border-border bg-card px-6 py-4 text-center text-sm text-muted-foreground">
          <p>
            GaussTwin v0.1.0 · High-Performance Digital Twin Framework ·{' '}
            <a href="#" className="hover:text-foreground transition-colors">
              Documentation
            </a>
          </p>
        </footer>
      </div>
    </div>
  )
}

export default MainLayout;
