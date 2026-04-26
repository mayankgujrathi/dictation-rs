export type TranscriptReformattingLevel = 'none' | 'minimal' | 'normal' | 'freeform'

export type LoggingSettings = {
  app_log_max_lines: number
  trace_file_limit: number
  enable_debug_logs: boolean
}

export type TranscriptionSettings = {
  built_in_dictionary: string[]
  user_dictionary: string[]
  model_cache_ttl_secs: number
  transcript_reformatting_level: TranscriptReformattingLevel
  llm_api_key: string | null
  llm_base_url: string
  llm_model_name: string
  llm_custom_prompt: string
}

export type AppSettings = {
  start_on_login: boolean
  logging: LoggingSettings
  transcription: TranscriptionSettings
}
