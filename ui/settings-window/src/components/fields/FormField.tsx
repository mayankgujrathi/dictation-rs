import type { ReactNode } from 'react'

type FormFieldProps = {
  label: string
  description: string
  children: ReactNode
}

export function FormField({ label, description, children }: FormFieldProps) {
  return (
    <div className="grid gap-2 rounded-xl border border-slate-700/60 bg-slate-900/50 p-3">
      <div>
        <div className="text-sm font-medium text-slate-100">{label}</div>
        <div className="text-xs text-slate-400">{description}</div>
      </div>
      {children}
    </div>
  )
}
