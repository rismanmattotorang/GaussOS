import { Suspense, useRef, useState } from 'react'
import { Canvas, useFrame } from '@react-three/fiber'
import { OrbitControls, Grid, PerspectiveCamera, Environment } from '@react-three/drei'
import * as THREE from 'three'
import { Loader2 } from 'lucide-react'

interface Agent3DProps {
  position: [number, number, number]
  color: string
  isActive?: boolean
}

function Agent3D({ position, color, isActive = false }: Agent3DProps) {
  const meshRef = useRef<THREE.Mesh>(null)
  const [hovered, setHovered] = useState(false)

  useFrame((state) => {
    if (meshRef.current && isActive) {
      meshRef.current.position.y = position[1] + Math.sin(state.clock.elapsedTime * 2) * 0.1
    }
  })

  return (
    <mesh
      ref={meshRef}
      position={position}
      onPointerOver={() => setHovered(true)}
      onPointerOut={() => setHovered(false)}
      scale={hovered ? 1.2 : 1}
    >
      <sphereGeometry args={[0.3, 16, 16]} />
      <meshStandardMaterial
        color={color}
        emissive={isActive ? color : '#000000'}
        emissiveIntensity={isActive ? 0.3 : 0}
        metalness={0.8}
        roughness={0.2}
      />
    </mesh>
  )
}

interface Connection3DProps {
  start: [number, number, number]
  end: [number, number, number]
  color?: string
}

function Connection3D({ start, end, color = '#22d3ee' }: Connection3DProps) {
  const points = [new THREE.Vector3(...start), new THREE.Vector3(...end)]
  const lineGeometry = new THREE.BufferGeometry().setFromPoints(points)

  return (
    <line geometry={lineGeometry}>
      <lineBasicMaterial color={color} linewidth={2} transparent opacity={0.4} />
    </line>
  )
}

interface Agent3DData {
  id: string
  position: [number, number, number]
  color: string
  isActive?: boolean
}

interface Connection3DData {
  from: string
  to: string
}

interface Viewer3DProps {
  agents?: Agent3DData[]
  connections?: Connection3DData[]
  gridSize?: number
  cameraPosition?: [number, number, number]
}

export function Viewer3D({
  agents = [],
  connections = [],
  gridSize = 20,
  cameraPosition = [10, 10, 10],
}: Viewer3DProps) {
  // Create agent position map for connections
  const agentPositions = new Map(agents.map((a) => [a.id, a.position]))

  return (
    <div className="relative h-full w-full rounded-lg overflow-hidden bg-black/50">
      <Canvas>
        <Suspense
          fallback={
            <mesh>
              <boxGeometry />
              <meshBasicMaterial color="gray" />
            </mesh>
          }
        >
          <PerspectiveCamera makeDefault position={cameraPosition} />
          <OrbitControls
            enableDamping
            dampingFactor={0.05}
            minDistance={5}
            maxDistance={50}
            maxPolarAngle={Math.PI / 2}
          />

          {/* Lighting */}
          <ambientLight intensity={0.4} />
          <directionalLight position={[10, 10, 5]} intensity={0.8} castShadow />
          <pointLight position={[-10, -10, -5]} intensity={0.3} color="#22d3ee" />
          <Environment preset="city" />

          {/* Grid */}
          <Grid
            args={[gridSize, gridSize]}
            cellSize={1}
            cellThickness={0.5}
            cellColor="#444444"
            sectionSize={5}
            sectionThickness={1}
            sectionColor="#666666"
            fadeDistance={50}
            fadeStrength={1}
            followCamera={false}
          />

          {/* Agents */}
          {agents.map((agent) => (
            <Agent3D
              key={agent.id}
              position={agent.position}
              color={agent.color}
              isActive={agent.isActive}
            />
          ))}

          {/* Connections */}
          {connections.map((conn, idx) => {
            const startPos = agentPositions.get(conn.from)
            const endPos = agentPositions.get(conn.to)
            if (!startPos || !endPos) return null

            return <Connection3D key={idx} start={startPos} end={endPos} />
          })}
        </Suspense>
      </Canvas>

      {/* Loading overlay */}
      {agents.length === 0 && (
        <div className="absolute inset-0 flex items-center justify-center bg-black/50">
          <div className="text-center">
            <Loader2 className="h-8 w-8 animate-spin mx-auto text-primary" />
            <p className="mt-2 text-sm text-muted-foreground">Loading 3D view...</p>
          </div>
        </div>
      )}
    </div>
  )
}
