# LLM Setup and Usage

This guide explains how to configure and use Vocoflow's optional LLM transcript reformatting.

Repository: https://github.com/mayankgujrathi/vocoflow

## What This Feature Does

Vocoflow always produces a local transcript first. If you choose a reformatting level other than `none`, Vocoflow can send that transcript to an OpenAI-compatible endpoint for cleanup/rewrite.

- `none`: no LLM post-processing
- `minimal`: small punctuation/casing fixes
- `normal`: readability and grammar improvements
- `freeform`: strongest rewrite with app-context-aware polish

## Required Settings

Open **Settings → Speech** and configure:

1. **Reformatting level** (`minimal`, `normal`, or `freeform`)
2. **LLM base URL** (OpenAI-compatible API base)
3. **LLM model name**
4. **LLM API key** (if your provider requires one)
5. Optional: **LLM custom prompt**

## Quick Start Example

Typical OpenAI-compatible shape:

- Base URL: provider endpoint root (for example, `https://api.openai.com/v1`)
- Model name: provider model id (for example, `gpt-4o-mini`)
- API key: token from your provider

Then speak normally and test with `minimal` first.

## Reliability Behavior

- If LLM post-processing succeeds, Vocoflow uses the LLM output.
- If LLM post-processing fails, Vocoflow falls back to the local transcript so clipboard/paste flow still continues.
- An error signal is shown, and the Settings window can display a flash warning message.

## Troubleshooting

If LLM fails repeatedly, check:

- Base URL is reachable
- Model name is correct
- API key is valid (if required)
- Network/proxy/firewall allows outbound calls
- Provider supports OpenAI-compatible request format

For logs and trace details, see [Settings and Logging](SETTINGS_AND_LOGGING.md).

## Related Docs

- [Settings and Logging](SETTINGS_AND_LOGGING.md)
- [Architecture](ARCHITECTURE.md)
- [Build and Release](BUILD_AND_RELEASE.md)
