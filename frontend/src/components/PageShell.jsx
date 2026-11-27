import Web3Background from './Web3Background.jsx'

// Shared Web3 page wrapper with interactive tsparticles mesh and cyber-grid across all routes.
export default function PageShell({ children, className = '' }) {
  return (
    <div className={`relative min-h-screen overflow-x-hidden ${className}`}>
      <Web3Background />
      <div className="pointer-events-none absolute inset-x-0 top-0 h-[560px] bg-grid opacity-80 dark:opacity-70" aria-hidden />
      <div className="relative z-10">{children}</div>
    </div>
  )
}
