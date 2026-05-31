import { useState } from 'react'
import { motion } from 'framer-motion'
import {
  Plus,
  Search,
  Brain,
  Cog,
  Zap,
  Users,
  Code,
  ChevronRight,
  ExternalLink,
  Copy,
  MoreVertical,
} from 'lucide-react'
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { cn } from '@/lib/utils'

interface AgentType {
  id: string
  name: string
  description: string
  category: 'cognitive' | 'reactive' | 'bdi' | 'custom'
  instanceCount: number
  parameters: number
  icon: typeof Brain
  color: string
  bgColor: string
}

const agentTypes: AgentType[] = [
  {
    id: 'bdi',
    name: 'BDI Agent',
    description: 'Belief-Desire-Intention agents with goal-directed reasoning and planning capabilities',
    category: 'bdi',
    instanceCount: 15420,
    parameters: 24,
    icon: Brain,
    color: 'text-twin-500',
    bgColor: 'bg-twin-500/10',
  },
  {
    id: 'cognitive',
    name: 'Cognitive Agent',
    description: 'Neural network-based decision making with learning and adaptation',
    category: 'cognitive',
    instanceCount: 8750,
    parameters: 48,
    icon: Cog,
    color: 'text-cyber-500',
    bgColor: 'bg-cyber-500/10',
  },
  {
    id: 'reactive',
    name: 'Reactive Agent',
    description: 'High-performance stimulus-response agents with SIMD optimization',
    category: 'reactive',
    instanceCount: 125000,
    parameters: 12,
    icon: Zap,
    color: 'text-gauss-500',
    bgColor: 'bg-gauss-500/10',
  },
  {
    id: 'swarm',
    name: 'Swarm Agent',
    description: 'Collective behavior agents for flocking, swarming, and emergent patterns',
    category: 'reactive',
    instanceCount: 50000,
    parameters: 16,
    icon: Users,
    color: 'text-orange-500',
    bgColor: 'bg-orange-500/10',
  },
]

const agentTemplates = [
  { name: 'Traffic Vehicle', category: 'reactive', instances: 12500 },
  { name: 'Pedestrian', category: 'cognitive', instances: 5000 },
  { name: 'Supply Chain Node', category: 'bdi', instances: 850 },
  { name: 'Financial Trader', category: 'cognitive', instances: 1200 },
  { name: 'Crowd Member', category: 'reactive', instances: 50000 },
  { name: 'Smart Grid Node', category: 'bdi', instances: 2500 },
]

