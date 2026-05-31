import { useState, useEffect } from 'react'
import { Link } from 'react-router-dom'
import { motion } from 'framer-motion'
import {
  Activity,
  Cpu,
  HardDrive,
  Zap,
  Users,
  Boxes,
  ArrowUpRight,
  ArrowDownRight,
  TrendingUp,
  Play,
  Plus,
  Clock,
  BarChart3,
} from 'lucide-react'
import {
  LineChart,
  Line,
  AreaChart,
  Area,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
} from 'recharts'
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { cn, formatCompact, formatRelativeTime } from '@/lib/utils'

// Mock data - would come from API
const generateMetricsData = () => {
  const now = Date.now()
  return Array.from({ length: 24 }, (_, i) => ({
    time: new Date(now - (23 - i) * 3600000).toLocaleTimeString('en-US', { hour: '2-digit' }),
    cpu: Math.random() * 40 + 30,
    memory: Math.random() * 30 + 40,
    agents: Math.floor(Math.random() * 5000 + 10000),
    events: Math.floor(Math.random() * 10000 + 5000),
  }))
}

const recentActivity = [
  { id: 1, type: 'simulation', action: 'started', name: 'Traffic Flow v2.3', time: new Date(Date.now() - 300000) },
  { id: 2, type: 'agent', action: 'created', name: 'Supply Chain Agent Pool', time: new Date(Date.now() - 900000) },
  { id: 3, type: 'simulation', action: 'completed', name: 'Crowd Dynamics Test', time: new Date(Date.now() - 1800000) },
  { id: 4, type: 'alert', action: 'warning', name: 'High memory usage detected', time: new Date(Date.now() - 3600000) },
  { id: 5, type: 'simulation', action: 'paused', name: 'Network Optimization', time: new Date(Date.now() - 7200000) },
]

const activeSimulations = [
  { id: '1', name: 'Traffic Flow Simulation', agents: 12500, step: 45230, status: 'running', progress: 67 },
  { id: '2', name: 'Supply Chain Model', agents: 8200, step: 23100, status: 'running', progress: 45 },
  { id: '3', name: 'Urban Planning v3', agents: 25000, step: 78400, status: 'running', progress: 89 },
]

