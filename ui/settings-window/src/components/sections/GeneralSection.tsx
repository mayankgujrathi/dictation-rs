import { useEffect, useMemo, useState } from 'react'

import { FormField } from '../fields/FormField'
import { ToggleField } from '../fields/ToggleField'

type GeneralSectionProps = {
  startOnLogin: boolean
  hotkeyBinding: string
  onChange: (value: boolean) => void
  onHotkeyBindingChange: (value: string) => void
  onReset: () => void
  saving?: boolean
}

export function GeneralSection({
  startOnLogin,
  hotkeyBinding,
  onChange,
  onHotkeyBindingChange,
  onReset,
  saving,
}: GeneralSectionProps) {
  const [draftBinding, setDraftBinding] = useState(hotkeyBinding)
  const [capturing, setCapturing] = useState(false)
  const [capturedSteps, setCapturedSteps] = useState<string[]>([])

  useEffect(() => {
    setDraftBinding(hotkeyBinding)
  }, [hotkeyBinding])

  const previewBinding = useMemo(() => {
    if (capturedSteps.length > 0) {
      return capturedSteps.join(', ')
    }
    return draftBinding
  }, [capturedSteps, draftBinding])

  useEffect(() => {
    if (!capturing) {
      return
    }

    const modifiers = { ctrl: false, shift: false, alt: false, meta: false }
    const mapKey = (event: KeyboardEvent): string | null => {
      const key = event.key
      if (key.length === 1 && /[a-z0-9`]/i.test(key)) {
        return key === '`' ? '`' : key.toUpperCase()
      }
      if (/^F([1-9]|1\d|2[0-4])$/i.test(key)) {
        return key.toUpperCase()
      }
      if (key === ' ') return 'Space'
      if (key === 'Tab') return 'Tab'
      if (key === 'Enter') return 'Enter'
      if (key === 'Escape') return 'Escape'
      return null
    }

    const withModifiers = (trigger: string) => {
      const parts: string[] = []
      if (modifiers.ctrl) parts.push('Ctrl')
      if (modifiers.shift) parts.push('Shift')
      if (modifiers.alt) parts.push('Alt')
      if (modifiers.meta) parts.push('Meta')
      parts.push(trigger)
      return parts.join('+')
    }

    const onKeyDown = (event: KeyboardEvent) => {
      if (event.repeat) return
      if (event.key === 'Escape') {
        event.preventDefault()
        setCapturing(false)
        setCapturedSteps([])
        return
      }
      if (event.key === 'Enter') {
        event.preventDefault()
        setDraftBinding((capturedSteps.length > 0 ? capturedSteps.join(', ') : draftBinding).trim())
        setCapturing(false)
        return
      }

      if (event.key === 'Control') {
        modifiers.ctrl = true
        return
      }
      if (event.key === 'Shift') {
        modifiers.shift = true
        return
      }
      if (event.key === 'Alt') {
        modifiers.alt = true
        return
      }
      if (event.key === 'Meta') {
        modifiers.meta = true
        return
      }

      const mapped = mapKey(event)
      if (!mapped) return
      event.preventDefault()
      setCapturedSteps((prev) => [...prev, withModifiers(mapped)])
    }

    const onKeyUp = (event: KeyboardEvent) => {
      if (event.key === 'Control') modifiers.ctrl = false
      if (event.key === 'Shift') modifiers.shift = false
      if (event.key === 'Alt') modifiers.alt = false
      if (event.key === 'Meta') modifiers.meta = false
    }

    const onMouseDown = (event: MouseEvent) => {
      const mapped = event.button === 0 ? 'MouseLeft' : event.button === 1 ? 'MouseMiddle' : event.button === 2 ? 'MouseRight' : event.button === 3 ? 'Mouse4' : event.button === 4 ? 'Mouse5' : null
      if (!mapped) return
      event.preventDefault()
      setCapturedSteps((prev) => [...prev, withModifiers(mapped)])
    }

    window.addEventListener('keydown', onKeyDown)
    window.addEventListener('keyup', onKeyUp)
    window.addEventListener('mousedown', onMouseDown)
    return () => {
      window.removeEventListener('keydown', onKeyDown)
      window.removeEventListener('keyup', onKeyUp)
      window.removeEventListener('mousedown', onMouseDown)
    }
  }, [capturing, capturedSteps, draftBinding])

  const chips = (binding: string) =>
    binding
      .split(/(\+|,)/)
      .map((part) => part.trim())
      .filter(Boolean)

  return (
    <section className="space-y-3">
      <div className="flex items-center justify-between">
        <h2 className="text-lg font-semibold text-slate-100">General</h2>
        <button type="button" onClick={onReset} className="rounded-lg border border-slate-600 px-2 py-1 text-xs text-slate-200">Reset defaults</button>
      </div>
      <FormField label="Start on login" description="Automatically starts Vocoflow when you sign in.">
        <div className="flex items-center justify-between gap-3">
          <ToggleField checked={startOnLogin} onChange={onChange} />
          <span className="text-xs text-slate-400">{saving ? 'Saving…' : 'Auto-saved'}</span>
        </div>
      </FormField>
      <FormField
        label="Global hotkey"
        description="Click Change, then press keys/mouse like in games. Enter to finish, Esc to cancel."
      >
        <div className="space-y-2">
          <div className="text-xs text-slate-400">Current: {hotkeyBinding}</div>
          <div className="flex flex-wrap gap-1">
            {chips(previewBinding).map((chip, idx) => (
              <span key={`${chip}-${idx}`} className="rounded-md border border-cyan-300/30 bg-cyan-500/10 px-2 py-1 text-xs text-cyan-100">
                {chip}
              </span>
            ))}
          </div>
          <div className="flex gap-2">
            <button
              type="button"
              onClick={() => {
                setCapturedSteps([])
                setCapturing(true)
              }}
              className="rounded-md border border-slate-600 px-2 py-1 text-xs text-slate-200"
            >
              Change
            </button>
            <button
              type="button"
              onClick={() => onHotkeyBindingChange(draftBinding.trim())}
              className="rounded-md border border-cyan-300/40 bg-cyan-500/10 px-2 py-1 text-xs text-cyan-100"
            >
              Apply
            </button>
          </div>
          {capturing && <div className="text-xs text-amber-200">Listening… press keys/mouse, Enter to accept, Esc to cancel.</div>}
        </div>
      </FormField>
    </section>
  )
}