export default function AgentsPage() {
  const [searchQuery, setSearchQuery] = useState('')
  const [selectedCategory, setSelectedCategory] = useState<string | null>(null)

  const filteredAgents = agentTypes.filter(
    (agent) =>
      agent.name.toLowerCase().includes(searchQuery.toLowerCase()) &&
      (!selectedCategory || agent.category === selectedCategory)
  )

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
        <div>
          <h1 className="text-3xl font-bold">Agent Catalog</h1>
          <p className="text-muted-foreground">
            Browse, configure, and deploy agent types for your simulations
          </p>
        </div>
        <Button variant="gradient">
          <Plus className="mr-2 h-4 w-4" />
          Create Agent Type
        </Button>
      </div>

      {/* Search & Filters */}
      <div className="flex flex-col gap-4 sm:flex-row sm:items-center">
        <div className="relative flex-1 max-w-md">
          <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
          <Input
            placeholder="Search agent types..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="pl-9"
          />
        </div>
        <div className="flex gap-2">
          {['all', 'bdi', 'cognitive', 'reactive'].map((cat) => (
            <Button
              key={cat}
              variant={selectedCategory === cat || (cat === 'all' && !selectedCategory) ? 'secondary' : 'outline'}
              size="sm"
              onClick={() => setSelectedCategory(cat === 'all' ? null : cat)}
            >
              {cat === 'all' ? 'All' : cat.charAt(0).toUpperCase() + cat.slice(1)}
            </Button>
          ))}
        </div>
      </div>

      {/* Agent Types Grid */}
      <div className="grid gap-4 md:grid-cols-2">
        {filteredAgents.map((agent, i) => (
          <motion.div
            key={agent.id}
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ delay: i * 0.1 }}
          >
            <Card className="card-hover h-full">
              <CardContent className="p-6">
                <div className="flex items-start gap-4">
                  <div className={cn('rounded-xl p-3', agent.bgColor)}>
                    <agent.icon className={cn('h-6 w-6', agent.color)} />
                  </div>
                  <div className="flex-1">
                    <div className="flex items-center justify-between">
                      <h3 className="text-lg font-semibold">{agent.name}</h3>
                      <Button variant="ghost" size="icon-sm">
                        <MoreVertical className="h-4 w-4" />
                      </Button>
                    </div>
                    <p className="mt-1 text-sm text-muted-foreground">{agent.description}</p>
                    
                    <div className="mt-4 flex items-center gap-4 text-sm">
                      <span className="text-muted-foreground">
                        <strong className="text-foreground">{agent.instanceCount.toLocaleString()}</strong> instances
                      </span>
                      <span className="text-muted-foreground">
                        <strong className="text-foreground">{agent.parameters}</strong> parameters
                      </span>
                    </div>

                    <div className="mt-4 flex gap-2">
                      <Button size="sm" variant="outline" className="flex-1">
                        <Code className="mr-1 h-3 w-3" />
                        View Code
                      </Button>
                      <Button size="sm" className="flex-1">
                        Configure
                        <ChevronRight className="ml-1 h-3 w-3" />
                      </Button>
                    </div>
                  </div>
                </div>
              </CardContent>
            </Card>
          </motion.div>
        ))}
      </div>

      {/* Agent Templates */}
      <Card>
        <CardHeader>
          <CardTitle>Active Agent Templates</CardTitle>
          <CardDescription>Pre-configured agent templates used in your simulations</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="divide-y divide-border">
            {agentTemplates.map((template) => (
              <div
                key={template.name}
                className="flex items-center justify-between py-3 first:pt-0 last:pb-0"
              >
                <div className="flex items-center gap-3">
                  <div
                    className={cn(
                      'h-8 w-8 rounded-lg flex items-center justify-center',
                      template.category === 'bdi' && 'bg-twin-500/10',
                      template.category === 'cognitive' && 'bg-cyber-500/10',
                      template.category === 'reactive' && 'bg-gauss-500/10'
                    )}
                  >
                    {template.category === 'bdi' && <Brain className="h-4 w-4 text-twin-500" />}
                    {template.category === 'cognitive' && <Cog className="h-4 w-4 text-cyber-500" />}
                    {template.category === 'reactive' && <Zap className="h-4 w-4 text-gauss-500" />}
                  </div>
                  <div>
                    <p className="font-medium">{template.name}</p>
                    <p className="text-xs text-muted-foreground capitalize">{template.category}</p>
                  </div>
                </div>
                <div className="flex items-center gap-4">
                  <span className="text-sm text-muted-foreground">
                    {template.instances.toLocaleString()} active
                  </span>
                  <Button variant="ghost" size="sm">
                    <ExternalLink className="h-4 w-4" />
                  </Button>
                </div>
              </div>
            ))}
          </div>
        </CardContent>
      </Card>

      {/* Quick Code */}
      <Card>
        <CardHeader>
          <CardTitle>Quick Start</CardTitle>
          <CardDescription>Create agents programmatically</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="relative rounded-lg bg-black/50 p-4 font-mono text-sm">
            <Button
              variant="ghost"
              size="icon-sm"
              className="absolute right-2 top-2"
            >
              <Copy className="h-4 w-4" />
            </Button>
            <pre className="text-muted-foreground overflow-x-auto">
              <span className="text-twin-400">use</span> gausstwin::agent::{'{'}BDIAgent, AgentConfig{'}'};{'\n'}
              <span className="text-twin-400">use</span> gausstwin::space::ContinuousSpace;{'\n\n'}
              <span className="text-muted-foreground">// Create agent configuration</span>{'\n'}
              <span className="text-twin-400">let</span> config = AgentConfig::builder(){'\n'}
              {'    '}.name(<span className="text-gauss-400">"TrafficVehicle"</span>){'\n'}
              {'    '}.initial_beliefs(vec![<span className="text-gauss-400">"road_clear"</span>]){'\n'}
              {'    '}.goals(vec![Goal::reach_destination()]){'\n'}
              {'    '}.build();{'\n\n'}
              <span className="text-muted-foreground">// Spawn agents in space</span>{'\n'}
              <span className="text-twin-400">let</span> agents = space.spawn_agents::{'<'}BDIAgent{'>'}(config, <span className="text-cyber-400">1000</span>);
            </pre>
          </div>
        </CardContent>
      </Card>
    </div>
  )
}
