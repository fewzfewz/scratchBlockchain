import BlockchainScene from './BlockchainScene.jsx'

export default function PageShell({ children, variant = 'default', className = '' }) {
  const isHero = variant === 'hero'

  return (
    <div className={`relative min-h-screen overflow-hidden ${className}`}>
      {/* Aurora */}
      <div className="absolute -top-40 -left-40 w-[42rem] h-[42rem] rounded-full opacity-25 animate-float pointer-events-none"
        style={{ background: 'radial-gradient(circle, rgba(14,165,233,0.45), transparent 70%)' }} />
      <div className="absolute top-20 -right-40 w-[36rem] h-[36rem] rounded-full opacity-20 animate-float-alt pointer-events-none"
        style={{ background: 'radial-gradient(circle, rgba(139,92,246,0.4), transparent 70%)' }} />
      <div className="absolute -bottom-32 left-1/4 w-[34rem] h-[34rem] rounded-full opacity-15 animate-float-slow pointer-events-none"
        style={{ background: 'radial-gradient(circle, rgba(52,211,153,0.35), transparent 70%)' }} />

      {isHero ? (
        <BlockchainScene className="absolute inset-0 z-0 opacity-90 dark:opacity-75" />
      ) : (
        <div className="absolute inset-0 particle-field pointer-events-none" />
      )}

      <div className="absolute inset-0 bg-grid pointer-events-none z-[1]" />
      <div className="relative z-10">{children}</div>
    </div>
  )
}
