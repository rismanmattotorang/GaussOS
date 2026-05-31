# GaussTwin Web UI

Modern, high-performance web interface for the GaussTwin Digital Twin Framework.

## 🚀 Features

- **Real-time Dashboard** - Monitor simulations, agents, and system metrics
- **Simulation Management** - Create, configure, and control simulations
- **Agent Catalog** - Browse, configure, and deploy agent types
- **Space Visualization** - 2D/3D visualization with spatial indexing
- **Analytics** - Performance metrics, trends, and anomaly detection
- **API Explorer** - Interactive REST API documentation and testing
- **Dark/Light Theme** - Beautiful, responsive design with theme support

## 🛠️ Tech Stack

- **Framework**: React 18 with TypeScript
- **Build Tool**: Vite 5
- **Styling**: TailwindCSS + shadcn/ui components
- **State Management**: Zustand
- **Data Fetching**: TanStack Query (React Query)
- **Routing**: React Router v6
- **Charts**: Recharts
- **3D**: Three.js / React Three Fiber
- **Animations**: Framer Motion
- **Forms**: React Hook Form + Zod
- **i18n**: i18next

## 📦 Installation

```bash
cd ui/web
npm install
```

## 🚀 Development

```bash
# Start development server
npm run dev

# Build for production
npm run build

# Preview production build
npm run preview
```

## 📁 Project Structure

```
src/
├── components/         # Reusable UI components
│   ├── ui/            # Base UI components (Button, Card, etc.)
│   └── layouts/       # Layout components
├── features/          # Feature-specific components
│   └── auth/          # Authentication components
├── hooks/             # Custom React hooks
├── lib/               # Utility libraries
│   ├── api.ts         # API client
│   ├── i18n.ts        # Internationalization
│   └── utils.ts       # Helper functions
├── pages/             # Page components
│   ├── auth/          # Login, Register
│   ├── dashboard/     # Main dashboard
│   ├── simulations/   # Simulation management
│   ├── agents/        # Agent catalog
│   ├── spaces/        # Space visualization
│   ├── analytics/     # Analytics dashboard
│   ├── settings/      # User settings
│   └── developer/     # API explorer
├── stores/            # Zustand state stores
├── types/             # TypeScript type definitions
└── utils/             # Utility functions
```

## 🎨 Design System

The UI uses a custom design system built on TailwindCSS with:

- **Brand Colors**: `gauss` (green), `twin` (purple), `cyber` (cyan)
- **Typography**: Cabinet Grotesk (headings), General Sans (body), JetBrains Mono (code)
- **Components**: Radix UI primitives with custom styling

## 🔗 API Integration

The web UI connects to the GaussTwin API server:

```typescript
// Configure API base URL in vite.config.ts
server: {
  proxy: {
    '/api': 'http://localhost:8080',
    '/ws': { target: 'ws://localhost:8080', ws: true }
  }
}
```

## 📊 Environment Variables

Create a `.env.local` file:

```env
VITE_API_URL=http://localhost:8080
VITE_WS_URL=ws://localhost:8080
```

## 🧪 Testing

```bash
# Run tests
npm run test

# Run tests with coverage
npm run test:coverage
```

## 📝 License

MIT License - see LICENSE file for details.
