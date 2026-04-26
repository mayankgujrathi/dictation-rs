import type { ReactNode } from 'react'

type SettingsShellProps = {
  sidebar: ReactNode
  children: ReactNode
}

export function SettingsShell({ sidebar, children }: SettingsShellProps) {
  return (
    <main className="h-screen overflow-hidden bg-bg text-slate-100">
      <div className="mx-auto grid h-screen max-w-7xl grid-cols-[90px_1fr] gap-4 p-4 md:grid-cols-[110px_1fr]">
        <aside className="sticky top-4 h-[calc(100vh-2rem)] overflow-hidden">{sidebar}</aside>
        <section className="h-[calc(100vh-2rem)] overflow-y-auto rounded-2xl border border-slate-700/60 bg-panel/85 p-5 shadow-glow backdrop-blur">
          {children}
        </section>
      </div>
    </main>
  )
}
