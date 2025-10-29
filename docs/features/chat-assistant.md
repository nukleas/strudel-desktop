# API Key Setup for Strudel Desktop Chat

The chat feature requires an API key from one of the supported providers. Here's how to set it up:

## Supported Providers

- **Anthropic Claude** (recommended): `claude-sonnet-4-5-20250929`
- **OpenAI**: `gpt-4o-mini`, `gpt-4o`, `o3`, `o4-mini`
- **Google Gemini**: `gemini-2.5-flash`, `gemini-2.5-pro`

## Setting Up Your API Key

### Method 1: Environment Variable (Recommended)

Set the appropriate environment variable before starting the app:

**For Anthropic Claude:**
```bash
export ANTHROPIC_API_KEY="your-api-key-here"
```

**For OpenAI:**
```bash
export OPENAI_API_KEY="your-api-key-here"
```

**For Google Gemini:**
```bash
export GEMINI_API_KEY="your-api-key-here"
```

### Method 2: Through the App UI

1. Open Strudel Desktop
2. Go to the Chat tab
3. Click the settings/gear icon
4. Enter your API key in the settings dialog
5. The key will be securely stored in your OS keychain

## Getting API Keys

### Anthropic Claude
1. Go to https://console.anthropic.com/
2. Sign up or log in
3. Navigate to API Keys
4. Create a new API key
5. Copy the key (starts with `sk-ant-`)

### OpenAI
1. Go to https://platform.openai.com/api-keys
2. Sign up or log in
3. Create a new API key
4. Copy the key (starts with `sk-`)

### Google Gemini
1. Go to https://aistudio.google.com/app/apikey
2. Sign up or log in
3. Create a new API key
4. Copy the key

## Security Notes

- API keys are stored securely in your OS keychain (macOS Keychain, Windows Credential Manager, Linux Secret Service)
- Never share your API keys publicly
- Keys are not stored in plain text in the application

## Troubleshooting

If you're getting "ApiKeyEnvNotFound" errors:

1. Make sure the environment variable is set correctly
2. Restart the application after setting the environment variable
3. Check that the variable name matches your chosen provider:
   - `ANTHROPIC_API_KEY` for Claude
   - `OPENAI_API_KEY` for OpenAI
   - `GEMINI_API_KEY` for Gemini

## Usage

Once your API key is set up, you can:
- Ask questions about Strudel functions
- Get help with pattern creation
- Request code examples
- Debug your patterns

The chat assistant has access to the latest Strudel documentation and can help you create musical patterns!
