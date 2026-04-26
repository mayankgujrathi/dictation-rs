type AboutSectionProps = {
  logsDir: string
  onOpenLogsDir: () => void
  onOpenExternalUrl: (url: string) => void
  opening?: boolean
}

const PROJECT_LINKS = [
  { label: 'GitHub repository', href: 'https://github.com/mayankgujrathi/vocoflow' },
  { label: 'Report an issue', href: 'https://github.com/mayankgujrathi/vocoflow/issues' },
  { label: 'Documentation', href: 'https://github.com/mayankgujrathi/vocoflow/tree/main/docs' },
  { label: 'License', href: 'https://github.com/mayankgujrathi/vocoflow/blob/main/LICENSE' },
  {
    label: 'Licensing and Acknowledgments',
    href: 'https://github.com/mayankgujrathi/vocoflow/blob/main/docs/LICENSES.md',
  },
] as const

export function AboutSection({ logsDir, onOpenLogsDir, onOpenExternalUrl, opening }: AboutSectionProps) {
  return (
    <section className="space-y-3">
      <h2 className="text-lg font-semibold text-slate-100">About</h2>

      <div className="rounded-xl border border-slate-700/60 bg-slate-900/50 p-4">
        <p className="text-sm text-slate-200">Project links</p>
        <ul className="mt-2 space-y-1 text-xs">
          {PROJECT_LINKS.map((link) => (
            <li key={link.href}>
              <a
                className="text-accent2 underline-offset-2 hover:underline"
                href={link.href}
                rel="noreferrer"
                onClick={(event) => {
                  event.preventDefault()
                  onOpenExternalUrl(link.href)
                }}
              >
                {link.label}
              </a>
            </li>
          ))}
        </ul>
      </div>

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
