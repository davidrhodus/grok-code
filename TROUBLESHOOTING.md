# Troubleshooting Guide

## Common Issues

### Slow API Responses

**⚠️ Important**: xAI's Grok API can be extremely slow (3-5+ minutes per response). This is normal behavior.

- The program shows progress updates every 15 seconds: `🤔 Thinking... (15s) (30s)`
- Responses are automatically retried if they timeout (>5 minutes)
- For faster responses, use OpenAI instead: `grok-code --dev`

### xAI API Credit Issues

If you see a 403 error mentioning "credits", this is a common issue with xAI:

1. **Wait 5-15 minutes** - Credits often take time to activate after purchase
2. **Check your team billing page** - Visit `https://console.x.ai/team/[your-team-id]` to verify credits are available
3. **Regenerate your API key** - After credits show as available, generate a new API key
4. **Use OpenAI as alternative** - Run with `--dev` flag: `grok-code --dev`

### Configuration Check

Run the check command to verify your setup:
```bash
grok-code check
```

This will show:
- Which API provider you're using
- If your API key is properly configured
- Status of optional environment variables
- Any configuration issues

### API Key Issues

- If you see "API key is required", the tool will show detailed setup instructions
- Store keys securely: `grok-code key set xai YOUR_KEY`
- Check if key is stored: `grok-code key list`
- Ensure your API key is correctly set in the environment
- Check API rate limits if you get 429 errors

### Debugging API Issues

Enable debug mode to see detailed API communication:
```bash
export DEBUG_API=1
grok-code
```

This will show:
- Exact API requests being sent
- Response details including error messages
- Tool execution details

### Build Issues

- Requires Rust 1.70 or later
- Ensure all system dependencies for git2 are installed:
  - On Ubuntu/Debian: `sudo apt-get install libssl-dev pkg-config`
  - On macOS: `brew install openssl`
  - On Fedora: `sudo dnf install openssl-devel`

### Runtime Issues

- The tool requires a Tokio runtime for async operations
- File operations require appropriate permissions
- Git operations require a valid git repository
- The program exits gracefully with helpful error messages instead of panicking

### Cache Issues

If you experience issues with cached responses:
```bash
# Disable cache temporarily
export GROK_CACHE=false

# View cache statistics
export DEBUG_CACHE=true
```

### Common Error Messages

1. **"Rate limit exceeded"** - The tool automatically retries with exponential backoff
2. **"Timeout"** - Automatically retried up to 3 times
3. **"Empty response from API"** - Usually indicates an API issue, try again
4. **"Failed to create agent"** - Check your API key configuration

### Getting Help

1. Check your configuration: `grok-code check`
2. Enable debug logging: `export DEBUG_API=1`
3. Review this troubleshooting guide
4. Check the project repository for updates and known issues 