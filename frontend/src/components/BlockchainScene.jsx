import { Suspense, useRef } from 'react'
import { Canvas, useFrame } from '@react-three/fiber'
import { Float, Stars, MeshDistortMaterial, Icosahedron, Torus, Sphere, Ring } from '@react-three/drei'

function BlockCore() {
  const ref = useRef()
  useFrame((_, dt) => {
    if (ref.current) {
      ref.current.rotation.y += dt * 0.35
      ref.current.rotation.x += dt * 0.12
    }
  })
  return (
    <Float speed={1.8} rotationIntensity={0.35} floatIntensity={0.6}>
      <Icosahedron ref={ref} args={[1.15, 1]}>
        <MeshDistortMaterial
          color="#38bdf8"
          emissive="#0284c7"
          emissiveIntensity={0.45}
          distort={0.22}
          speed={1.8}
          roughness={0.15}
          metalness={0.85}
        />
      </Icosahedron>
    </Float>
  )
}

function ChainRing() {
  const ref = useRef()
  useFrame((_, dt) => {
    if (ref.current) ref.current.rotation.z += dt * 0.2
  })
  return (
    <Torus ref={ref} args={[2.1, 0.07, 20, 80]} rotation={[Math.PI / 2.8, 0.3, 0]}>
      <meshStandardMaterial color="#c084fc" emissive="#7c3aed" emissiveIntensity={0.35} metalness={0.9} roughness={0.1} />
    </Torus>
  )
}

function Satellites() {
  const g = useRef()
  useFrame((_, dt) => {
    if (g.current) g.current.rotation.y -= dt * 0.45
  })
  const nodes = [
    [2.4, 0.6, 0.2],
    [-2.1, -0.8, 0.5],
    [0.3, 2.0, -0.4],
    [1.2, -1.6, 0.8],
  ]
  return (
    <group ref={g}>
      {nodes.map((pos, i) => (
        <Sphere key={i} args={[0.14, 16, 16]} position={pos}>
          <meshStandardMaterial color={i % 2 ? '#34d399' : '#fbbf24'} emissive={i % 2 ? '#059669' : '#d97706'} emissiveIntensity={0.55} />
        </Sphere>
      ))}
      <Ring args={[2.35, 2.38, 64]} rotation={[Math.PI / 2, 0, 0]}>
        <meshBasicMaterial color="#38bdf8" transparent opacity={0.25} />
      </Ring>
    </group>
  )
}

function SceneContent() {
  return (
    <>
      <ambientLight intensity={0.35} />
      <pointLight position={[8, 8, 6]} intensity={1.1} color="#38bdf8" />
      <pointLight position={[-6, -4, 4]} intensity={0.55} color="#a78bfa" />
      <Stars radius={90} depth={45} count={2500} factor={2.5} fade speed={0.4} />
      <BlockCore />
      <ChainRing />
      <Satellites />
    </>
  )
}

export default function BlockchainScene({ className = '' }) {
  return (
    <div className={`pointer-events-none ${className}`} aria-hidden>
      <Canvas camera={{ position: [0, 0, 5.5], fov: 42 }} dpr={[1, 1.75]} gl={{ alpha: true, antialias: true }}>
        <Suspense fallback={null}>
          <SceneContent />
        </Suspense>
      </Canvas>
    </div>
  )
}
