import { useState } from 'react'
import { motion } from 'framer-motion'
import {
  BarChart3,
  TrendingUp,
  TrendingDown,
  Activity,
  AlertTriangle,
  FileDown,
  RefreshCw,
} from 'lucide-react'
import {
  BarChart,
  Bar,
  AreaChart,
  Area,
  PieChart,
  Pie,
  Cell,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
} from 'recharts'
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { cn, formatCompact } from '@/lib/utils'

const performanceData = Array.from({ length: 30 }, (_, i) => ({
  day: i + 1,
  agents: Math.floor(10000 + Math.random() * 5000),
  events: Math.floor(50000 + Math.random() * 20000),
  throughput: Math.floor(1000 + Math.random() * 500),
}))

const categoryData = [
  { name: 'Traffic', value: 35, color: '#8b5cf6' },
  { name: 'Supply Chain', value: 25, color: '#22d3ee' },
  { name: 'Financial', value: 20, color: '#22c566' },
  { name: 'Urban', value: 15, color: '#f59e0b' },
  { name: 'Other', value: 5, color: '#6b7280' },
]

const anomalies = [
  { id: 1, type: 'High CPU', severity: 'warning', simulation: 'Traffic Flow v2', time: '15m ago' },
  { id: 2, type: 'Memory Spike', severity: 'error', simulation: 'Supply Chain', time: '1h ago' },
  { id: 3, type: 'Deadlock Detected', severity: 'error', simulation: 'Financial Model', time: '2h ago' },
  { id: 4, type: 'Slow Convergence', severity: 'info', simulation: 'Crowd Dynamics', time: '4h ago' },
]

