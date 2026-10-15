// Shared UI kit for the "Aurora" design system. Pages compose these instead
// of hand-rolling headers, cards, inputs and buttons, so they stay consistent.
import { Loader2, CheckCircle2, AlertCircle, Info, X } from 'lucide-react'
import PageShell from './PageShell.jsx'

export const inputClass =
  'w-full h-11 px-3.5 rounded-xl bg-white dark:bg-slate-950/60 border border-slate-200 dark:border-slate-800 text-sm text-slate-900 dark:text-white placeholder-slate-400 focus:outline-none focus:ring-2 focus:ring-blue-500/40 focus:border-blue-500/60 transition disabled:opacity-60'
export const textareaClass =
  'w-full px-3.5 py-3 rounded-xl bg-white dark:bg-slate-950/60 border border-slate-200 dark:border-slate-800 text-sm text-slate-900 dark:text-white placeholder-slate-400 focus:outline-none focus:ring-2 focus:ring-blue-500/40 focus:border-blue-500/60 transition'

const BUTTONS = {
  primary: 'bg-gradient-to-r from-blue-600 to-cyan-600 text-white shadow-sm shadow-blue-600/20 hover:opacity-95',
  secondary: 'glass text-slate-900 dark:text-white',
  ghost: 'text-slate-600 dark:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-800',
  danger: 'bg-rose-600 text-white hover:bg-rose-500',
  success: 'bg-emerald-600 text-white hover:bg-emerald-500',
}
const SIZES = { sm: 'h-9 px-3 text-sm', md: 'h-11 px-5 text-sm' }

export function buttonClass(variant = 'primary', size = 'md', extra = '') {
  return `inline-flex items-center justify-center gap-2 rounded-xl font-semibold transition disabled:opacity-50 disabled:cursor-not-allowed ${BUTTONS[variant]} ${SIZES[size]} ${extra}`
}

export function Button({ variant = 'primary', size = 'md', loading = false, icon: Icon, children, className = '', ...props }) {
  return (
    <button type="button" {...props} disabled={props.disabled || loading} className={buttonClass(variant, size, className)}>
      {loading ? <Loader2 className="w-4 h-4 animate-spin" /> : Icon ? <Icon className="w-4 h-4" /> : null}
      {children}
    </button>
  )
}

/** Page frame: background + centered column. */
export function Page({ children, width = 'max-w-6xl' }) {
  return (
    <PageShell>
      <div className={`${width} mx-auto px-4 py-12`}>{children}</div>
    </PageShell>
  )
}

/** Left-aligned page header: eyebrow, title, description, actions. */
export function PageHeader({ icon: Icon, eyebrow, title, description, actions, children }) {
  return (
    <header className="flex flex-wrap items-end justify-between gap-6 mb-8">
      <div className="min-w-0">
        {eyebrow && (
          <div className="flex items-center gap-2 text-sm font-medium text-blue-600 dark:text-blue-300">
            {Icon && <Icon className="w-4 h-4" />} {eyebrow}
          </div>
        )}
        <h1 className="mt-2 text-3xl sm:text-4xl font-semibold text-slate-900 dark:text-white">{title}</h1>
        {description && <p className="mt-2 text-slate-500 max-w-2xl">{description}</p>}
        {children}
      </div>
      {actions && <div className="flex flex-wrap items-center gap-2">{actions}</div>}
    </header>
  )
}

export function Card({ title, description, actions, children, className = '', padded = true, icon: Icon }) {
  return (
    <section className={`rounded-2xl glass-strong ${padded ? 'p-5 sm:p-6' : ''} ${className}`}>
      {(title || actions) && (
        <div className={`flex flex-wrap items-start justify-between gap-3 ${padded ? 'mb-4' : 'p-5 sm:p-6 pb-0 mb-4'}`}>
          <div className="min-w-0">
            {title && (
              <h2 className="flex items-center gap-2 text-base font-semibold text-slate-900 dark:text-white">
                {Icon && <Icon className="w-4 h-4 text-blue-600 dark:text-blue-300" />}{title}
              </h2>
            )}
            {description && <p className="mt-1 text-sm text-slate-500">{description}</p>}
          </div>
          {actions && <div className="flex items-center gap-2">{actions}</div>}
        </div>
      )}
      {children}
    </section>
  )
}

export function Field({ label, hint, children, className = '' }) {
  return (
    <label className={`block ${className}`}>
      <span className="block text-xs font-medium text-slate-600 dark:text-slate-300 mb-1.5">{label}</span>
      {children}
      {hint && <span className="block mt-1.5 text-xs text-slate-500">{hint}</span>}
    </label>
  )
}

const NOTICE = {
  ok: ['bg-emerald-500/10 text-emerald-700 dark:text-emerald-400', CheckCircle2],
  err: ['bg-rose-500/10 text-rose-700 dark:text-rose-400', AlertCircle],
  info: ['bg-blue-500/10 text-blue-700 dark:text-blue-300', Info],
  warn: ['bg-amber-500/10 text-amber-700 dark:text-amber-400', AlertCircle],
}

/** Status message. `type` accepts ok/success, err/error, info, warn. */
export function Notice({ type = 'info', children, className = '' }) {
  if (!children) return null
  const key = type === 'success' ? 'ok' : type === 'error' ? 'err' : NOTICE[type] ? type : 'info'
  const [cls, Icon] = NOTICE[key]
  return (
    <div role="status" className={`flex items-start gap-2 p-3 rounded-xl text-sm ${cls} ${className}`}>
      <Icon className="w-4 h-4 mt-0.5 shrink-0" />
      <span className="min-w-0 break-words">{children}</span>
    </div>
  )
}

