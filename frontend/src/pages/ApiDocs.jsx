import SwaggerUI from 'swagger-ui-react'
import 'swagger-ui-react/swagger-ui.css'

export default function ApiDocs() {
  return (
    <div className="min-h-screen">
      <div className="max-w-7xl mx-auto px-4 py-8">
        <div className="mb-6">
          <p className="text-xs uppercase tracking-widest text-blue-400 font-medium">Scratch Blockchain</p>
          <h1 className="text-3xl md:text-4xl font-bold text-slate-900 dark:text-white mt-1">RPC API Reference</h1>
          <p className="text-slate-500 dark:text-slate-400 mt-2">
            Interactive documentation for the Nebula RPC API. Try endpoints directly from your browser against <code className="text-xs">http://localhost:8545</code>.
          </p>
        </div>
        <div className="swagger-ui-wrapper">
          <SwaggerUI url="/openapi.yaml" />
        </div>
      </div>
    </div>
  )
}
