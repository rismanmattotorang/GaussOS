import { useState, useRef, useEffect } from 'react'
import { motion } from 'framer-motion'
import {
  Grid3X3,
  Layers,
  Share2,
  Map,
  Plus,
  ZoomIn,
  ZoomOut,
  Maximize2,
  RotateCcw,
  Eye,
  EyeOff,
} from 'lucide-react'
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { cn, formatCompact } from '@/lib/utils'

interface SpaceType {
  id: string
  name: string
  description: string
  icon: typeof Grid3X3
  dimensions: string
  agentCount: number
  color: string
  bgColor: string
}

const spaceTypes: SpaceType[] = [
  {
    id: 'grid',
    name: 'Grid Space',
    description: '2D/3D discrete grid with efficient neighbor lookup',
    icon: Grid3X3,
    dimensions: '256 × 256',
    agentCount: 25000,
    color: 'text-twin-500',
    bgColor: 'bg-twin-500/10',
  },
  {
    id: 'continuous',
    name: 'Continuous Space',
    description: 'Floating-point coordinate space with spatial indexing',
    icon: Map,
    dimensions: '1000 × 1000',
    agentCount: 12500,
    color: 'text-cyber-500',
    bgColor: 'bg-cyber-500/10',
  },
  {
    id: 'graph',
    name: 'Graph Space',
    description: 'Network topology with weighted edges',
    icon: Share2,
    dimensions: '2,450 nodes',
    agentCount: 8500,
    color: 'text-gauss-500',
    bgColor: 'bg-gauss-500/10',
  },
  {
    id: 'layered',
    name: 'Layered Space',
    description: 'Multi-layer composite space with cross-layer interactions',
    icon: Layers,
    dimensions: '3 layers',
    agentCount: 45000,
    color: 'text-orange-500',
    bgColor: 'bg-orange-500/10',
  },
]

