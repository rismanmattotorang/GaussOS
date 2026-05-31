import { useState, useEffect } from 'react'
import { useParams, Link } from 'react-router-dom'
import { motion } from 'framer-motion'
import {
  ArrowLeft,
  Play,
  Pause,
  Square,
  RefreshCw,
  Download,
  Settings,
  Maximize2,
  Activity,
  Users,
  Zap,
  Clock,
  MoreVertical,
  TrendingUp,
  AlertCircle,
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
import { Badge } from '@/components/ui/badge'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { Progress } from '@/components/ui/progress'
import { Viewer3D } from '@/components/3d-viewer'
import { cn, formatCompact, formatRelativeTime } from '@/lib/utils'

// Mock data generation
const generateTimeSeriesData = (points = 50) => {
  return Array.from({ length: points }, (_, i) => ({
    time: i,
    agents: Math.floor(Math.random() * 5000 + 10000),
    events: Math.floor(Math.random() * 10000 + 5000),
    cpu: Math.random() * 40 + 30,
    memory: Math.random() * 30 + 40,
  }))
}

// Generate random 3D agent positions
const generate3DAgents = (count = 50) => {
  const colors = ['#22c566', '#22d3ee', '#8b5cf6', '#f59e0b', '#ef4444']
  return Array.from({ length: count }, (_, i) => ({
    id: `agent-${i}`,
    position: [
      (Math.random() - 0.5) * 10,
      Math.random() * 2,
      (Math.random() - 0.5) * 10,
    ] as [number, number, number],
    color: colors[Math.floor(Math.random() * colors.length)],
    isActive: Math.random() > 0.5,
  }))
}

const generate3DConnections = (agents: any[], count = 20) => {
  const connections = []
  for (let i = 0; i < count; i++) {
    const from = agents[Math.floor(Math.random() * agents.length)]
    const to = agents[Math.floor(Math.random() * agents.length)]
    if (from.id !== to.id) {
      connections.push({ from: from.id, to: to.id })
    }
  }
  return connections
}

export default function SimulationDetailPage() {
  const { id } = useParams<{ id: string }>()
  const [isPlaying, setIsPlaying] = useState(true)
  const [timeSeriesData, setTimeSeriesData] = useState(generateTimeSeriesData())
  const [agents3D, setAgents3D] = useState(generate3DAgents())
  const [connections3D, setConnections3D] = useState(() => generate3DConnections(agents3D))

  // Simulate real-time updates
  useEffect(() => {
    const interval = setInterval(() => {
      if (isPlaying) {
        setTimeSeriesData((prev) => {
          const newData = [...prev.slice(1)]
          const lastTime = prev[prev.length - 1].time
          newData.push({
            time: lastTime + 1,
            agents: Math.floor(Math.random() * 5000 + 10000),
            events: Math.floor(Math.random() * 10000 + 5000),
            cpu: Math.random() * 40 + 30,
            memory: Math.random() * 30 + 40,
          })
          return newData
        })
      }
    }, 1000)
    return () => clearInterval(interval)
  }, [isPlaying])

  const simulation = {
    id: id || '1',
    name: 'Traffic Flow Simulation',
    description: 'Urban traffic optimization model with adaptive signal control',
    status: 'running' as const,
    agentCount: 12500,
    currentStep: 45230,
    totalSteps: 100000,
    startedAt: new Date(Date.now() - 3600000 * 5),
    cpuUsage: 67,
    memoryUsage: 45,
    throughput: 2500,
    tags: ['traffic', 'urban', 'optimization'],
  }

  const progress = (simulation.currentStep / simulation.totalSteps) * 100

  const stats = [
    {
      label: 'Active Agents',
      value: formatCompact(simulation.agentCount),
      icon: Users,
      color: 'text-cyber-500',
      bgColor: 'bg-cyber-500/10',
    },
    {
      label: 'Current Step',
      value: formatCompact(simulation.currentStep),
      icon: Zap,
      color: 'text-twin-500',
      bgColor: 'bg-twin-500/10',
    },
    {
      label: 'CPU Usage',
      value: `${simulation.cpuUsage}%`,
      icon: Activity,
      color: 'text-gauss-500',
      bgColor: 'bg-gauss-500/10',
    },
    {
      label: 'Throughput',
      value: `${formatCompact(simulation.throughput)}/s`,
      icon: TrendingUp,
      color: 'text-orange-500',
      bgColor: 'bg-orange-500/10',
    },
  ]

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex flex-col gap-4">
        <div className="flex items-center gap-3">
          <Link to="/simulations">
            <Button variant="ghost" size="icon">
              <ArrowLeft className="h-4 w-4" />
            </Button>
          </Link>
          <div className="flex-1">
            <div className="flex items-center gap-3">
              <h1 className="text-3xl font-bold">{simulation.name}</h1>
              <Badge variant="success">
                {isPlaying ? 'Running' : 'Paused'}
              </Badge>
            </div>
            <p className="text-muted-foreground mt-1">{simulation.description}</p>
          </div>
          <Button variant="ghost" size="icon">
            <MoreVertical className="h-4 w-4" />
          </Button>
        </div>

        {/* Progress Bar */}
        <Card>
          <CardContent className="p-4">
            <div className="flex items-center justify-between mb-2">
              <span className="text-sm font-medium">Simulation Progress</span>
              <span className="text-sm text-muted-foreground">{Math.round(progress)}%</span>
            </div>
            <Progress value={progress} className="h-2" />
            <div className="flex items-center justify-between mt-2 text-xs text-muted-foreground">
              <span>
                Step {formatCompact(simulation.currentStep)} of {formatCompact(simulation.totalSteps)}
              </span>
              <span className="flex items-center gap-1">
                <Clock className="h-3 w-3" />
                Started {formatRelativeTime(simulation.startedAt)}
              </span>
            </div>
          </CardContent>
        </Card>

        {/* Control Buttons */}
        <div className="flex flex-wrap gap-2">
          <Button
            variant={isPlaying ? 'secondary' : 'default'}
            onClick={() => setIsPlaying(!isPlaying)}
          >
            {isPlaying ? (
              <>
                <Pause className="mr-2 h-4 w-4" />
                Pause
              </>
            ) : (
              <>
                <Play className="mr-2 h-4 w-4" />
                Resume
              </>
            )}
          </Button>
          <Button variant="outline">
            <Square className="mr-2 h-4 w-4" />
            Stop
          </Button>
          <Button variant="outline">
            <RefreshCw className="mr-2 h-4 w-4" />
            Restart
          </Button>
          <Button variant="outline">
            <Settings className="mr-2 h-4 w-4" />
            Configure
          </Button>
          <Button variant="outline">
            <Download className="mr-2 h-4 w-4" />
            Export Data
          </Button>
        </div>
      </div>

      {/* Stats Grid */}
      <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
        {stats.map((stat, i) => (
          <motion.div
            key={stat.label}
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ delay: i * 0.05 }}
          >
            <Card>
              <CardContent className="p-5">
                <div className="flex items-center gap-3">
                  <div className={cn('rounded-lg p-2.5', stat.bgColor)}>
                    <stat.icon className={cn('h-5 w-5', stat.color)} />
                  </div>
                  <div>
                    <p className="text-2xl font-bold">{stat.value}</p>
                    <p className="text-sm text-muted-foreground">{stat.label}</p>
                  </div>
                </div>
              </CardContent>
            </Card>
          </motion.div>
        ))}
      </div>

      {/* Main Content Tabs */}
      <Tabs defaultValue="overview" className="space-y-4">
        <TabsList>
          <TabsTrigger value="overview">Overview</TabsTrigger>
          <TabsTrigger value="3d-view">3D View</TabsTrigger>
          <TabsTrigger value="metrics">Metrics</TabsTrigger>
          <TabsTrigger value="logs">Logs</TabsTrigger>
          <TabsTrigger value="config">Configuration</TabsTrigger>
        </TabsList>

        <TabsContent value="overview" className="space-y-4">
          {/* Performance Charts */}
          <div className="grid gap-6 lg:grid-cols-2">
            <Card>
              <CardHeader>
                <CardTitle className="flex items-center gap-2">
                  <Activity className="h-5 w-5 text-twin-500" />
                  Agent Activity
                </CardTitle>
                <CardDescription>Active agents and events over time</CardDescription>
              </CardHeader>
              <CardContent>
                <div className="h-[300px]">
                  <ResponsiveContainer width="100%" height="100%">
                    <LineChart data={timeSeriesData}>
                      <CartesianGrid strokeDasharray="3 3" className="stroke-border" />
                      <XAxis
                        dataKey="time"
                        className="text-xs"
                        tick={{ fill: 'hsl(var(--muted-foreground))' }}
                      />
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

            <Card>
              <CardHeader>
                <CardTitle className="flex items-center gap-2">
                  <TrendingUp className="h-5 w-5 text-gauss-500" />
                  Resource Usage
                </CardTitle>
                <CardDescription>CPU and memory consumption</CardDescription>
              </CardHeader>
              <CardContent>
                <div className="h-[300px]">
                  <ResponsiveContainer width="100%" height="100%">
                    <AreaChart data={timeSeriesData}>
                      <defs>
                        <linearGradient id="cpuGrad" x1="0" y1="0" x2="0" y2="1">
                          <stop offset="5%" stopColor="hsl(var(--primary))" stopOpacity={0.3} />
                          <stop offset="95%" stopColor="hsl(var(--primary))" stopOpacity={0} />
                        </linearGradient>
                        <linearGradient id="memGrad" x1="0" y1="0" x2="0" y2="1">
                          <stop offset="5%" stopColor="#22d3ee" stopOpacity={0.3} />
                          <stop offset="95%" stopColor="#22d3ee" stopOpacity={0} />
                        </linearGradient>
                      </defs>
                      <CartesianGrid strokeDasharray="3 3" className="stroke-border" />
                      <XAxis
                        dataKey="time"
                        className="text-xs"
                        tick={{ fill: 'hsl(var(--muted-foreground))' }}
                      />
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
                        fill="url(#cpuGrad)"
                        name="CPU %"
                      />
                      <Area
                        type="monotone"
                        dataKey="memory"
                        stroke="#22d3ee"
                        fill="url(#memGrad)"
                        name="Memory %"
                      />
                    </AreaChart>
                  </ResponsiveContainer>
                </div>
              </CardContent>
            </Card>
          </div>

          {/* Event Log Preview */}
          <Card>
            <CardHeader>
              <CardTitle>Recent Events</CardTitle>
              <CardDescription>Latest simulation events and state changes</CardDescription>
            </CardHeader>
            <CardContent>
              <div className="space-y-3">
                {[
                  { type: 'info', message: 'Agent pool initialized with 12,500 agents', time: new Date(Date.now() - 120000) },
                  { type: 'success', message: 'Checkpoint saved at step 45,000', time: new Date(Date.now() - 300000) },
                  { type: 'warning', message: 'High CPU usage detected (82%)', time: new Date(Date.now() - 600000) },
                  { type: 'info', message: 'Traffic density: 67% (Normal)', time: new Date(Date.now() - 900000) },
                ].map((event, idx) => (
                  <div key={idx} className="flex items-start gap-3 text-sm">
                    <div
                      className={cn(
                        'mt-0.5 h-2 w-2 rounded-full',
                        event.type === 'success' && 'bg-gauss-500',
                        event.type === 'warning' && 'bg-yellow-500',
                        event.type === 'info' && 'bg-cyber-500'
                      )}
                    />
                    <div className="flex-1">
                      <p>{event.message}</p>
                      <p className="text-xs text-muted-foreground mt-0.5">
                        {formatRelativeTime(event.time)}
                      </p>
                    </div>
                  </div>
                ))}
              </div>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="3d-view">
          <Card>
            <CardHeader>
              <div className="flex items-center justify-between">
                <div>
                  <CardTitle className="flex items-center gap-2">
                    3D Spatial View
                  </CardTitle>
                  <CardDescription>Real-time 3D visualization of agent positions and interactions</CardDescription>
                </div>
                <Button variant="outline" size="sm">
                  <Maximize2 className="mr-2 h-4 w-4" />
                  Fullscreen
                </Button>
              </div>
            </CardHeader>
            <CardContent>
              <div className="h-[600px]">
                <Viewer3D agents={agents3D} connections={connections3D} />
              </div>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="metrics">
          <Card>
            <CardHeader>
              <CardTitle>Detailed Metrics</CardTitle>
              <CardDescription>Comprehensive performance and simulation metrics</CardDescription>
            </CardHeader>
            <CardContent>
              <div className="text-center py-12">
                <AlertCircle className="h-12 w-12 mx-auto text-muted-foreground" />
                <p className="mt-4 text-muted-foreground">Metrics view coming soon</p>
              </div>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="logs">
          <Card>
            <CardHeader>
              <CardTitle>Simulation Logs</CardTitle>
              <CardDescription>Complete log history and debugging information</CardDescription>
            </CardHeader>
            <CardContent>
              <div className="text-center py-12">
                <AlertCircle className="h-12 w-12 mx-auto text-muted-foreground" />
                <p className="mt-4 text-muted-foreground">Logs view coming soon</p>
              </div>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="config">
          <Card>
            <CardHeader>
              <CardTitle>Configuration</CardTitle>
              <CardDescription>Simulation parameters and settings</CardDescription>
            </CardHeader>
            <CardContent>
              <div className="text-center py-12">
                <AlertCircle className="h-12 w-12 mx-auto text-muted-foreground" />
                <p className="mt-4 text-muted-foreground">Configuration view coming soon</p>
              </div>
            </CardContent>
          </Card>
        </TabsContent>
      </Tabs>
    </div>
  )
}
