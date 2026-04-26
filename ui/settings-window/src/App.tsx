import { useEffect, useMemo, useState } from 'react'
import { FaCircleInfo, FaGear, FaMicrophoneLines, FaSliders } from 'react-icons/fa6'

import {
  getAboutLogsDir,
  getAllSettings,
  openAboutExternalUrl,
  openAboutLogsDir,
  resetDefaults,
  updateLogging,
  updateStartOnLogin,
  updateTranscription,
} from './ipc'
import { SettingsShell } from './components/layout/SettingsShell'
import { SettingsSidebar, type SettingsTab } from './components/navigation/SettingsSidebar'
import { AboutSection } from './components/sections/AboutSection'
import { GeneralSection } from './components/sections/GeneralSection'
import { LoggingSection } from './components/sections/LoggingSection'
import { TranscriptionSection } from './components/sections/TranscriptionSection'
import type { AppSettings } from './types/settings'

const EMPTY_SETTINGS: AppSettings = {
  start_on_login: false,
  logging: { app_log_max_lines: 1000, trace_file_limit: 100, enable_debug_logs: false },
  transcription: {
    built_in_dictionary: [],
    user_dictionary: [],
    model_cache_ttl_secs: 600,
    transcript_reformatting_level: 'none',
    llm_api_key: null,
    llm_base_url: '',
    llm_model_name: '',
    llm_custom_prompt: '',
  },
}

function App() {
  const [activeTab, setActiveTab] = useState<SettingsTab>('general')
  const [settings, setSettings] = useState<AppSettings>(EMPTY_SETTINGS)
  const [logsDir, setLogsDir] = useState('')
  const [status, setStatus] = useState('Loading settings...')
  const [savingKey, setSavingKey] = useState<string>('')

  const sidebarItems = useMemo(
    () => [
      { id: 'general' as const, label: 'General', icon: FaSliders },
      { id: 'logging' as const, label: 'Logging', icon: FaGear },
      { id: 'transcription' as const, label: 'Speech', icon: FaMicrophoneLines },
      { id: 'about' as const, label: 'About', icon: FaCircleInfo },
    ],
    [],
  )

  useEffect(() => {
    const load = async () => {
      try {
        const [loadedSettings, loadedLogsDir] = await Promise.all([getAllSettings(), getAboutLogsDir()])
        setSettings(loadedSettings)
        setLogsDir(loadedLogsDir)
        setStatus('Settings loaded.')
      } catch (error) {
        setStatus(`Failed to load settings: ${String(error)}`)
      }
    }
    void load()
  }, [])

  useEffect(() => {
    const timer = setTimeout(async () => {
      setSavingKey('general')
      try {
        await updateStartOnLogin(settings.start_on_login)
        setStatus('General settings auto-saved.')
      } catch (error) {
        setStatus(`Save failed: ${String(error)}`)
      } finally {
        setSavingKey('')
      }
    }, 250)
    return () => clearTimeout(timer)
  }, [settings.start_on_login])

  useEffect(() => {
    const timer = setTimeout(async () => {
      setSavingKey('logging')
      try {
        await updateLogging(settings.logging)
        setStatus('Logging settings auto-saved.')
      } catch (error) {
        setStatus(`Save failed: ${String(error)}`)
      } finally {
        setSavingKey('')
      }
    }, 300)
    return () => clearTimeout(timer)
  }, [settings.logging])

  useEffect(() => {
    const timer = setTimeout(async () => {
      setSavingKey('transcription')
      try {
        await updateTranscription(settings.transcription)
        setStatus('Transcription settings auto-saved.')
      } catch (error) {
        setStatus(`Save failed: ${String(error)}`)
      } finally {
        setSavingKey('')
      }
    }, 500)
    return () => clearTimeout(timer)
  }, [settings.transcription])

  const openLogs = async () => {
    setSavingKey('about')
    try {
      const dir = await openAboutLogsDir()
      setLogsDir(dir)
      setStatus('Opened logs directory.')
    } catch (error) {
      setStatus(`Open logs failed: ${String(error)}`)
    } finally {
      setSavingKey('')
    }
  }

  const openExternalLink = async (url: string) => {
    try {
      await openAboutExternalUrl(url)
      setStatus('Opened link in default browser.')
    } catch (error) {
      setStatus(`Open link failed: ${String(error)}`)
    }
  }

  const runReset = async (scope: 'general' | 'logging' | 'transcription' | 'all') => {
    setSavingKey(scope)
    try {
      const next = await resetDefaults(scope)
      setSettings(next)
      setStatus(`Reset ${scope} defaults.`)
    } catch (error) {
      setStatus(`Reset failed: ${String(error)}`)
    } finally {
      setSavingKey('')
    }
  }

  return (
    <SettingsShell sidebar={<SettingsSidebar items={sidebarItems} activeTab={activeTab} onSelect={setActiveTab} />}>
      <header className="mb-5 rounded-xl border border-slate-700/50 bg-material-gradient p-4">
        <div className="flex items-center justify-between gap-3">
          <div>
            <h1 className="text-xl font-semibold">Vocoflow Settings</h1>
            <p className="mt-1 text-xs text-slate-200/90"></p>
          </div>
          <button type="button" onClick={() => void runReset('all')} className="rounded-lg border border-slate-500/70 bg-slate-900/40 px-3 py-1.5 text-xs text-slate-100">
            Reset all defaults
          </button>
        </div>
      </header>

      <div className="space-y-5">
        {activeTab === 'general' && (
          <GeneralSection
            startOnLogin={settings.start_on_login}
            onChange={(next) => setSettings((prev) => ({ ...prev, start_on_login: next }))}
            onReset={() => void runReset('general')}
            saving={savingKey === 'general'}
          />
        )}

        {activeTab === 'logging' && (
          <LoggingSection
            value={settings.logging}
            onChange={(next) => setSettings((prev) => ({ ...prev, logging: next }))}
            onReset={() => void runReset('logging')}
            saving={savingKey === 'logging'}
          />
        )}

        {activeTab === 'transcription' && (
          <TranscriptionSection
            value={settings.transcription}
            onChange={(next) => setSettings((prev) => ({ ...prev, transcription: next }))}
            onReset={() => void runReset('transcription')}
            saving={savingKey === 'transcription'}
          />
        )}

        {activeTab === 'about' && (
          <AboutSection
            logsDir={logsDir}
            onOpenLogsDir={openLogs}
            onOpenExternalUrl={(url) => void openExternalLink(url)}
            opening={savingKey === 'about'}
          />
        )}
      </div>

      <footer className="mt-6 rounded-lg border border-slate-700/60 bg-slate-900/40 px-3 py-2 text-xs text-slate-300">{status}</footer>
    </SettingsShell>
  )
}

export default App
