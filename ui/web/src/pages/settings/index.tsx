import { useState } from 'react'
import { motion } from 'framer-motion'
import {
  User,
  Palette,
  Bell,
  Key,
  Shield,
  Save,
  Camera,
  Moon,
  Sun,
  Monitor,
  Copy,
  Eye,
  EyeOff,
  Plus,
  Trash2,
} from 'lucide-react'
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { useTheme } from '@/components/theme-provider'
import { useAuth } from '@/features/auth/auth-provider'
import { useToast } from '@/hooks/use-toast'
import { cn } from '@/lib/utils'

const tabs = [
  { id: 'profile', label: 'Profile', icon: User },
  { id: 'appearance', label: 'Appearance', icon: Palette },
  { id: 'notifications', label: 'Notifications', icon: Bell },
  { id: 'api-keys', label: 'API Keys', icon: Key },
  { id: 'security', label: 'Security', icon: Shield },
]

const apiKeys = [
  { id: '1', name: 'Production Key', key: 'gt_live_xxxxxxxxxxxxxxxxxxxxx', lastUsed: '2 hours ago', created: 'Jan 10, 2026' },
  { id: '2', name: 'Development Key', key: 'gt_test_xxxxxxxxxxxxxxxxxxxxx', lastUsed: '5 minutes ago', created: 'Jan 15, 2026' },
]