export default function AnalyticsPage() {
  const [dateRange, setDateRange] = useState('30d')

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
        <div>
          <h1 className="text-3xl font-bold">Analytics</h1>
          <p className="text-muted-foreground">
            Performance metrics and insights across your simulations
          </p>
        </div>
        <div className="flex gap-2">
          <div className="flex items-center border border-border rounded-lg">
            {['24h', '7d', '30d', '90d'].map((range) => (
              <Button
                key={range}
                variant={dateRange === range ? 'secondary' : 'ghost'}
                size="sm"
                onClick={() => setDateRange(range)}
              >
                {range}
              </Button>
            ))}
          </div>
          <Button variant="outline" size="icon">
            <RefreshCw className="h-4 w-4" />
          </Button>
          <Button variant="outline">
            <FileDown className="mr-2 h-4 w-4" />
            Export
          </Button>
        </div>
      </div>

      {/* Summary Stats */}
      <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
        {[
          {
            label: 'Total Simulations',
            value: 156,
            change: '+12%',
            increasing: true,
            icon: BarChart3,
            color: 'text-twin-500',
          },
          {
            label: 'Avg. Throughput',
            value: '1.2K/s',
            change: '+8%',
            increasing: true,
            icon: Activity,
            color: 'text-cyber-500',
          },
          {
            label: 'Success Rate',
            value: '94.5%',
            change: '-2%',
            increasing: false,
            icon: TrendingUp,
            color: 'text-gauss-500',
          },
          {
            label: 'Anomalies',
            value: 4,
            change: '+2',
            increasing: true,
            icon: AlertTriangle,
            color: 'text-orange-500',
          },
        ].map((stat, i) => (
          <motion.div
            key={stat.label}
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ delay: i * 0.1 }}
          >
            <Card>
              <CardContent className="p-4">
                <div className="flex items-center justify-between">
                  <stat.icon className={cn('h-5 w-5', stat.color)} />
                  <span
                    className={cn(
                      'flex items-center text-sm',
                      stat.increasing && stat.label !== 'Anomalies' ? 'text-gauss-500' : 'text-red-500'
                    )}
                  >
                    {stat.increasing ? (
                      <TrendingUp className="mr-1 h-3 w-3" />
                    ) : (
                      <TrendingDown className="mr-1 h-3 w-3" />
                    )}
                    {stat.change}
                  </span>
                </div>
                <p className="mt-2 text-2xl font-bold">{stat.value}</p>
                <p className="text-sm text-muted-foreground">{stat.label}</p>
              </CardContent>
            </Card>
          </motion.div>
        ))}
      </div>

      {/* Charts Row */}
      <div className="grid gap-6 lg:grid-cols-2">
        {/* Performance Over Time */}
        <Card>
          <CardHeader>
            <CardTitle>Performance Over Time</CardTitle>
            <CardDescription>Agent activity and event throughput</CardDescription>
          </CardHeader>
          <CardContent>
            <div className="h-[300px]">
              <ResponsiveContainer width="100%" height="100%">
                <AreaChart data={performanceData}>
                  <defs>
                    <linearGradient id="agentsGradient" x1="0" y1="0" x2="0" y2="1">
                      <stop offset="5%" stopColor="#8b5cf6" stopOpacity={0.3} />
                      <stop offset="95%" stopColor="#8b5cf6" stopOpacity={0} />
                    </linearGradient>
                    <linearGradient id="eventsGradient" x1="0" y1="0" x2="0" y2="1">
                      <stop offset="5%" stopColor="#22d3ee" stopOpacity={0.3} />
                      <stop offset="95%" stopColor="#22d3ee" stopOpacity={0} />
                    </linearGradient>
                  </defs>
                  <CartesianGrid strokeDasharray="3 3" className="stroke-border" />
                  <XAxis
                    dataKey="day"
                    tick={{ fill: 'hsl(var(--muted-foreground))', fontSize: 12 }}
                  />
                  <YAxis
                    tickFormatter={(v) => formatCompact(v)}
                    tick={{ fill: 'hsl(var(--muted-foreground))', fontSize: 12 }}
                  />
                  <Tooltip
                    contentStyle={{
                      backgroundColor: 'hsl(var(--card))',
                      border: '1px solid hsl(var(--border))',
                      borderRadius: '8px',
                    }}
                    formatter={(value: number) => formatCompact(value)}
                  />
                  <Area
                    type="monotone"
                    dataKey="agents"
                    stroke="#8b5cf6"
                    fill="url(#agentsGradient)"
                    name="Agents"
                  />
                  <Area
                    type="monotone"
                    dataKey="events"
                    stroke="#22d3ee"
                    fill="url(#eventsGradient)"
                    name="Events"
                  />
                </AreaChart>
              </ResponsiveContainer>
            </div>
          </CardContent>
        </Card>

        {/* Simulation Categories */}
        <Card>
          <CardHeader>
            <CardTitle>Simulation Categories</CardTitle>
            <CardDescription>Distribution by domain</CardDescription>
          </CardHeader>
          <CardContent>
            <div className="h-[300px] flex items-center">
              <ResponsiveContainer width="100%" height="100%">
                <PieChart>
                  <Pie
                    data={categoryData}
                    cx="50%"
                    cy="50%"
                    innerRadius={60}
                    outerRadius={100}
                    paddingAngle={2}
                    dataKey="value"
                  >
                    {categoryData.map((entry, index) => (
                      <Cell key={`cell-${index}`} fill={entry.color} />
                    ))}
                  </Pie>
                  <Tooltip
                    contentStyle={{
                      backgroundColor: 'hsl(var(--card))',
                      border: '1px solid hsl(var(--border))',
                      borderRadius: '8px',
                    }}
                    formatter={(value: number) => `${value}%`}
                  />
                </PieChart>
              </ResponsiveContainer>
              <div className="space-y-2">
                {categoryData.map((item) => (
                  <div key={item.name} className="flex items-center gap-2">
                    <div
                      className="h-3 w-3 rounded-full"
                      style={{ backgroundColor: item.color }}
                    />
                    <span className="text-sm">{item.name}</span>
                    <span className="text-sm text-muted-foreground">{item.value}%</span>
                  </div>
                ))}
              </div>
            </div>
          </CardContent>
        </Card>
      </div>

      {/* Throughput Chart */}
      <Card>
        <CardHeader>
          <CardTitle>Throughput Trends</CardTitle>
          <CardDescription>Steps per second over the selected period</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="h-[250px]">
            <ResponsiveContainer width="100%" height="100%">
              <BarChart data={performanceData}>
                <CartesianGrid strokeDasharray="3 3" className="stroke-border" />
                <XAxis
                  dataKey="day"
                  tick={{ fill: 'hsl(var(--muted-foreground))', fontSize: 12 }}
                />
                <YAxis tick={{ fill: 'hsl(var(--muted-foreground))', fontSize: 12 }} />
                <Tooltip
                  contentStyle={{
                    backgroundColor: 'hsl(var(--card))',
                    border: '1px solid hsl(var(--border))',
                    borderRadius: '8px',
                  }}
                />
                <Bar
                  dataKey="throughput"
                  fill="#22c566"
                  radius={[4, 4, 0, 0]}
                  name="Steps/sec"
                />
              </BarChart>
            </ResponsiveContainer>
          </div>
        </CardContent>
      </Card>

      {/* Anomaly Detection */}
      <Card>
        <CardHeader>
          <CardTitle>Anomaly Detection</CardTitle>
          <CardDescription>Automatically detected issues across simulations</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="space-y-3">
            {anomalies.map((anomaly) => (
              <div
                key={anomaly.id}
                className={cn(
                  'flex items-center justify-between rounded-lg border p-4',
                  anomaly.severity === 'error' && 'border-destructive/50 bg-destructive/5',
                  anomaly.severity === 'warning' && 'border-yellow-500/50 bg-yellow-500/5',
                  anomaly.severity === 'info' && 'border-border'
                )}
              >
                <div className="flex items-center gap-3">
                  <AlertTriangle
                    className={cn(
                      'h-5 w-5',
                      anomaly.severity === 'error' && 'text-destructive',
                      anomaly.severity === 'warning' && 'text-yellow-500',
                      anomaly.severity === 'info' && 'text-muted-foreground'
                    )}
                  />
                  <div>
                    <p className="font-medium">{anomaly.type}</p>
                    <p className="text-sm text-muted-foreground">{anomaly.simulation}</p>
                  </div>
                </div>
                <div className="flex items-center gap-4">
                  <span className="text-sm text-muted-foreground">{anomaly.time}</span>
                  <Button variant="outline" size="sm">
                    Investigate
                  </Button>
                </div>
              </div>
            ))}
          </div>
        </CardContent>
      </Card>
    </div>
  )
}
