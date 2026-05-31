import { useState } from 'react'
import { motion } from 'framer-motion'
import {
  Play,
  Copy,
  ChevronRight,
  ChevronDown,
  Code,
  BookOpen,
  Zap,
  Globe,
  Lock,
  CheckCircle2,
} from 'lucide-react'
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { useToast } from '@/hooks/use-toast'
import { cn } from '@/lib/utils'

interface Endpoint {
  method: 'GET' | 'POST' | 'PUT' | 'DELETE'
  path: string
  description: string
  auth: boolean
}

interface EndpointCategory {
  name: string
  endpoints: Endpoint[]
}

const apiCategories: EndpointCategory[] = [
  {
    name: 'Simulations',
    endpoints: [
      { method: 'GET', path: '/api/v1/simulations', description: 'List all simulations', auth: true },
      { method: 'POST', path: '/api/v1/simulations', description: 'Create a new simulation', auth: true },
      { method: 'GET', path: '/api/v1/simulations/:id', description: 'Get simulation details', auth: true },
      { method: 'PUT', path: '/api/v1/simulations/:id', description: 'Update simulation', auth: true },
      { method: 'DELETE', path: '/api/v1/simulations/:id', description: 'Delete simulation', auth: true },
      { method: 'POST', path: '/api/v1/simulations/:id/start', description: 'Start simulation', auth: true },
      { method: 'POST', path: '/api/v1/simulations/:id/stop', description: 'Stop simulation', auth: true },
    ],
  },
  {
    name: 'Agents',
    endpoints: [
      { method: 'GET', path: '/api/v1/agents', description: 'List agent types', auth: true },
      { method: 'POST', path: '/api/v1/agents', description: 'Create agent type', auth: true },
      { method: 'GET', path: '/api/v1/simulations/:id/agents', description: 'List simulation agents', auth: true },
      { method: 'GET', path: '/api/v1/simulations/:id/agents/:agentId', description: 'Get agent details', auth: true },
    ],
  },
  {
    name: 'Spaces',
    endpoints: [
      { method: 'GET', path: '/api/v1/spaces', description: 'List space configurations', auth: true },
      { method: 'POST', path: '/api/v1/spaces', description: 'Create space', auth: true },
      { method: 'GET', path: '/api/v1/simulations/:id/space', description: 'Get simulation space', auth: true },
    ],
  },
  {
    name: 'Analytics',
    endpoints: [
      { method: 'GET', path: '/api/v1/analytics/overview', description: 'Get analytics overview', auth: true },
      { method: 'GET', path: '/api/v1/analytics/metrics', description: 'Get system metrics', auth: true },
      { method: 'GET', path: '/api/v1/simulations/:id/metrics', description: 'Get simulation metrics', auth: true },
    ],
  },
  {
    name: 'Authentication',
    endpoints: [
      { method: 'POST', path: '/api/v1/auth/login', description: 'User login', auth: false },
      { method: 'POST', path: '/api/v1/auth/register', description: 'User registration', auth: false },
      { method: 'POST', path: '/api/v1/auth/refresh', description: 'Refresh access token', auth: false },
      { method: 'POST', path: '/api/v1/auth/logout', description: 'User logout', auth: true },
    ],
  },
]

const methodColors: Record<string, string> = {
  GET: 'bg-gauss-500/10 text-gauss-500',
  POST: 'bg-twin-500/10 text-twin-500',
  PUT: 'bg-yellow-500/10 text-yellow-500',
  DELETE: 'bg-destructive/10 text-destructive',
}

