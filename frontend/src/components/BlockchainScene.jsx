import { Suspense, useRef } from 'react'
import { Canvas, useFrame } from '@react-three/fiber'
import {
  Float,
  Stars,
  Sparkles,
  OrbitControls,
  MeshDistortMaterial,
  Icosahedron,
  Torus,
  Sphere,
  Ring,
} from '@react-three/drei'

function BlockCore() {
  const coreRef = useRef()
  const wireRef = useRef()
  useFrame((_, dt) => {
    if (coreRef.current) {
      coreRef.current.rotation.y += dt * 0.35
      coreRef.current.rotation.x += dt * 0.14
    }
    if (wireRef.current) {
      wireRef.current.rotation.y -= dt * 0.22
      wireRef.current.rotation.z += dt * 0.1
    }
  })
  return (
    <Float speed={1.9} rotationIntensity={0.35} floatIntensity={0.55}>
      <group>
        <Icosahedron ref={coreRef} args={[1.05, 2]}>
          <MeshDistortMaterial
            color="#6d5dfc"
            emissive="#4c2ce6"
            emissiveIntensity={0.65}
            distort={0.24}
            speed={2.1}
            roughness={0.12}
            metalness={0.9}
          />
        </Icosahedron>
        <Icosahedron ref={wireRef} args={[1.38, 1]}>
          <meshBasicMaterial color="#22d3ee" wireframe transparent opacity={0.28} />
        </Icosahedron>
      </group>
    </Float>
  )
}

function ChainRings() {
  const ring1 = useRef()
  const ring2 = useRef()
  useFrame((_, dt) => {
    if (ring1.current) ring1.current.rotation.z += dt * 0.24
    if (ring2.current) ring2.current.rotation.z -= dt * 0.18
  })
  return (
    <group>
      <Torus ref={ring1} args={[2.05, 0.045, 20, 96]} rotation={[Math.PI / 2.7, 0.35, 0]}>
        <meshStandardMaterial color="#22d3ee" emissive="#0891b2" emissiveIntensity={0.65} metalness={0.92} roughness={0.1} />
      </Torus>
      <Torus ref={ring2} args={[2.5, 0.025, 16, 96]} rotation={[-Math.PI / 3.2, -0.4, 0.4]}>
        <meshStandardMaterial color="#a78bfa" emissive="#6d28d9" emissiveIntensity={0.55} metalness={0.9} roughness={0.15} />
      </Torus>
    </group>
  )
}

function ValidatorSatellites() {
  const g = useRef()
  useFrame((_, dt) => {
    if (g.current) g.current.rotation.y -= dt * 0.42
  })
  const nodes = [
    [2.35, 0.55, 0.2],
    [-2.2, -0.7, 0.45],
    [0.25, 2.05, -0.45],
    [1.15, -1.75, 0.75],
    [-1.4, 1.55, -0.6],
  ]
  return (
    <group ref={g}>
      {nodes.map((pos, i) => (
        <Sphere key={i} args={[0.12, 16, 16]} position={pos}>
          <meshStandardMaterial
            color={i % 2 === 0 ? '#34d399' : '#22d3ee'}
            emissive={i % 2 === 0 ? '#059669' : '#0284c7'}
            emissiveIntensity={0.9}
          />
        </Sphere>
      ))}
      <Ring args={[2.32, 2.35, 72]} rotation={[Math.PI / 2, 0, 0]}>
        <meshBasicMaterial color="#38bdf8" transparent opacity={0.28} />
      </Ring>
    </group>
  )
}

function SceneContent() {
  return (
    <>
      <ambientLight intensity={0.45} />
      <pointLight position={[8, 8, 6]} intensity={1.4} color="#818cf8" />
      <pointLight position={[-6, -5, 4]} intensity={0.9} color="#22d3ee" />
      <Stars radius={85} depth={45} count={2200} factor={2.6} fade speed={0.45} />
      <Sparkles count={45} scale={5.5} size={1.8} speed={0.35} opacity={0.55} color="#38bdf8" />
      <BlockCore />
      <ChainRings />
      <ValidatorSatellites />
      <OrbitControls enableZoom={false} enablePan={false} autoRotate autoRotateSpeed={0.75} />
    </>
  )
}

export default function BlockchainScene({ className = '' }) {
  return (
    <div className={`cursor-grab active:cursor-grabbing ${className}`} aria-label="Interactive 3D Nebula Consensus Core">
      <Canvas camera={{ position: [0, 0, 7], fov: 42 }} dpr={[1, 1.75]} gl={{ alpha: true, antialias: true }}>
        <Suspense fallback={null}>
          <SceneContent />
        </Suspense>
      </Canvas>
    </div>
  )
}
