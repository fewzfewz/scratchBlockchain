// Shared Web3 page wrapper with subtle cyber-grid backdrop across all routes.
export default function PageShell({ children, className = '' }) {
  return (
    <div className={`relative min-h-screen overflow-x-hidden ${className}`}>
      <div className="pointer-events-none absolute inset-x-0 top-0 h-[560px] bg-grid opacity-80 dark:opacity-60" aria-hidden />
      <div className="relative z-10">{children}</div>
    </div>
  )
}
