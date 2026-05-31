import { useState } from 'react'
import { Link } from 'react-router-dom'
import { motion, AnimatePresence } from 'framer-motion'
import {
  Plus,
  Search,
  Grid3X3,
  List,
  Play,
  Pause,
  Square,
  MoreVertical,
  Clock,
  Users,
  Zap,
  CheckCircle2,
  XCircle,
  AlertCircle,
} from 'lucide-react'
import { Card, CardContent } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { cn, formatCompact, formatRelativeTime } from '@/lib/utils'

type SimulationStatus = 'running' | 'paused' | 'stopped' | 'completed' | 'error'

interface Simulation {
  id: string
  name: string
  description: string
  status: SimulationStatus
  agentCount: number
  currentStep: number
  totalSteps: number
  startedAt: Date
  updatedAt: Date
  cpuUsage: number
  memoryUsage: number
  tags: string[]
}

const mockSimulations: Simulation[] = [
  {
    id: '1',
    name: 'Traffic Flow Simulation',
    description: 'Urban traffic optimization model with adaptive signals',
    status: 'running',
    agentCount: 12500,
    currentStep: 45230,
    totalSteps: 100000,
    startedAt: new Date(Date.now() - 3600000 * 5),
    updatedAt: new Date(Date.now() - 60000),
    cpuUsage: 67,
    memoryUsage: 45,
    tags: ['traffic', 'urban', 'optimization'],
  },
  {
    id: '2',
    name: 'Supply Chain Model',
    description: 'End-to-end supply chain with disruption scenarios',
    status: 'running',
    agentCount: 8200,
    currentStep: 23100,
    totalSteps: 50000,
    startedAt: new Date(Date.now() - 3600000 * 2),
    updatedAt: new Date(Date.now() - 120000),
    cpuUsage: 45,
    memoryUsage: 38,
    tags: ['supply-chain', 'logistics'],
  },
  {
    id: '3',
    name: 'Crowd Dynamics Test',
    description: 'Emergency evacuation simulation for stadium',
    status: 'completed',
    agentCount: 50000,
    currentStep: 10000,
    totalSteps: 10000,
    startedAt: new Date(Date.now() - 86400000),
    updatedAt: new Date(Date.now() - 3600000 * 3),
    cpuUsage: 0,
    memoryUsage: 0,
    tags: ['crowd', 'evacuation', 'safety'],
  },
  {
    id: '4',
    name: 'Financial Market Model',
    description: 'High-frequency trading simulation with multiple strategies',
    status: 'paused',
    agentCount: 1500,
    currentStep: 890000,
    totalSteps: 1000000,
    startedAt: new Date(Date.now() - 86400000 * 2),
    updatedAt: new Date(Date.now() - 7200000),
    cpuUsage: 0,
    memoryUsage: 25,
    tags: ['finance', 'trading', 'HFT'],
  },
  {
    id: '5',
    name: 'Disease Spread Model',
    description: 'Epidemiological model with vaccination strategies',
    status: 'error',
    agentCount: 100000,
    currentStep: 5600,
    totalSteps: 50000,
    startedAt: new Date(Date.now() - 3600000 * 8),
    updatedAt: new Date(Date.now() - 3600000),
    cpuUsage: 0,
    memoryUsage: 0,
    tags: ['epidemiology', 'health'],
  },
  {
    id: '6',
    name: 'Smart Grid Optimization',
    description: 'Renewable energy distribution with demand prediction',
    status: 'stopped',
    agentCount: 5000,
    currentStep: 0,
    totalSteps: 100000,
    startedAt: new Date(Date.now() - 86400000 * 5),
    updatedAt: new Date(Date.now() - 86400000),
    cpuUsage: 0,
    memoryUsage: 0,
    tags: ['energy', 'smart-grid', 'renewable'],
  },
]

const statusConfig: Record<SimulationStatus, { icon: typeof Play; color: string; label: string }> = {
  running: { icon: Play, color: 'text-gauss-500', label: 'Running' },
  paused: { icon: Pause, color: 'text-yellow-500', label: 'Paused' },
  stopped: { icon: Square, color: 'text-muted-foreground', label: 'Stopped' },
  completed: { icon: CheckCircle2, color: 'text-cyber-500', label: 'Completed' },
  error: { icon: XCircle, color: 'text-destructive', label: 'Error' },
}

