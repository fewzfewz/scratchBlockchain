// Page wrapper. The background glow lives on <body> (index.css), so pages
// sit on one calm, static canvas instead of their own animated blobs.
export default function PageShell({ children, className = '' }) {
  return <div className={`relative min-h-screen ${className}`}>{children}</div>
}
