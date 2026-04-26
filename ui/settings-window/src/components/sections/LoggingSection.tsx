import type { LoggingSettings } from '../../types/settings'
import { ToggleField } from '../fields/ToggleField'
import { FormField } from '../fields/FormField'

type LoggingSectionProps = {
  value: LoggingSettings
  onChange: (next: LoggingSettings) => void
  onReset: () => void
  saving?: boolean
}

export function LoggingSection({ value, onChange, onReset, saving }: LoggingSectionProps) {
  return (
    <section className="space-y-3">
      <div className="flex items-center justify-between">
        <h2 className="text-lg font-semibold text-slate-100">Logging</h2>
        <button type="button" onClick={onReset} className="rounded-lg border border-slate-600 px-2 py-1 text-xs text-slate-200">Reset defaults</button>
      </div>
      <div className="grid gap-3 md:grid-cols-2">
        <FormField label="App log max lines" description="Maximum retained lines in application.log.">
          <div className="space-y-2">
            <input
              type="range"
              min={100}
              max={10000}
              step={100}
              value={value.app_log_max_lines}
              onChange={(e) => onChange({ ...value, app_log_max_lines: Number(e.target.value) || 100 })}
              className="w-full accent-accent"
            />
            <div className="flex justify-between text-xs text-slate-400">
              <span>100</span>
              <span className="font-medium text-slate-200">{value.app_log_max_lines}</span>
              <span>10000</span>
            </div>
          </div>
        </FormField>
        <FormField label="Trace file limit" description="How many trace files to keep in logs/traces.">
          <div className="space-y-2">
            <input
              type="range"
              min={10}
              max={500}
              step={5}
              value={value.trace_file_limit}
              onChange={(e) => onChange({ ...value, trace_file_limit: Number(e.target.value) || 10 })}
              className="w-full accent-accent"
            />
            <div className="flex justify-between text-xs text-slate-400">
              <span>10</span>
              <span className="font-medium text-slate-200">{value.trace_file_limit}</span>
              <span>500</span>
            </div>
          </div>
        </FormField>
      </div>
      <FormField label="Enable debug logs" description="Writes additional diagnostic details to logs.">
        <div className="flex items-center justify-between">
          <ToggleField checked={value.enable_debug_logs} onChange={(checked) => onChange({ ...value, enable_debug_logs: checked })} />
          <span className="text-xs text-slate-400">{saving ? 'Saving…' : 'Auto-saved'}</span>
        </div>
      </FormField>
    </section>
  )
}
