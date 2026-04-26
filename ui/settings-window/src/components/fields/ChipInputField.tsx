import { useState } from 'react'
import { FaXmark } from 'react-icons/fa6'

type ChipInputFieldProps = {
  chips: string[]
  onChange: (chips: string[]) => void
  editable?: boolean
  placeholder?: string
}

export function ChipInputField({ chips, onChange, editable = true, placeholder }: ChipInputFieldProps) {
  const [draft, setDraft] = useState('')

  const commitDraft = () => {
    const next = draft
      .split(',')
      .map((part) => part.trim())
      .filter(Boolean)
      .filter((item) => !chips.includes(item))

    if (!next.length) return
    onChange([...chips, ...next])
    setDraft('')
  }

  return (
    <div className="grid gap-2">
      <div className="flex flex-wrap gap-2">
        {chips.map((chip) => (
          <span key={chip} className="inline-flex items-center gap-1 rounded-full border border-cyan-300/30 bg-cyan-500/10 px-2 py-1 text-xs text-cyan-100">
            {chip}
            <button
              type="button"
              onClick={(event) => {
                event.stopPropagation()
                onChange(chips.filter((item) => item !== chip))
              }}
              className="rounded-full p-0.5 text-cyan-200 hover:bg-cyan-400/20"
              aria-label={`Remove ${chip}`}
            >
              <FaXmark />
            </button>
          </span>
        ))}
      </div>
      <input
        value={draft}
        onChange={(event) => setDraft(event.target.value)}
        onBlur={commitDraft}
        disabled={!editable}
        onKeyDown={(event) => {
          if (event.key === 'Enter' || event.key === ',') {
            event.preventDefault()
            commitDraft()
          }
        }}
        placeholder={placeholder || 'Type word and press Enter'}
        className="w-full rounded-lg border border-slate-600 bg-slate-950/70 px-3 py-2"
      />
    </div>
  )
}
