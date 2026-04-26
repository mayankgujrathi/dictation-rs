import { FormField } from '../fields/FormField'
import { ToggleField } from '../fields/ToggleField'

type GeneralSectionProps = {
  startOnLogin: boolean
  onChange: (value: boolean) => void
  onReset: () => void
  saving?: boolean
}

export function GeneralSection({ startOnLogin, onChange, onReset, saving }: GeneralSectionProps) {
  return (
    <section className="space-y-3">
      <div className="flex items-center justify-between">
        <h2 className="text-lg font-semibold text-slate-100">General</h2>
        <button type="button" onClick={onReset} className="rounded-lg border border-slate-600 px-2 py-1 text-xs text-slate-200">Reset defaults</button>
      </div>
      <FormField label="Start on login" description="Automatically starts Dictation when you sign in.">
        <div className="flex items-center justify-between gap-3">
          <ToggleField checked={startOnLogin} onChange={onChange} />
          <span className="text-xs text-slate-400">{saving ? 'Saving…' : 'Auto-saved'}</span>
        </div>
      </FormField>
    </section>
  )
}
