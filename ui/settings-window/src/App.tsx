import { useCallback, useMemo, useState } from 'react'

type IpcMethod = 'GET' | 'POST' | 'PUT' | 'DELETE' | 'PATCH'

type IpcRequest<TPayload> = {
  request_id?: string
  method: IpcMethod
  endpoint: string
  payload: TPayload
}

type IpcError = {
  code?: string
  message?: string
  field?: string
  expected?: string
  received?: string
}

type IpcResponse<TPayload> = {
  request_id?: string
  ok: boolean
  kind: string
  payload: TPayload
  error?: IpcError
}

type SettingsResponse = {
  settings: {
    start_on_login: boolean
    logging: {
      app_log_max_lines: number
      trace_file_limit: number
      enable_debug_logs: boolean
    }
    transcription: {
      built_in_dictionary: string[]
      user_dictionary: string[]
      model_cache_ttl_secs: number
      transcript_reformatting_level: 'none' | 'minimal' | 'normal' | 'freeform'
      llm_api_key: string | null
      llm_base_url: string
      llm_model_name: string
      llm_custom_prompt: string
    }
  }
}

const makeRequestId = (): string => {
  if (typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function') {
    return crypto.randomUUID()
  }
  return String(Date.now())
}

const normalizeError = (errorLike: unknown): Required<Pick<IpcError, 'code' | 'message'>> & IpcError => {
  if (typeof errorLike === 'string') {
    return { code: 'UNKNOWN', message: errorLike }
  }
  if (!errorLike || typeof errorLike !== 'object') {
    return { code: 'UNKNOWN', message: 'Unknown IPC error' }
  }

  const candidate = errorLike as IpcError
  return {
    code: candidate.code || 'UNKNOWN',
    message: candidate.message || 'IPC request failed',
    field: candidate.field,
    expected: candidate.expected,
    received:
      typeof candidate.received === 'string'
        ? candidate.received
        : candidate.received != null
          ? JSON.stringify(candidate.received)
          : undefined,
  }
}

const formatErrorForUi = (error: IpcError): string => {
  const lines = [`code: ${error.code || 'UNKNOWN'}`, `message: ${error.message || 'IPC request failed'}`]
  if (error.field) lines.push(`field: ${error.field}`)
  if (error.expected) lines.push(`expected: ${error.expected}`)
  if (error.received) lines.push(`received: ${error.received}`)
  return lines.join('\n')
}

const sendIpc = async <TPayloadReq, TPayloadRes>(
  request: IpcRequest<TPayloadReq>,
): Promise<IpcResponse<TPayloadRes>> => {
  const request_id = request.request_id || makeRequestId()
  const body = JSON.stringify({ ...request, request_id })

  return await new Promise((resolve, reject) => {
    const xhr = new XMLHttpRequest()
    xhr.open('POST', '/ipc', true)
    xhr.setRequestHeader('content-type', 'application/json')
    xhr.timeout = 10000

    xhr.onload = () => {
      let parsed: IpcResponse<TPayloadRes> | null = null

      try {
        parsed = JSON.parse(xhr.responseText || '{}') as IpcResponse<TPayloadRes>
      } catch {
        reject(new Error(`IPC response was not valid JSON (status=${xhr.status}, request_id=${request_id})`))
        return
      }

      if (xhr.status >= 400 || !parsed?.ok) {
        const normalized = normalizeError(parsed?.error || parsed)
        reject(new Error(formatErrorForUi(normalized)))
        return
      }

      resolve(parsed)
    }

    xhr.onerror = () => reject(new Error(`IPC request failed (network error, request_id=${request_id})`))
    xhr.ontimeout = () => reject(new Error(`IPC request timed out for request_id=${request_id}`))
    xhr.send(body)
  })
}

const pretty = (value: unknown): string => (typeof value === 'string' ? value : JSON.stringify(value, null, 2))

function App() {
  const [response, setResponse] = useState<string>('No response yet.')
  const [concatX, setConcatX] = useState<string>('42')
  const [concatY, setConcatY] = useState<string>('hello')

  const concatXNumber = useMemo(() => Number(concatX), [concatX])

  const runRequest = useCallback(async (action: () => Promise<unknown>) => {
    try {
      const reply = await action()
      setResponse(pretty(reply))
    } catch (error) {
      setResponse(`Request failed:\n${String(error)}`)
    }
  }, [])

  return (
    <main className="page">
      <h1>Settings</h1>
      <p className="subtitle">
        Wry-compatible React + TypeScript settings shell using lightweight IPC over <code>/ipc</code>.
      </p>

      <section className="card" aria-label="Bridge demo actions">
        <div className="row">
          <button
            type="button"
            onClick={() =>
              runRequest(() =>
                sendIpc<Record<string, never>, SettingsResponse>({
                  method: 'GET',
                  endpoint: '/settings',
                  payload: {},
                }),
              )
            }
          >
            Load Settings
          </button>
          <button
            type="button"
            onClick={() =>
              runRequest(() =>
                sendIpc<{ source: string }, { source: string }>({
                  method: 'POST',
                  endpoint: '/settings/ping',
                  payload: { source: 'settings-ui' },
                }),
              )
            }
          >
            Send Ping
          </button>
        </div>

        <div className="row wrap">
          <label>
            x:
            <input
              value={concatX}
              onChange={(event) => setConcatX(event.target.value)}
              inputMode="numeric"
              aria-label="concat x"
            />
          </label>
          <label>
            y:
            <input value={concatY} onChange={(event) => setConcatY(event.target.value)} aria-label="concat y" />
          </label>
          <button
            type="button"
            onClick={() =>
              runRequest(() =>
                sendIpc<{ x: number; y: string }, { value?: string }>({
                  method: 'POST',
                  endpoint: '/settings/concat',
                  payload: {
                    x: Number.isFinite(concatXNumber) ? concatXNumber : 0,
                    y: concatY,
                  },
                }),
              )
            }
          >
            Send settings.concat
          </button>
        </div>

        <pre className="response" aria-live="polite">
          {response}
        </pre>
      </section>
    </main>
  )
}

export default App