export default function SpacesPage() {
  const [selectedSpace, setSelectedSpace] = useState<string>('grid')
  const canvasRef = useRef<HTMLCanvasElement>(null)
  const [showHeatmap, setShowHeatmap] = useState(false)
  const [zoom, setZoom] = useState(1)

  // Simple visualization demo
  useEffect(() => {
    const canvas = canvasRef.current
    if (!canvas) return

    const ctx = canvas.getContext('2d')
    if (!ctx) return

    const width = canvas.width
    const height = canvas.height

    // Draw background
    ctx.fillStyle = '#0a0a0a'
    ctx.fillRect(0, 0, width, height)

    // Draw grid
    ctx.strokeStyle = '#1a1a2e'
    ctx.lineWidth = 1
    const gridSize = 20 * zoom

    for (let x = 0; x < width; x += gridSize) {
      ctx.beginPath()
      ctx.moveTo(x, 0)
      ctx.lineTo(x, height)
      ctx.stroke()
    }
    for (let y = 0; y < height; y += gridSize) {
      ctx.beginPath()
      ctx.moveTo(0, y)
      ctx.lineTo(width, y)
      ctx.stroke()
    }

    // Draw agents as points
    const agentCount = 500
    for (let i = 0; i < agentCount; i++) {
      const x = Math.random() * width
      const y = Math.random() * height
      const hue = showHeatmap ? 0 + (y / height) * 60 : 260 + Math.random() * 40
      
      ctx.beginPath()
      ctx.arc(x, y, 2 * zoom, 0, Math.PI * 2)
      ctx.fillStyle = `hsla(${hue}, 70%, 60%, 0.8)`
      ctx.fill()
    }

    // Draw connections for selected agents
    ctx.strokeStyle = 'rgba(139, 92, 246, 0.2)'
    ctx.lineWidth = 0.5
    for (let i = 0; i < 50; i++) {
      const x1 = Math.random() * width
      const y1 = Math.random() * height
      const x2 = x1 + (Math.random() - 0.5) * 100
      const y2 = y1 + (Math.random() - 0.5) * 100
      ctx.beginPath()
      ctx.moveTo(x1, y1)
      ctx.lineTo(x2, y2)
      ctx.stroke()
    }
  }, [zoom, showHeatmap, selectedSpace])

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
        <div>
          <h1 className="text-3xl font-bold">Spaces</h1>
          <p className="text-muted-foreground">
            Configure and visualize simulation spaces
          </p>
        </div>
        <Button variant="gradient">
          <Plus className="mr-2 h-4 w-4" />
          Create Space
        </Button>
      </div>

      {/* Space Types */}
      <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
        {spaceTypes.map((space, i) => (
          <motion.div
            key={space.id}
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ delay: i * 0.05 }}
          >
            <Card
              className={cn(
                'card-hover cursor-pointer transition-all',
                selectedSpace === space.id && 'ring-2 ring-primary'
              )}
              onClick={() => setSelectedSpace(space.id)}
            >
              <CardContent className="p-4">
                <div className="flex items-center gap-3">
                  <div className={cn('rounded-lg p-2', space.bgColor)}>
                    <space.icon className={cn('h-5 w-5', space.color)} />
                  </div>
                  <div className="flex-1 min-w-0">
                    <h3 className="font-medium truncate">{space.name}</h3>
                    <p className="text-xs text-muted-foreground">{space.dimensions}</p>
                  </div>
                </div>
                <p className="mt-2 text-sm text-muted-foreground line-clamp-2">
                  {space.description}
                </p>
                <p className="mt-2 text-xs text-muted-foreground">
                  {formatCompact(space.agentCount)} agents
                </p>
              </CardContent>
            </Card>
          </motion.div>
        ))}
      </div>

      {/* Visualization */}
      <Card>
        <CardHeader className="flex flex-row items-center justify-between">
          <div>
            <CardTitle>Space Visualization</CardTitle>
            <CardDescription>
              Live view of {spaceTypes.find((s) => s.id === selectedSpace)?.name}
            </CardDescription>
          </div>
          <div className="flex items-center gap-2">
            <Button
              variant={showHeatmap ? 'secondary' : 'outline'}
              size="sm"
              onClick={() => setShowHeatmap(!showHeatmap)}
            >
              {showHeatmap ? <Eye className="mr-1 h-4 w-4" /> : <EyeOff className="mr-1 h-4 w-4" />}
              Heatmap
            </Button>
            <div className="flex items-center border border-border rounded-lg">
              <Button
                variant="ghost"
                size="icon-sm"
                onClick={() => setZoom(Math.max(0.5, zoom - 0.25))}
              >
                <ZoomOut className="h-4 w-4" />
              </Button>
              <span className="px-2 text-sm tabular-nums">{Math.round(zoom * 100)}%</span>
              <Button
                variant="ghost"
                size="icon-sm"
                onClick={() => setZoom(Math.min(2, zoom + 0.25))}
              >
                <ZoomIn className="h-4 w-4" />
              </Button>
            </div>
            <Button variant="outline" size="icon-sm" onClick={() => setZoom(1)}>
              <RotateCcw className="h-4 w-4" />
            </Button>
            <Button variant="outline" size="icon-sm">
              <Maximize2 className="h-4 w-4" />
            </Button>
          </div>
        </CardHeader>
        <CardContent>
          <div className="relative rounded-lg overflow-hidden bg-black border border-border">
            <canvas
              ref={canvasRef}
              width={800}
              height={500}
              className="w-full h-[500px]"
            />
            {/* Overlay stats */}
            <div className="absolute top-4 left-4 space-y-2">
              <div className="glass-dark rounded-lg px-3 py-2 text-sm">
                <span className="text-muted-foreground">Agents: </span>
                <span className="font-medium text-white">500</span>
              </div>
              <div className="glass-dark rounded-lg px-3 py-2 text-sm">
                <span className="text-muted-foreground">Step: </span>
                <span className="font-medium text-white">12,450</span>
              </div>
              <div className="glass-dark rounded-lg px-3 py-2 text-sm">
                <span className="text-muted-foreground">FPS: </span>
                <span className="font-medium text-gauss-400">60</span>
              </div>
            </div>

            {/* Legend */}
            <div className="absolute bottom-4 right-4 glass-dark rounded-lg px-3 py-2">
              <div className="flex items-center gap-4 text-xs">
                <div className="flex items-center gap-1.5">
                  <span className="h-2 w-2 rounded-full bg-twin-500" />
                  <span className="text-white">BDI</span>
                </div>
                <div className="flex items-center gap-1.5">
                  <span className="h-2 w-2 rounded-full bg-cyber-500" />
                  <span className="text-white">Cognitive</span>
                </div>
                <div className="flex items-center gap-1.5">
                  <span className="h-2 w-2 rounded-full bg-gauss-500" />
                  <span className="text-white">Reactive</span>
                </div>
              </div>
            </div>
          </div>
        </CardContent>
      </Card>

      {/* Space Configuration */}
      <div className="grid gap-6 lg:grid-cols-2">
        <Card>
          <CardHeader>
            <CardTitle>Spatial Indexing</CardTitle>
            <CardDescription>Active indexing structures for efficient queries</CardDescription>
          </CardHeader>
          <CardContent>
            <div className="space-y-3">
              {[
                { name: 'KD-Tree', queries: 125000, latency: '0.2ms' },
                { name: 'Grid Hash', queries: 890000, latency: '0.05ms' },
                { name: 'R*-Tree', queries: 45000, latency: '0.3ms' },
              ].map((index) => (
                <div
                  key={index.name}
                  className="flex items-center justify-between rounded-lg border border-border p-3"
                >
                  <div>
                    <p className="font-medium">{index.name}</p>
                    <p className="text-xs text-muted-foreground">
                      {formatCompact(index.queries)} queries/s
                    </p>
                  </div>
                  <div className="text-right">
                    <p className="text-sm font-medium text-gauss-500">{index.latency}</p>
                    <p className="text-xs text-muted-foreground">avg latency</p>
                  </div>
                </div>
              ))}
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>Pathfinding</CardTitle>
            <CardDescription>Active pathfinding algorithms</CardDescription>
          </CardHeader>
          <CardContent>
            <div className="space-y-3">
              {[
                { name: 'A*', paths: 12500, avgLength: 45.2 },
                { name: 'HPA*', paths: 8900, avgLength: 156.8 },
                { name: 'Flow Field', paths: 250, avgLength: 'N/A' },
              ].map((algo) => (
                <div
                  key={algo.name}
                  className="flex items-center justify-between rounded-lg border border-border p-3"
                >
                  <div>
                    <p className="font-medium">{algo.name}</p>
                    <p className="text-xs text-muted-foreground">
                      {formatCompact(algo.paths)} paths computed
                    </p>
                  </div>
                  <div className="text-right">
                    <p className="text-sm font-medium">{algo.avgLength}</p>
                    <p className="text-xs text-muted-foreground">avg length</p>
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