export default function SettingsPage() {
  const [activeTab, setActiveTab] = useState('profile')
  const { theme, setTheme } = useTheme()
  const { user } = useAuth()
  const { toast } = useToast()
  const [showKey, setShowKey] = useState<string | null>(null)

  const handleSave = () => {
    toast({
      title: 'Settings saved',
      description: 'Your preferences have been updated.',
      variant: 'success',
    })
  }

  return (
    <div className="space-y-6">
      {/* Header */}
      <div>
        <h1 className="text-3xl font-bold">Settings</h1>
        <p className="text-muted-foreground">Manage your account and preferences</p>
      </div>

      <div className="flex flex-col gap-6 lg:flex-row">
        {/* Sidebar */}
        <Card className="h-fit lg:w-64">
          <CardContent className="p-2">
            <nav className="space-y-1">
              {tabs.map((tab) => (
                <button
                  key={tab.id}
                  onClick={() => setActiveTab(tab.id)}
                  className={cn(
                    'flex w-full items-center gap-3 rounded-lg px-3 py-2 text-sm font-medium transition-colors',
                    activeTab === tab.id
                      ? 'bg-primary/10 text-primary'
                      : 'text-muted-foreground hover:bg-accent hover:text-foreground'
                  )}
                >
                  <tab.icon className="h-4 w-4" />
                  {tab.label}
                </button>
              ))}
            </nav>
          </CardContent>
        </Card>

        {/* Content */}
        <div className="flex-1 space-y-6">
          {activeTab === 'profile' && (
            <motion.div
              initial={{ opacity: 0, x: 20 }}
              animate={{ opacity: 1, x: 0 }}
              className="space-y-6"
            >
              <Card>
                <CardHeader>
                  <CardTitle>Profile Information</CardTitle>
                  <CardDescription>Update your personal details</CardDescription>
                </CardHeader>
                <CardContent className="space-y-6">
                  {/* Avatar */}
                  <div className="flex items-center gap-4">
                    <div className="relative">
                      <div className="h-20 w-20 rounded-full bg-gradient-to-br from-twin-400 to-cyber-400 flex items-center justify-center text-2xl font-bold text-white">
                        {user?.name?.charAt(0).toUpperCase() || 'U'}
                      </div>
                      <button className="absolute bottom-0 right-0 rounded-full bg-primary p-1.5 text-primary-foreground shadow-lg">
                        <Camera className="h-3 w-3" />
                      </button>
                    </div>
                    <div>
                      <p className="font-medium">{user?.name || 'User'}</p>
                      <p className="text-sm text-muted-foreground">{user?.email}</p>
                    </div>
                  </div>

                  <div className="grid gap-4 sm:grid-cols-2">
                    <div className="space-y-2">
                      <Label htmlFor="name">Full Name</Label>
                      <Input id="name" defaultValue={user?.name || ''} />
                    </div>
                    <div className="space-y-2">
                      <Label htmlFor="email">Email</Label>
                      <Input id="email" type="email" defaultValue={user?.email || ''} />
                    </div>
                  </div>

                  <Button onClick={handleSave}>
                    <Save className="mr-2 h-4 w-4" />
                    Save Changes
                  </Button>
                </CardContent>
              </Card>
            </motion.div>
          )}

          {activeTab === 'appearance' && (
            <motion.div
              initial={{ opacity: 0, x: 20 }}
              animate={{ opacity: 1, x: 0 }}
              className="space-y-6"
            >
              <Card>
                <CardHeader>
                  <CardTitle>Theme</CardTitle>
                  <CardDescription>Customize the appearance of the application</CardDescription>
                </CardHeader>
                <CardContent>
                  <div className="grid gap-4 sm:grid-cols-3">
                    {[
                      { id: 'light', label: 'Light', icon: Sun },
                      { id: 'dark', label: 'Dark', icon: Moon },
                      { id: 'system', label: 'System', icon: Monitor },
                    ].map((option) => (
                      <button
                        key={option.id}
                        onClick={() => setTheme(option.id as 'light' | 'dark' | 'system')}
                        className={cn(
                          'flex flex-col items-center gap-2 rounded-lg border p-4 transition-all',
                          theme === option.id
                            ? 'border-primary bg-primary/5'
                            : 'border-border hover:border-primary/50'
                        )}
                      >
                        <option.icon className="h-6 w-6" />
                        <span className="text-sm font-medium">{option.label}</span>
                      </button>
                    ))}
                  </div>
                </CardContent>
              </Card>
            </motion.div>
          )}

          {activeTab === 'notifications' && (
            <motion.div
              initial={{ opacity: 0, x: 20 }}
              animate={{ opacity: 1, x: 0 }}
              className="space-y-6"
            >
              <Card>
                <CardHeader>
                  <CardTitle>Notification Preferences</CardTitle>
                  <CardDescription>Choose what notifications you receive</CardDescription>
                </CardHeader>
                <CardContent className="space-y-4">
                  {[
                    { id: 'sim-complete', label: 'Simulation completed', description: 'Get notified when a simulation finishes' },
                    { id: 'sim-error', label: 'Simulation errors', description: 'Receive alerts when simulations encounter errors' },
                    { id: 'anomaly', label: 'Anomaly detection', description: 'Get notified of detected anomalies' },
                    { id: 'updates', label: 'Product updates', description: 'Receive news about new features' },
                  ].map((item) => (
                    <div key={item.id} className="flex items-center justify-between">
                      <div>
                        <p className="font-medium">{item.label}</p>
                        <p className="text-sm text-muted-foreground">{item.description}</p>
                      </div>
                      <label className="relative inline-flex cursor-pointer items-center">
                        <input type="checkbox" className="peer sr-only" defaultChecked />
                        <div className="peer h-6 w-11 rounded-full bg-muted after:absolute after:left-[2px] after:top-0.5 after:h-5 after:w-5 after:rounded-full after:border after:border-gray-300 after:bg-white after:transition-all after:content-[''] peer-checked:bg-primary peer-checked:after:translate-x-full" />
                      </label>
                    </div>
                  ))}
                </CardContent>
              </Card>
            </motion.div>
          )}

          {activeTab === 'api-keys' && (
            <motion.div
              initial={{ opacity: 0, x: 20 }}
              animate={{ opacity: 1, x: 0 }}
              className="space-y-6"
            >
              <Card>
                <CardHeader className="flex flex-row items-center justify-between">
                  <div>
                    <CardTitle>API Keys</CardTitle>
                    <CardDescription>Manage your API keys for programmatic access</CardDescription>
                  </div>
                  <Button>
                    <Plus className="mr-2 h-4 w-4" />
                    Create Key
                  </Button>
                </CardHeader>
                <CardContent>
                  <div className="space-y-4">
                    {apiKeys.map((apiKey) => (
                      <div
                        key={apiKey.id}
                        className="flex items-center justify-between rounded-lg border border-border p-4"
                      >
                        <div className="space-y-1">
                          <p className="font-medium">{apiKey.name}</p>
                          <div className="flex items-center gap-2">
                            <code className="rounded bg-muted px-2 py-0.5 text-sm">
                              {showKey === apiKey.id ? apiKey.key : '••••••••••••••••••••••'}
                            </code>
                            <Button
                              variant="ghost"
                              size="icon-sm"
                              onClick={() => setShowKey(showKey === apiKey.id ? null : apiKey.id)}
                            >
                              {showKey === apiKey.id ? (
                                <EyeOff className="h-3 w-3" />
                              ) : (
                                <Eye className="h-3 w-3" />
                              )}
                            </Button>
                            <Button variant="ghost" size="icon-sm">
                              <Copy className="h-3 w-3" />
                            </Button>
                          </div>
                          <p className="text-xs text-muted-foreground">
                            Created {apiKey.created} · Last used {apiKey.lastUsed}
                          </p>
                        </div>
                        <Button variant="ghost" size="icon" className="text-destructive">
                          <Trash2 className="h-4 w-4" />
                        </Button>
                      </div>
                    ))}
                  </div>
                </CardContent>
              </Card>
            </motion.div>
          )}

          {activeTab === 'security' && (
            <motion.div
              initial={{ opacity: 0, x: 20 }}
              animate={{ opacity: 1, x: 0 }}
              className="space-y-6"
            >
              <Card>
                <CardHeader>
                  <CardTitle>Change Password</CardTitle>
                  <CardDescription>Update your password to keep your account secure</CardDescription>
                </CardHeader>
                <CardContent className="space-y-4">
                  <div className="space-y-2">
                    <Label htmlFor="current-password">Current Password</Label>
                    <Input id="current-password" type="password" />
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="new-password">New Password</Label>
                    <Input id="new-password" type="password" />
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="confirm-password">Confirm New Password</Label>
                    <Input id="confirm-password" type="password" />
                  </div>
                  <Button>Update Password</Button>
                </CardContent>
              </Card>

              <Card>
                <CardHeader>
                  <CardTitle>Two-Factor Authentication</CardTitle>
                  <CardDescription>Add an extra layer of security to your account</CardDescription>
                </CardHeader>
                <CardContent>
                  <div className="flex items-center justify-between">
                    <div>
                      <p className="font-medium">Status: Disabled</p>
                      <p className="text-sm text-muted-foreground">
                        Protect your account with 2FA
                      </p>
                    </div>
                    <Button variant="outline">Enable 2FA</Button>
                  </div>
                </CardContent>
              </Card>
            </motion.div>
          )}
        </div>
      </div>
    </div>
  )
}
