type ToggleFieldProps = {
  checked: boolean
  onChange: (next: boolean) => void
}

export function ToggleField({ checked, onChange }: ToggleFieldProps) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      onClick={() => onChange(!checked)}
      className={`relative inline-flex h-6 w-11 items-center rounded-full transition ${checked ? 'bg-accent' : 'bg-slate-700'}`}
    >
      <span
        className={`inline-block h-5 w-5 transform rounded-full bg-white transition ${checked ? 'translate-x-5' : 'translate-x-1'}`}
      />
    </button>
  )
}