export default function SimulationsPage() {
  const [view, setView] = useState<'grid' | 'list'>('grid')
  const [searchQuery, setSearchQuery] = useState('')
  const [statusFilter, setStatusFilter] = useState<SimulationStatus | 'all'>('all')

  const filteredSimulations = mockSimulations.filter((sim) => {
    const matchesSearch =
      sim.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
      sim.description.toLowerCase().includes(searchQuery.toLowerCase()) ||
      sim.tags.some((tag) => tag.toLowerCase().includes(searchQuery.toLowerCase()))
    const matchesStatus = statusFilter === 'all' || sim.status === statusFilter
    return matchesSearch && matchesStatus
  })

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
        <div>
          <h1 className="text-3xl font-bold">Simulations</h1>
          <p className="text-muted-foreground">Manage and monitor your simulation instances</p>
        </div>
        <Link to="/simulations/new">
          <Button variant="gradient">
            <Plus className="mr-2 h-4 w-4" />
            New Simulation
          </Button>
        </Link>
      </div>

      {/* Filters */}
      <div className="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
        <div className="flex flex-1 gap-3">
          <div className="relative flex-1 max-w-md">
            <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
            <Input
              placeholder="Search simulations..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="pl-9"
            />
          </div>
          <div className="flex gap-2">
            {(['all', 'running', 'paused', 'completed'] as const).map((status) => (
              <Button
                key={status}
                variant={statusFilter === status ? 'default' : 'outline'}
                size="sm"
                onClick={() => setStatusFilter(status)}
                className={cn(
                  statusFilter === status && status !== 'all' && statusConfig[status]?.color
                )}
              >
                {status === 'all' ? 'All' : statusConfig[status].label}
              </Button>
            ))}
          </div>
        </div>
        <div className="flex items-center gap-2">
          <Button
            variant={view === 'grid' ? 'secondary' : 'ghost'}
            size="icon"
            onClick={() => setView('grid')}
          >
            <Grid3X3 className="h-4 w-4" />
          </Button>
          <Button
            variant={view === 'list' ? 'secondary' : 'ghost'}
            size="icon"
            onClick={() => setView('list')}
          >
            <List className="h-4 w-4" />
          </Button>
        </div>
      </div>

      {/* Simulations Grid/List */}
      <AnimatePresence mode="wait">
        {view === 'grid' ? (
          <motion.div
            key="grid"
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3"
          >
            {filteredSimulations.map((sim, i) => {
              const StatusIcon = statusConfig[sim.status].icon
              const progress = (sim.currentStep / sim.totalSteps) * 100

              return (
                <motion.div
                  key={sim.id}
                  initial={{ opacity: 0, y: 20 }}
                  animate={{ opacity: 1, y: 0 }}
                  transition={{ delay: i * 0.05 }}
                >
                  <Link to={`/simulations/${sim.id}`}>
                    <Card className="card-hover h-full">
                      <CardContent className="p-5">
                        <div className="flex items-start justify-between">
                          <div className="flex items-center gap-2">
                            <StatusIcon
                              className={cn('h-4 w-4', statusConfig[sim.status].color)}
                            />
                            <span className={cn('text-sm font-medium', statusConfig[sim.status].color)}>
                              {statusConfig[sim.status].label}
                            </span>
                          </div>
                          <Button variant="ghost" size="icon-sm">
                            <MoreVertical className="h-4 w-4" />
                          </Button>
                        </div>

                        <h3 className="mt-3 text-lg font-semibold">{sim.name}</h3>
                        <p className="mt-1 text-sm text-muted-foreground line-clamp-2">
                          {sim.description}
                        </p>

                        {/* Progress bar */}
                        <div className="mt-4">
                          <div className="flex items-center justify-between text-sm">
                            <span className="text-muted-foreground">Progress</span>
                            <span className="font-medium">{Math.round(progress)}%</span>
                          </div>
                          <div className="mt-1.5 h-1.5 w-full rounded-full bg-secondary">
                            <div
                              className={cn(
                                'h-full rounded-full transition-all',
                                sim.status === 'completed'
                                  ? 'bg-cyber-500'
                                  : sim.status === 'error'
                                  ? 'bg-destructive'
                                  : 'bg-gradient-to-r from-twin-500 to-cyber-500'
                              )}
                              style={{ width: `${progress}%` }}
                            />
                          </div>
                        </div>

                        {/* Stats */}
                        <div className="mt-4 flex items-center gap-4 text-sm text-muted-foreground">
                          <div className="flex items-center gap-1">
                            <Users className="h-3.5 w-3.5" />
                            {formatCompact(sim.agentCount)}
                          </div>
                          <div className="flex items-center gap-1">
                            <Zap className="h-3.5 w-3.5" />
                            {formatCompact(sim.currentStep)} steps
                          </div>
                        </div>

                        {/* Tags */}
                        <div className="mt-4 flex flex-wrap gap-1.5">
                          {sim.tags.slice(0, 3).map((tag) => (
                            <span
                              key={tag}
                              className="rounded-full bg-secondary px-2 py-0.5 text-xs text-muted-foreground"
                            >
                              {tag}
                            </span>
                          ))}
                        </div>

                        {/* Time */}
                        <div className="mt-4 flex items-center gap-1 text-xs text-muted-foreground">
                          <Clock className="h-3 w-3" />
                          Updated {formatRelativeTime(sim.updatedAt)}
                        </div>
                      </CardContent>
                    </Card>
                  </Link>
                </motion.div>
              )
            })}
          </motion.div>
        ) : (
          <motion.div
            key="list"
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
          >
            <Card>
              <div className="divide-y divide-border">
                {filteredSimulations.map((sim) => {
                  const StatusIcon = statusConfig[sim.status].icon
                  const progress = (sim.currentStep / sim.totalSteps) * 100

                  return (
                    <Link
                      key={sim.id}
                      to={`/simulations/${sim.id}`}
                      className="flex items-center gap-4 p-4 transition-colors hover:bg-accent/50"
                    >
                      <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-secondary">
                        <StatusIcon
                          className={cn('h-5 w-5', statusConfig[sim.status].color)}
                        />
                      </div>

                      <div className="flex-1 min-w-0">
                        <div className="flex items-center gap-2">
                          <h3 className="font-medium truncate">{sim.name}</h3>
                          <span
                            className={cn(
                              'text-xs font-medium px-2 py-0.5 rounded-full',
                              sim.status === 'running' && 'bg-gauss-500/10 text-gauss-500',
                              sim.status === 'paused' && 'bg-yellow-500/10 text-yellow-500',
                              sim.status === 'completed' && 'bg-cyber-500/10 text-cyber-500',
                              sim.status === 'error' && 'bg-destructive/10 text-destructive',
                              sim.status === 'stopped' && 'bg-muted text-muted-foreground'
                            )}
                          >
                            {statusConfig[sim.status].label}
                          </span>
                        </div>
                        <p className="text-sm text-muted-foreground truncate">
                          {sim.description}
                        </p>
                      </div>

                      <div className="hidden md:flex items-center gap-6">
                        <div className="text-center">
                          <p className="text-sm font-medium">{formatCompact(sim.agentCount)}</p>
                          <p className="text-xs text-muted-foreground">Agents</p>
                        </div>
                        <div className="text-center">
                          <p className="text-sm font-medium">{Math.round(progress)}%</p>
                          <p className="text-xs text-muted-foreground">Progress</p>
                        </div>
                        <div className="text-center min-w-[80px]">
                          <p className="text-sm font-medium">{formatRelativeTime(sim.updatedAt)}</p>
                          <p className="text-xs text-muted-foreground">Updated</p>
                        </div>
                      </div>

                      <Button variant="ghost" size="icon">
                        <MoreVertical className="h-4 w-4" />
                      </Button>
                    </Link>
                  )
                })}
              </div>
            </Card>
          </motion.div>
        )}
      </AnimatePresence>

      {filteredSimulations.length === 0 && (
        <div className="text-center py-12">
          <AlertCircle className="mx-auto h-12 w-12 text-muted-foreground" />
          <h3 className="mt-4 text-lg font-semibold">No simulations found</h3>
          <p className="text-muted-foreground">
            Try adjusting your search or filter criteria
          </p>
        </div>
      )}
    </div>
  )
}
