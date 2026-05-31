import { motion } from 'framer-motion'

export function LoadingScreen() {
  return (
    <div className="fixed inset-0 flex items-center justify-center bg-background/80 backdrop-blur-sm">
      <div className="flex flex-col items-center gap-6">
        {/* Animated logo */}
        <div className="relative">
          <motion.div
            className="h-16 w-16 rounded-2xl bg-gradient-to-br from-twin-500 to-cyber-500"
            animate={{
              scale: [1, 1.1, 1],
              rotate: [0, 180, 360],
            }}
            transition={{
              duration: 2,
              repeat: Infinity,
              ease: 'easeInOut',
            }}
          />
          <motion.div
            className="absolute inset-0 h-16 w-16 rounded-2xl bg-gradient-to-br from-twin-500/50 to-cyber-500/50"
            animate={{
              scale: [1.2, 1, 1.2],
              opacity: [0.5, 0.8, 0.5],
            }}
            transition={{
              duration: 2,
              repeat: Infinity,
              ease: 'easeInOut',
            }}
          />
        </div>

        {/* Text */}
        <div className="flex flex-col items-center gap-2">
          <h2 className="text-xl font-semibold gradient-text">GaussTwin</h2>
          <p className="text-sm text-muted-foreground">Loading...</p>
        </div>

        {/* Progress dots */}
        <div className="flex gap-1">
          {[0, 1, 2].map((i) => (
            <motion.div
              key={i}
              className="h-2 w-2 rounded-full bg-twin-500"
              animate={{
                opacity: [0.3, 1, 0.3],
                scale: [0.8, 1.2, 0.8],
              }}
              transition={{
                duration: 1,
                repeat: Infinity,
                delay: i * 0.2,
              }}
            />
          ))}
        </div>
      </div>
    </div>
  )
}
