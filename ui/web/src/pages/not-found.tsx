import { Link } from 'react-router-dom'
import { motion } from 'framer-motion'
import { Home, ArrowLeft } from 'lucide-react'
import { Button } from '@/components/ui/button'

export default function NotFoundPage() {
  return (
    <div className="min-h-screen flex items-center justify-center bg-background relative overflow-hidden">
      {/* Background effects */}
      <div className="absolute inset-0 bg-grid opacity-20" />
      <div className="absolute inset-0 bg-mesh" />
      
      <div className="relative z-10 text-center px-4">
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          className="space-y-6"
        >
          {/* Animated 404 */}
          <div className="relative inline-block">
            <motion.h1
              className="text-[150px] sm:text-[200px] font-bold gradient-text leading-none"
              animate={{
                textShadow: [
                  '0 0 20px rgba(139, 92, 246, 0.3)',
                  '0 0 40px rgba(139, 92, 246, 0.5)',
                  '0 0 20px rgba(139, 92, 246, 0.3)',
                ],
              }}
              transition={{ duration: 2, repeat: Infinity }}
            >
              404
            </motion.h1>
            <motion.div
              className="absolute inset-0 bg-gradient-to-r from-twin-500/20 via-transparent to-cyber-500/20 blur-3xl"
              animate={{
                opacity: [0.3, 0.6, 0.3],
              }}
              transition={{ duration: 2, repeat: Infinity }}
            />
          </div>

          <div className="space-y-2">
            <h2 className="text-2xl sm:text-3xl font-bold">Page Not Found</h2>
            <p className="text-muted-foreground max-w-md mx-auto">
              The page you're looking for doesn't exist or has been moved.
              Let's get you back on track.
            </p>
          </div>

          <div className="flex flex-col sm:flex-row items-center justify-center gap-4">
            <Link to="/">
              <Button variant="gradient" size="lg">
                <Home className="mr-2 h-4 w-4" />
                Go to Dashboard
              </Button>
            </Link>
            <Button variant="outline" size="lg" onClick={() => window.history.back()}>
              <ArrowLeft className="mr-2 h-4 w-4" />
              Go Back
            </Button>
          </div>
        </motion.div>

        {/* Floating particles */}
        <div className="absolute inset-0 overflow-hidden pointer-events-none">
          {Array.from({ length: 20 }).map((_, i) => (
            <motion.div
              key={i}
              className="absolute h-2 w-2 rounded-full bg-twin-500/30"
              style={{
                left: `${Math.random() * 100}%`,
                top: `${Math.random() * 100}%`,
              }}
              animate={{
                y: [0, -30, 0],
                opacity: [0.3, 0.7, 0.3],
              }}
              transition={{
                duration: 3 + Math.random() * 2,
                repeat: Infinity,
                delay: Math.random() * 2,
              }}
            />
          ))}
        </div>
      </div>
    </div>
  )
}