export default function DashboardPage() {
  const [metricsData, setMetricsData] = useState(generateMetricsData())

  // Simulate real-time updates
  useEffect(() => {
    const interval = setInterval(() => {
      setMetricsData(generateMetricsData())
    }, 5000)
    return () => clearInterval(interval)
  }, [])

  const stats = [
    {
      name: 'Active Simulations',
      value: 3,
      change: '+2',
      changeType: 'increase',
      icon: Boxes,
      color: 'text-twin-500',
      bgColor: 'bg-twin-500/10',
    },
    {
      name: 'Total Agents',
      value: 45700,
      change: '+12.5%',
      changeType: 'increase',
      icon: Users,
      color: 'text-cyber-500',
      bgColor: 'bg-cyber-500/10',
    },
    {
      name: 'CPU Usage',
      value: '47%',
      change: '-5%',
      changeType: 'decrease',
      icon: Cpu,
      color: 'text-gauss-500',
      bgColor: 'bg-gauss-500/10',
    },
    {
      name: 'Memory Usage',
      value: '62%',
      change: '+8%',
      changeType: 'increase',
      icon: HardDrive,
      color: 'text-orange-500',
      bgColor: 'bg-orange-500/10',
    },
  ]

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
        <div>
          <h1 className="text-3xl font-bold">Dashboard</h1>
          <p className="text-muted-foreground">Monitor and manage your digital twin simulations</p>
        </div>
        <div className="flex gap-3">
          <Link to="/simulations">
            <Button variant="outline">
              <BarChart3 className="mr-2 h-4 w-4" />
              View All
            </Button>
          </Link>
          <Link to="/simulations/new">
            <Button variant="gradient">
              <Plus className="mr-2 h-4 w-4" />
              New Simulation
            </Button>
          </Link>
        </div>
      </div>

      {/* Stats Grid */}
      <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
        {stats.map((stat, i) => (
          <motion.div
            key={stat.name}
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ delay: i * 0.1 }}
          >
            <Card className="card-hover">
              <CardContent className="p-6">
                <div className="flex items-center justify-between">
                  <div className={cn('rounded-lg p-2.5', stat.bgColor)}>
                    <stat.icon className={cn('h-5 w-5', stat.color)} />
                  </div>
                  <span
                    className={cn(
                      'flex items-center text-sm font-medium',
                      stat.changeType === 'increase' ? 'text-gauss-500' : 'text-red-500'
                    )}
                  >
                    {stat.changeType === 'increase' ? (
                      <ArrowUpRight className="mr-1 h-4 w-4" />
                    ) : (
                      <ArrowDownRight className="mr-1 h-4 w-4" />
                    )}
                    {stat.change}
                  </span>
                </div>
                <div className="mt-4">
                  <p className="text-2xl font-bold">
                    {typeof stat.value === 'number' ? formatCompact(stat.value) : stat.value}
                  </p>
                  <p className="text-sm text-muted-foreground">{stat.name}</p>
                </div>
              </CardContent>
            </Card>
          </motion.div>
        ))}
      </div>

      {/* Charts Row */}
      <div className="grid gap-6 lg:grid-cols-2">
        {/* Performance Chart */}
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Activity className="h-5 w-5 text-twin-500" />
              System Performance
            </CardTitle>
            <CardDescription>CPU and Memory usage over time</CardDescription>
          </CardHeader>
          <CardContent>
            <div className="h-[300px]">
              <ResponsiveContainer width="100%" height="100%">
                <AreaChart data={metricsData}>
                  <defs>
                    <linearGradient id="cpuGradient" x1="0" y1="0" x2="0" y2="1">
                      <stop offset="5%" stopColor="hsl(var(--primary))" stopOpacity={0.3} />
                      <stop offset="95%" stopColor="hsl(var(--primary))" stopOpacity={0} />
                    </linearGradient>
                    <linearGradient id="memGradient" x1="0" y1="0" x2="0" y2="1">
                      <stop offset="5%" stopColor="#22d3ee" stopOpacity={0.3} />
                      <stop offset="95%" stopColor="#22d3ee" stopOpacity={0} />
                    </linearGradient>
                  </defs>
                  <CartesianGrid strokeDasharray="3 3" className="stroke-border" />
                  <XAxis dataKey="time" className="text-xs" tick={{ fill: 'hsl(var(--muted-foreground))' }} />
                  <YAxis className="text-xs" tick={{ fill: 'hsl(var(--muted-foreground))' }} />
                  <Tooltip
                    contentStyle={{
                      backgroundColor: 'hsl(var(--card))',
                      border: '1px solid hsl(var(--border))',
                      borderRadius: '8px',
                    }}
                  />
                  <Area
                    type="monotone"
                    dataKey="cpu"
                    stroke="hsl(var(--primary))"
                    fill="url(#cpuGradient)"
                    name="CPU %"
                  />
                  <Area
                    type="monotone"
                    dataKey="memory"
                    stroke="#22d3ee"
                    fill="url(#memGradient)"
                    name="Memory %"
                  />
                </AreaChart>
              </ResponsiveContainer>
            </div>
          </CardContent>
        </Card>

        {/* Agent Activity Chart */}
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <TrendingUp className="h-5 w-5 text-gauss-500" />
              Agent Activity
            </CardTitle>
            <CardDescription>Active agents and events per hour</CardDescription>
          </CardHeader>
          <CardContent>
            <div className="h-[300px]">
              <ResponsiveContainer width="100%" height="100%">
                <LineChart data={metricsData}>
                  <CartesianGrid strokeDasharray="3 3" className="stroke-border" />
                  <XAxis dataKey="time" className="text-xs" tick={{ fill: 'hsl(var(--muted-foreground))' }} />
                  <YAxis className="text-xs" tick={{ fill: 'hsl(var(--muted-foreground))' }} />
                  <Tooltip
                    contentStyle={{
                      backgroundColor: 'hsl(var(--card))',
                      border: '1px solid hsl(var(--border))',
                      borderRadius: '8px',
                    }}
                  />
                  <Line
                    type="monotone"
                    dataKey="agents"
                    stroke="#22c566"
                    strokeWidth={2}
                    dot={false}
                    name="Agents"
                  />
                  <Line
                    type="monotone"
                    dataKey="events"
                    stroke="#8b5cf6"
                    strokeWidth={2}
                    dot={false}
                    name="Events"
                  />
                </LineChart>
              </ResponsiveContainer>
            </div>
          </CardContent>
        </Card>
      </div>

      {/* Bottom Row */}
      <div className="grid gap-6 lg:grid-cols-3">
        {/* Active Simulations */}
        <Card className="lg:col-span-2">
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Zap className="h-5 w-5 text-yellow-500" />
              Active Simulations
            </CardTitle>
            <CardDescription>Currently running simulation instances</CardDescription>
          </CardHeader>
          <CardContent>
            <div className="space-y-4">
              {activeSimulations.map((sim) => (
                <Link
                  key={sim.id}
                  to={`/simulations/${sim.id}`}
                  className="block rounded-lg border border-border p-4 transition-all hover:border-primary/50 hover:bg-accent/50"
                >
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-3">
                      <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-gauss-500/10">
                        <Play className="h-5 w-5 text-gauss-500" />
                      </div>
                      <div>
                        <p className="font-medium">{sim.name}</p>
                        <p className="text-sm text-muted-foreground">
                          {formatCompact(sim.agents)} agents · Step {formatCompact(sim.step)}
                        </p>
                      </div>
                    </div>
                    <div className="text-right">
                      <p className="text-sm font-medium">{sim.progress}%</p>
                      <div className="mt-1 h-1.5 w-24 rounded-full bg-secondary">
                        <div
                          className="h-full rounded-full bg-gradient-to-r from-twin-500 to-cyber-500"
                          style={{ width: `${sim.progress}%` }}
                        />
                      </div>
                    </div>
                  </div>
                </Link>
              ))}
            </div>
          </CardContent>
        </Card>

        {/* Recent Activity */}
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Clock className="h-5 w-5 text-muted-foreground" />
              Recent Activity
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="space-y-4">
              {recentActivity.map((activity) => (
                <div key={activity.id} className="flex items-start gap-3">
                  <div
                    className={cn(
                      'mt-0.5 h-2 w-2 rounded-full',
                      activity.type === 'simulation' && 'bg-twin-500',
                      activity.type === 'agent' && 'bg-cyber-500',
                      activity.type === 'alert' && 'bg-yellow-500'
                    )}
                  />
                  <div className="flex-1 space-y-1">
                    <p className="text-sm">
                      <span className="font-medium capitalize">{activity.action}</span>:{' '}
                      {activity.name}
                    </p>
                    <p className="text-xs text-muted-foreground">
                      {formatRelativeTime(activity.time)}
                    </p>
                  </div>
                </div>
              ))}
            </div>
          </CardContent>
        </Card>
      </div>
    </div>
  )
}
