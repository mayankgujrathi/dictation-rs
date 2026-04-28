import { useState } from 'react'
import type { TranscriptionSettings } from '../../types/settings'
import { FaAlignLeft, FaBolt, FaCircleInfo, FaListCheck, FaPencil, FaWandMagicSparkles } from 'react-icons/fa6'
import { ChipInputField } from '../fields/ChipInputField'
import { FormField } from '../fields/FormField'

type TranscriptionSectionProps = {
  value: TranscriptionSettings
  onChange: (next: TranscriptionSettings) => void
  onReset: () => void
  onOpenLlmGuide?: () => void
  saving?: boolean
}

export function TranscriptionSection({ value, onChange, onReset, onOpenLlmGuide, saving }: TranscriptionSectionProps) {
  const [edit, setEdit] = useState({ userDict: false, baseUrl: false, modelName: false, apiKey: false, prompt: false })
  const pencilClass = (active: boolean) =>
    `rounded-md border px-2 py-1 text-xs transition ${
      active
        ? 'border-cyan-300/60 bg-cyan-500/20 text-cyan-100'
        : 'border-slate-600 bg-slate-900/40 text-slate-300 hover:border-slate-500'
    }`
  const levels = [
    { key: 'none', label: 'None', icon: FaAlignLeft },
    { key: 'minimal', label: 'Minimal', icon: FaListCheck },
    { key: 'normal', label: 'Normal', icon: FaBolt },
    { key: 'freeform', label: 'Freeform', icon: FaWandMagicSparkles },
  ] as const
  const levelHelp: Record<string, string> = {
    none: 'Return transcript unchanged; typically bypasses model calls.',
    minimal: 'Small edits only: punctuation/casing/minor clarity.',
    normal: 'Improve readability and grammar while preserving intent.',
    freeform: 'Most polished context-adapted output; can alter wording.',
  }

  return (
    <section className="space-y-3">
      <div className="flex items-center justify-between">
        <h2 className="text-lg font-semibold text-slate-100">Transcription</h2>
        <div className="flex items-center gap-2">
          <button type="button" onClick={onOpenLlmGuide} className="rounded-lg border border-cyan-500/50 px-2 py-1 text-xs text-cyan-200 hover:bg-cyan-500/10">
            Learn more (LLM)
          </button>
          <button type="button" onClick={onReset} className="rounded-lg border border-slate-600 px-2 py-1 text-xs text-slate-200">Reset defaults</button>
        </div>
      </div>
      <div className="grid gap-3 md:grid-cols-2">
        <FormField label="Model cache TTL (secs)" description="How long model metadata is cached.">
          <div className="space-y-2">
            <input type="range" min={0} max={7200} step={30} value={value.model_cache_ttl_secs} onChange={(e) => onChange({ ...value, model_cache_ttl_secs: Number(e.target.value) || 0 })} className="w-full accent-accent" />
            <div className="flex justify-between text-xs text-slate-400"><span>0</span><span className="font-medium text-slate-200">{value.model_cache_ttl_secs}</span><span>7200</span></div>
          </div>
        </FormField>
        <FormField label="Reformatting level" description="Controls transcript post-formatting behavior.">
          <div className="grid grid-cols-2 gap-2">
            {levels.map((lvl) => {
              const Icon = lvl.icon
              const selected = value.transcript_reformatting_level === lvl.key
              return (
                <label key={lvl.key} title={levelHelp[lvl.key]} className={`flex cursor-pointer items-center gap-2 rounded-lg border px-2 py-2 text-xs ${selected ? 'border-cyan-300/40 bg-cyan-500/10 text-cyan-100' : 'border-slate-600 bg-slate-950/50 text-slate-300'}`}>
                  <input type="radio" className="hidden" checked={selected} onChange={() => onChange({ ...value, transcript_reformatting_level: lvl.key })} />
                  <Icon /> {lvl.label} <FaCircleInfo className="opacity-70" />
                </label>
              )
            })}
          </div>
        </FormField>
      </div>
      <FormField label="User dictionary" description="Your custom vocabulary list.">
        <div className="space-y-2">
          <div className="flex justify-end"><button type="button" onClick={() => setEdit((s) => ({ ...s, userDict: !s.userDict }))} className={pencilClass(edit.userDict)}><FaPencil /></button></div>
          <ChipInputField editable={edit.userDict} chips={value.user_dictionary} onChange={(chips) => onChange({ ...value, user_dictionary: chips })} placeholder="Add custom word and press Enter" />
        </div>
      </FormField>
      <div className="grid gap-3 md:grid-cols-2">
        <FormField label="LLM base URL" description="Endpoint for transcript reformatting model.">
          <div className="space-y-2"><div className="flex justify-end"><button type="button" onClick={() => setEdit((s) => ({ ...s, baseUrl: !s.baseUrl }))} className={pencilClass(edit.baseUrl)}><FaPencil /></button></div><input disabled={!edit.baseUrl} value={value.llm_base_url} onChange={(e) => onChange({ ...value, llm_base_url: e.target.value })} className="w-full rounded-lg border border-slate-600 bg-slate-950/70 px-3 py-2" /></div>
        </FormField>
        <FormField label="LLM model name" description="Model identifier used for reformatting.">
          <div className="space-y-2"><div className="flex justify-end"><button type="button" onClick={() => setEdit((s) => ({ ...s, modelName: !s.modelName }))} className={pencilClass(edit.modelName)}><FaPencil /></button></div><input disabled={!edit.modelName} value={value.llm_model_name} onChange={(e) => onChange({ ...value, llm_model_name: e.target.value })} className="w-full rounded-lg border border-slate-600 bg-slate-950/70 px-3 py-2" /></div>
        </FormField>
      </div>
      <FormField label="LLM API key" description="Optional API key if required by provider.">
        <div className="space-y-2"><div className="flex justify-end"><button type="button" onClick={() => setEdit((s) => ({ ...s, apiKey: !s.apiKey }))} className={pencilClass(edit.apiKey)}><FaPencil /></button></div><input disabled={!edit.apiKey} value={value.llm_api_key ?? ''} onChange={(e) => onChange({ ...value, llm_api_key: e.target.value || null })} className="w-full rounded-lg border border-slate-600 bg-slate-950/70 px-3 py-2" /></div>
      </FormField>
      <FormField label="LLM custom prompt" description="Additional instructions for transcript post-processing.">
        <div className="space-y-2"><div className="flex justify-end"><button type="button" onClick={() => setEdit((s) => ({ ...s, prompt: !s.prompt }))} className={pencilClass(edit.prompt)}><FaPencil /></button></div><textarea disabled={!edit.prompt} value={value.llm_custom_prompt} onChange={(e) => onChange({ ...value, llm_custom_prompt: e.target.value })} className="min-h-24 w-full rounded-lg border border-slate-600 bg-slate-950/70 px-3 py-2" /></div>
      </FormField>
      <div className="flex justify-end"><span className="text-xs text-slate-400">{saving ? 'Saving…' : 'Auto-saved'}</span></div>
    </section>
  )
}
