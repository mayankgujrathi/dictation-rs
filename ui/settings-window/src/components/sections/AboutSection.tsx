type AboutSectionProps = {
  logsDir: string
  onOpenLogsDir: () => void
  opening?: boolean
}

export function AboutSection({ logsDir, onOpenLogsDir, opening }: AboutSectionProps) {
  return (
    <section className="space-y-3">
      <h2 className="text-lg font-semibold text-slate-100">About</h2>
      <div className="rounded-xl border border-slate-700/60 bg-slate-900/50 p-4">
        <p className="text-sm text-slate-200">Logs directory</p>
        <p className="mt-1 break-all text-xs text-slate-400">{logsDir || 'Loading...'}</p>
        <div className="mt-3">
          <button type="button" onClick={onOpenLogsDir} disabled={opening} className="rounded-lg bg-accent2 px-3 py-1 text-xs text-slate-950">
            {opening ? 'Opening…' : 'Open in file explorer'}
          </button>
        </div>
      </div>
    </section>
  )
}