export default function ApiExplorerPage() {
  const [expandedCategory, setExpandedCategory] = useState<string | null>('Simulations')
  const [selectedEndpoint, setSelectedEndpoint] = useState<Endpoint | null>(null)
  const [response, setResponse] = useState<string | null>(null)
  const [loading, setLoading] = useState(false)
  const { toast } = useToast()

  const handleTryIt = async () => {
    if (!selectedEndpoint) return
    setLoading(true)

    // Simulate API call
    await new Promise((resolve) => setTimeout(resolve, 1000))

    const mockResponse = {
      success: true,
      data: {
        id: 'sim_123456',
        name: 'Traffic Flow Simulation',
        status: 'running',
        agentCount: 12500,
        currentStep: 45230,
        createdAt: new Date().toISOString(),
      },
      meta: {
        requestId: 'req_abc123',
        duration: '45ms',
      },
    }

    setResponse(JSON.stringify(mockResponse, null, 2))
    setLoading(false)
  }

  const copyToClipboard = (text: string) => {
    navigator.clipboard.writeText(text)
    toast({
      title: 'Copied!',
      description: 'Code copied to clipboard',
    })
  }

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
        <div>
          <h1 className="text-3xl font-bold">API Explorer</h1>
          <p className="text-muted-foreground">
            Explore and test the GaussTwin REST API
          </p>
        </div>
        <div className="flex gap-2">
          <Button variant="outline">
            <BookOpen className="mr-2 h-4 w-4" />
            Documentation
          </Button>
          <Button variant="outline">
            <Code className="mr-2 h-4 w-4" />
            OpenAPI Spec
          </Button>
        </div>
      </div>

      {/* Quick Info */}
      <div className="grid gap-4 sm:grid-cols-3">
        <Card>
          <CardContent className="flex items-center gap-3 p-4">
            <div className="rounded-lg bg-twin-500/10 p-2">
              <Globe className="h-5 w-5 text-twin-500" />
            </div>
            <div>
              <p className="text-sm text-muted-foreground">Base URL</p>
              <code className="text-sm font-medium">https://api.gausstwin.io/v1</code>
            </div>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="flex items-center gap-3 p-4">
            <div className="rounded-lg bg-gauss-500/10 p-2">
              <Zap className="h-5 w-5 text-gauss-500" />
            </div>
            <div>
              <p className="text-sm text-muted-foreground">Rate Limit</p>
              <p className="text-sm font-medium">1000 requests/min</p>
            </div>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="flex items-center gap-3 p-4">
            <div className="rounded-lg bg-cyber-500/10 p-2">
              <Lock className="h-5 w-5 text-cyber-500" />
            </div>
            <div>
              <p className="text-sm text-muted-foreground">Authentication</p>
              <p className="text-sm font-medium">Bearer Token (JWT)</p>
            </div>
          </CardContent>
        </Card>
      </div>

      <div className="grid gap-6 lg:grid-cols-2">
        {/* Endpoints List */}
        <Card>
          <CardHeader>
            <CardTitle>Endpoints</CardTitle>
            <CardDescription>Browse available API endpoints</CardDescription>
          </CardHeader>
          <CardContent className="p-0">
            <div className="divide-y divide-border">
              {apiCategories.map((category) => (
                <div key={category.name}>
                  <button
                    onClick={() =>
                      setExpandedCategory(
                        expandedCategory === category.name ? null : category.name
                      )
                    }
                    className="flex w-full items-center justify-between px-4 py-3 hover:bg-accent transition-colors"
                  >
                    <span className="font-medium">{category.name}</span>
                    {expandedCategory === category.name ? (
                      <ChevronDown className="h-4 w-4 text-muted-foreground" />
                    ) : (
                      <ChevronRight className="h-4 w-4 text-muted-foreground" />
                    )}
                  </button>
                  {expandedCategory === category.name && (
                    <motion.div
                      initial={{ height: 0 }}
                      animate={{ height: 'auto' }}
                      exit={{ height: 0 }}
                      className="overflow-hidden"
                    >
                      <div className="space-y-1 px-4 pb-3">
                        {category.endpoints.map((endpoint) => (
                          <button
                            key={endpoint.path}
                            onClick={() => setSelectedEndpoint(endpoint)}
                            className={cn(
                              'flex w-full items-center gap-3 rounded-lg px-3 py-2 text-sm transition-colors',
                              selectedEndpoint?.path === endpoint.path
                                ? 'bg-primary/10 text-primary'
                                : 'hover:bg-accent'
                            )}
                          >
                            <span
                              className={cn(
                                'rounded px-2 py-0.5 text-xs font-medium',
                                methodColors[endpoint.method]
                              )}
                            >
                              {endpoint.method}
                            </span>
                            <span className="flex-1 truncate text-left font-mono text-xs">
                              {endpoint.path}
                            </span>
                            {endpoint.auth && <Lock className="h-3 w-3 text-muted-foreground" />}
                          </button>
                        ))}
                      </div>
                    </motion.div>
                  )}
                </div>
              ))}
            </div>
          </CardContent>
        </Card>

        {/* Request/Response */}
        <div className="space-y-6">
          {selectedEndpoint ? (
            <>
              <Card>
                <CardHeader>
                  <div className="flex items-center gap-3">
                    <span
                      className={cn(
                        'rounded px-2 py-1 text-sm font-medium',
                        methodColors[selectedEndpoint.method]
                      )}
                    >
                      {selectedEndpoint.method}
                    </span>
                    <code className="text-sm font-mono">{selectedEndpoint.path}</code>
                  </div>
                  <CardDescription>{selectedEndpoint.description}</CardDescription>
                </CardHeader>
                <CardContent className="space-y-4">
                  {selectedEndpoint.auth && (
                    <div className="flex items-center gap-2 text-sm text-muted-foreground">
                      <Lock className="h-4 w-4" />
                      <span>Requires authentication</span>
                    </div>
                  )}

                  {selectedEndpoint.path.includes(':id') && (
                    <div className="space-y-2">
                      <Label>Path Parameters</Label>
                      <Input placeholder="Simulation ID" />
                    </div>
                  )}

                  {(selectedEndpoint.method === 'POST' ||
                    selectedEndpoint.method === 'PUT') && (
                    <div className="space-y-2">
                      <Label>Request Body</Label>
                      <div className="relative">
                        <textarea
                          className="min-h-[150px] w-full rounded-lg border border-border bg-muted/50 p-3 font-mono text-sm"
                          defaultValue={JSON.stringify(
                            {
                              name: 'My Simulation',
                              description: 'A new simulation',
                              config: {},
                            },
                            null,
                            2
                          )}
                        />
                      </div>
                    </div>
                  )}

                  <Button
                    className="w-full"
                    variant="gradient"
                    onClick={handleTryIt}
                    disabled={loading}
                  >
                    {loading ? (
                      <>
                        <span className="animate-spin mr-2">⏳</span>
                        Sending...
                      </>
                    ) : (
                      <>
                        <Play className="mr-2 h-4 w-4" />
                        Try It
                      </>
                    )}
                  </Button>
                </CardContent>
              </Card>

              {response && (
                <Card>
                  <CardHeader className="flex flex-row items-center justify-between">
                    <div className="flex items-center gap-2">
                      <CheckCircle2 className="h-4 w-4 text-gauss-500" />
                      <CardTitle className="text-base">Response</CardTitle>
                      <span className="rounded bg-gauss-500/10 px-2 py-0.5 text-xs font-medium text-gauss-500">
                        200 OK
                      </span>
                    </div>
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => copyToClipboard(response)}
                    >
                      <Copy className="mr-1 h-3 w-3" />
                      Copy
                    </Button>
                  </CardHeader>
                  <CardContent>
                    <pre className="max-h-[300px] overflow-auto rounded-lg bg-black/50 p-4 text-sm">
                      <code className="text-muted-foreground">{response}</code>
                    </pre>
                  </CardContent>
                </Card>
              )}
            </>
          ) : (
            <Card>
              <CardContent className="flex flex-col items-center justify-center py-12 text-center">
                <Code className="h-12 w-12 text-muted-foreground mb-4" />
                <h3 className="text-lg font-medium">Select an Endpoint</h3>
                <p className="text-muted-foreground">
                  Choose an endpoint from the list to view details and try it out
                </p>
              </CardContent>
            </Card>
          )}
        </div>
      </div>

      {/* Code Examples */}
      <Card>
        <CardHeader>
          <CardTitle>Quick Start</CardTitle>
          <CardDescription>Example code to get started with the API</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="relative">
            <Button
              variant="ghost"
              size="sm"
              className="absolute right-2 top-2"
              onClick={() =>
                copyToClipboard(`curl -X GET "https://api.gausstwin.io/v1/simulations" \\
  -H "Authorization: Bearer YOUR_API_KEY" \\
  -H "Content-Type: application/json"`)
              }
            >
              <Copy className="h-3 w-3" />
            </Button>
            <pre className="overflow-auto rounded-lg bg-black/50 p-4 text-sm">
              <code className="text-muted-foreground">
                <span className="text-cyber-400">curl</span> -X GET{' '}
                <span className="text-gauss-400">"https://api.gausstwin.io/v1/simulations"</span> \{'\n'}
                {'  '}-H <span className="text-gauss-400">"Authorization: Bearer YOUR_API_KEY"</span> \{'\n'}
                {'  '}-H <span className="text-gauss-400">"Content-Type: application/json"</span>
              </code>
            </pre>
          </div>
        </CardContent>
      </Card>
    </div>
  )
}
