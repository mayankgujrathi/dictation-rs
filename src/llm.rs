use async_openai::Client;
use async_openai::config::OpenAIConfig;
use async_openai::types::{
  ChatCompletionRequestMessage, ChatCompletionRequestSystemMessageArgs,
  ChatCompletionRequestUserMessageArgs, CreateChatCompletionRequestArgs,
};
use tracing::{debug, error};

#[derive(Debug, Clone)]
pub struct LlmPostProcessorConfig {
  pub api_key: Option<String>,
  pub base_url: String,
  pub model_name: String,
  pub custom_prompt: String,
  pub system_prompt: String,
  pub reformatting_level: String,
}

#[derive(Debug, Clone, Default)]
pub struct LlmAppContext {
  pub window_title: String,
  pub application_name: Option<String>,
  pub application_description: Option<String>,
}

pub fn process_transcript_with_llm(
  cfg: &LlmPostProcessorConfig,
  transcript_text: &str,
  app_context: &LlmAppContext,
) -> Result<String, ()> {
  let runtime = match tokio::runtime::Builder::new_current_thread()
    .enable_all()
    .build()
  {
    Ok(rt) => rt,
    Err(e) => {
      error!(error = %e, "failed to create llm runtime");
      return Err(());
    }
  };

  runtime.block_on(process_transcript_with_llm_async(
    cfg,
    transcript_text,
    app_context,
  ))
}

async fn process_transcript_with_llm_async(
  cfg: &LlmPostProcessorConfig,
  transcript_text: &str,
  app_context: &LlmAppContext,
) -> Result<String, ()> {
  let mut openai_cfg = OpenAIConfig::new().with_api_base(cfg.base_url.clone());
  if let Some(api_key) = cfg.api_key.as_ref().filter(|v| !v.trim().is_empty()) {
    openai_cfg = openai_cfg.with_api_key(api_key.clone());
  }

  let client = Client::with_config(openai_cfg);
  let user_prompt = format!(
    "{}\n\nReturn only the final transcript text. No explanation, no labels, no markdown.\n\nReformatting level: {}\n\nApp context:\n- Window title: {}\n- Application name: {}\n- Application description: {}\n\nTranscript:\n{}",
    cfg.custom_prompt,
    cfg.reformatting_level,
    app_context.window_title,
    app_context.application_name.as_deref().unwrap_or(""),
    app_context.application_description.as_deref().unwrap_or(""),
    transcript_text
  );

  let mut messages: Vec<ChatCompletionRequestMessage> = Vec::new();

  if !cfg.system_prompt.trim().is_empty() {
    let system_message = ChatCompletionRequestSystemMessageArgs::default()
      .content(cfg.system_prompt.clone())
      .build();
    let Ok(system_message) = system_message else {
      error!("failed to build llm system message");
      return Err(());
    };
    messages.push(system_message.into());
  }

  let user_message = ChatCompletionRequestUserMessageArgs::default()
    .content(user_prompt)
    .build();
  let Ok(user_message) = user_message else {
    error!("failed to build llm user message");
    return Err(());
  };
  messages.push(user_message.into());

  let request = CreateChatCompletionRequestArgs::default()
    .model(cfg.model_name.clone())
    .messages(messages)
    .build();
  let Ok(request) = request else {
    error!("failed to build llm chat request");
    return Err(());
  };

  let response = client.chat().create(request).await;
  let Ok(response) = response else {
    error!(error = ?response.err(), "llm chat request failed");
    return Err(());
  };

  let text = response
    .choices
    .first()
    .and_then(|choice| choice.message.content.as_ref())
    .map(|s| s.trim().to_owned())
    .filter(|s| !s.is_empty());

  let Some(text) = text else {
    debug!("llm response had no usable content");
    return Err(());
  };

  let reformatted = text.trim().to_owned();
  if reformatted.is_empty() {
    error!("llm response was empty after trimming");
    return Err(());
  }

  debug!(reformatted_transcript = %reformatted, "llm plain-text output accepted");

  tracing::info!(
    model_generated_transcript = %reformatted,
    "llm model-generated transcript ready"
  );

  Ok(reformatted)
}

#[cfg(test)]
mod tests {
  #[test]
  fn test_plain_text_output_is_trimmed() {
    let input = "  Hello world.  ";
    let output = input.trim().to_owned();
    assert_eq!(output, "Hello world.");
  }
}