export function Stat({ label, value, sub, icon: Icon }) {
  return (
    <div className="rounded-2xl glass-strong p-4">
      <div className="flex items-center gap-1.5 text-[11px] uppercase tracking-wider text-slate-500">
        {Icon && <Icon className="w-3.5 h-3.5" />}{label}
      </div>
      <div className="mt-1 font-display text-xl font-semibold tabular-nums text-slate-900 dark:text-white truncate">{value}</div>
      {sub && <div className="mt-0.5 text-xs text-slate-500 truncate">{sub}</div>}
    </div>
  )
}

export function StatGrid({ children, cols = 4 }) {
  const c = { 2: 'sm:grid-cols-2', 3: 'sm:grid-cols-3', 4: 'sm:grid-cols-2 lg:grid-cols-4' }[cols]
  return <div className={`grid grid-cols-2 ${c} gap-3 mb-8`}>{children}</div>
}

/** Segmented tabs. `tabs`: [{ id, label, icon? }]. */
export function Tabs({ tabs, value, onChange, className = '' }) {
  return (
    <div role="tablist" className={`inline-flex max-w-full overflow-x-auto p-1 rounded-xl bg-slate-100 dark:bg-slate-900 border border-slate-200 dark:border-slate-800 ${className}`}>
      {tabs.map(({ id, label, icon: Icon }) => (
        <button key={id} role="tab" type="button" aria-selected={value === id} onClick={() => onChange(id)}
          className={`shrink-0 inline-flex items-center gap-1.5 h-9 px-4 rounded-lg text-sm font-medium transition ${
            value === id ? 'bg-white dark:bg-slate-800 text-slate-900 dark:text-white shadow-sm' : 'text-slate-500 hover:text-slate-900 dark:hover:text-white'}`}>
          {Icon && <Icon className="w-4 h-4" />}{label}
        </button>
      ))}
    </div>
  )
}

const BADGE = {
  neutral: 'bg-slate-100 dark:bg-slate-800 text-slate-600 dark:text-slate-300',
  violet: 'bg-blue-500/10 text-blue-700 dark:text-blue-300',
  green: 'bg-emerald-500/15 text-emerald-700 dark:text-emerald-400',
  red: 'bg-rose-500/15 text-rose-700 dark:text-rose-400',
  amber: 'bg-amber-500/15 text-amber-700 dark:text-amber-400',
}

export function Badge({ tone = 'neutral', children, className = '' }) {
  return <span className={`inline-flex items-center gap-1 h-6 px-2.5 rounded-full text-xs font-medium ${BADGE[tone]} ${className}`}>{children}</span>
}

/** Small live/offline indicator. */
export function LiveBadge({ online, label, offlineLabel = 'Node offline' }) {
  return (
    <Badge tone={online ? 'green' : online === false ? 'red' : 'neutral'}>
      <span className={`w-1.5 h-1.5 rounded-full ${online ? 'bg-emerald-500 animate-pulse-dot' : online === false ? 'bg-rose-500' : 'bg-slate-400'}`} />
      {online ? label : online === false ? offlineLabel : 'Connecting…'}
    </Badge>
  )
}

export function EmptyState({ icon: Icon, title, children }) {
  return (
    <div className="py-10 text-center">
      {Icon && <Icon className="w-8 h-8 mx-auto text-slate-300 dark:text-slate-600" />}
      {title && <p className="mt-3 font-medium text-slate-700 dark:text-slate-300">{title}</p>}
      {children && <div className="mt-1 text-sm text-slate-500">{children}</div>}
    </div>
  )
}

/** Key/value rows. `rows`: [[label, value, mono?]]. */
export function KeyValue({ rows }) {
  return (
    <dl className="divide-y divide-slate-100 dark:divide-slate-800">
      {rows.map(([k, v, mono]) => (
        <div key={k} className="flex items-center justify-between gap-4 py-2.5 text-sm">
          <dt className="text-slate-500 shrink-0">{k}</dt>
          <dd className={`text-right text-slate-900 dark:text-white truncate ${mono ? 'font-mono text-xs' : 'font-medium'}`}>{v}</dd>
        </div>
      ))}
    </dl>
  )
}

/** Centered dialog. Closes on backdrop click and Escape. */
export function Modal({ title, onClose, children, footer }) {
  return (
    <div className="fixed inset-0 z-[70] bg-slate-950/60 flex items-center justify-center p-4" onClick={onClose}
      onKeyDown={(e) => { if (e.key === 'Escape') onClose() }}>
      <div role="dialog" aria-modal="true" aria-label={title} className="w-full max-w-md rounded-2xl glass-strong shadow-2xl animate-fade-in" onClick={(e) => e.stopPropagation()}>
        <div className="flex items-center justify-between px-5 py-4 border-b border-slate-100 dark:border-slate-800">
          <h3 className="font-semibold text-slate-900 dark:text-white">{title}</h3>
          <button type="button" onClick={onClose} aria-label="Close" className="p-1.5 rounded-lg text-slate-400 hover:text-slate-900 dark:hover:text-white"><X className="w-4 h-4" /></button>
        </div>
        <div className="p-5 space-y-4">{children}</div>
        {footer && <div className="flex justify-end gap-2 px-5 py-4 border-t border-slate-100 dark:border-slate-800">{footer}</div>}
      </div>
    </div>
  )
}
