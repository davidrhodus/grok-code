# Grok Code - AI Coding Agent

A powerful CLI tool that provides a Claude-like coding assistant experience using multiple AI providers: xAI's Grok, OpenAI's GPT, or Anthropic's Claude.

```
 ██████╗ ██████╗  ██████╗ ██╗  ██╗    ██╗  ██╗     ██████╗██╗     ██╗
██╔════╝ ██╔══██╗██╔═══██╗██║ ██╔╝    ██║  ██║    ██╔════╝██║     ██║
██║  ███╗██████╔╝██║   ██║█████╔╝     ███████║    ██║     ██║     ██║
██║   ██║██╔══██╗██║   ██║██╔═██╗     ╚════██║    ██║     ██║     ██║
╚██████╔╝██║  ██║╚██████╔╝██║  ██╗         ██║    ╚██████╗███████╗██║
 ╚═════╝ ╚═╝  ╚═╝ ╚═════╝ ╚═╝  ╚═╝         ╚═╝     ╚═════╝╚══════╝╚═╝
```

## Features

- **Interactive Chat Mode**: Have conversations with AI about your code
- **Code Analysis**: Automatically scans and understands your project structure
- **File Operations**: Read, write, and edit files with AI assistance
- **Shell Commands**: Execute commands with safety confirmations
- **Code Search**: Search through your codebase using text or regex
- **Git Integration**: Create commits and submit pull requests
- **Debugging Support**: Analyze errors and logs, suggest fixes
- **Web Search**: Search the web for documentation and solutions
- **Jira Integration**: Create tickets directly from the CLI
- **Response Caching**: Caches API responses for repeated queries to improve performance
- **Concurrent Tool Execution**: Execute multiple independent tools in parallel for faster results
- **Colored Output**: Enhanced readability with color-coded messages, errors, and debug information

## Installation

### From Source

```bash
# Clone the repository
git clone https://github.com/davidrhodus/grok-code.git
cd grok-code

# Build the project
cargo build --release

# The binary will be at ./target/release/grok-code
```

## Configuration

### API Keys

The tool requires an API key from one of the supported providers: xAI (Grok), OpenAI, or Anthropic (Claude).

#### Secure Key Storage (Recommended)

Store your API keys securely in your system's keychain:

```bash
# Store xAI API key
grok-code key set xai YOUR_XAI_API_KEY

# Store OpenAI API key
grok-code key set openai YOUR_OPENAI_API_KEY

# Store Anthropic API key
grok-code key set anthropic YOUR_ANTHROPIC_API_KEY

# List stored keys
grok-code key list

# Remove a stored key
grok-code key delete xai
```

#### Environment Variables

Alternatively, you can use environment variables:

```bash
# For xAI's Grok (default)
export XAI_API_KEY="your-xai-api-key"

# For OpenAI (dev mode)
export OPENAI_API_KEY="your-openai-api-key"

# For Anthropic Claude
export ANTHROPIC_API_KEY="your-anthropic-api-key"
```

The tool checks for API keys in this order:
1. Command-line argument (--api-key)
2. Secure keystore
3. Environment variables

### Optional Environment Variables

```bash
# For GitHub integration
export GITHUB_TOKEN="your-github-token"
export GITHUB_REPO="owner/repo"

# For Jira integration
export JIRA_API_KEY="your-jira-api-key"
export JIRA_URL="https://your-company.atlassian.net"
export JIRA_PROJECT="PROJECT-KEY"
export JIRA_EMAIL="your-email@company.com"  # Optional, defaults to user@email.com

# For Response Caching
export GROK_CACHE="true"      # Enable/disable caching (default: true)
export DEBUG_CACHE="true"      # Show cache statistics after each prompt

# For Performance Tuning
export API_TIMEOUT_SECS="120"  # API timeout in seconds (default: 60 for OpenAI, 300 for xAI)
export API_MAX_RETRIES="5"     # Maximum API retry attempts (default: 3)
```

## Usage

### Basic Usage

```bash
# Interactive mode (default)
grok-code

# TUI mode for better visualization
grok-code --tui

# Non-interactive mode
grok-code prompt -p "Add error handling to the main function"

# Automate a task
grok-code automate -p "refactor all test files to use async/await"

# Auto-run commands without confirmation prompts
grok-code --auto-run prompt -p "Run all tests and fix any issues"

# Check configuration
grok-code check
```

### TUI Mode 🖥️

NEW: Terminal User Interface mode provides a better visualization for chat conversations:

```bash
grok-code --tui
```

**TUI Features:**
- Rich terminal interface using ratatui
- Better visualization of chat messages with color coding
- Keyboard navigation:
  - `i` - Enter input mode to type messages
  - `s` - Enter scroll mode to navigate history
  - `j/k` or arrow keys - Scroll through messages
  - `Enter` - Send message (in input mode)
  - `Esc` - Exit current mode
  - `Ctrl-C` - Quit TUI
- Code diff visualization (coming soon)
- Real-time updates and progress indicators

### Command-Line Options

```
Options:
  --api-key <API_KEY>    xAI API key (or use XAI_API_KEY env var)
  --dev                  Use OpenAI GPT-3.5 for development (cheaper, requires OPENAI_API_KEY)
  --claude               Use Anthropic Claude (requires ANTHROPIC_API_KEY)
  --max-depth <DEPTH>    Max depth for codebase scan [default: 3]
  --dry-run              Print changes without applying them
  --summarize            Generate enhanced codebase summary on startup
  --no-confirm           Skip confirmation prompts
  --auto-run             Automatically run commands without confirmation (alias for --no-confirm)
  -v, --verbose          Enable verbose output (detailed logs)
  --tui                  Enable TUI mode for better visualization
  -h, --help             Print help

Commands:
  prompt     Run a single prompt non-interactively
  automate   Automate a task with pre-confirmation
  check      Check configuration and API setup
  key        Manage API keys in secure storage
```

### Subcommands

- `check`: Verify your configuration and API key setup
- `prompt`: Run a single prompt non-interactively
- `automate`: Automate a task with AI assistance
- `key`: Manage API keys in secure storage
  - `key set <provider> <api_key>`: Store an API key
  - `key delete <provider>`: Remove a stored API key
  - `key list`: Show which providers have stored keys

## Concurrent Tool Execution

When the AI needs to execute multiple tools, it intelligently executes them in parallel when possible:
- Tools that don't depend on git operations run concurrently
- Git operations (commits, PRs, merge conflicts) run sequentially to avoid conflicts
- Results are displayed in the order requested, not completion order

This significantly improves performance when the AI needs to read multiple files, search in different locations, or perform other independent operations simultaneously.

## Available Tools

The AI agent has access to these built-in tools:

1. **read_file**: Read file contents
2. **write_file**: Create or overwrite files (with timestamped backups and retention)
3. **edit_file**: Edit specific lines in a file (with timestamped backups and retention)
4. **list_files**: List directory contents
5. **run_shell_command**: Execute shell commands
6. **search_codebase**: Search for text or regex patterns
7. **debug_code**: Analyze error messages and suggest fixes
8. **analyze_log**: Analyze log files for issues
9. **run_lint**: Run cargo clippy with optional fixes
10. **resolve_merge_conflict**: Intelligently resolve git conflicts with multiple strategies
11. **create_commit**: Create git commits
12. **submit_pr**: Submit GitHub pull requests
13. **web_search**: Search the web via DuckDuckGo
14. **create_jira_ticket**: Create Jira tickets
15. **list_backups**: List all backups for a file
16. **clean_backups**: Clean old backups based on retention policy

## Backup Management 💾

Grok-code now creates timestamped backups instead of simple `.bak` files, with automatic cleanup based on retention policies.

### Backup Features

- **Timestamped Backups**: Files are backed up with timestamps (e.g., `file.txt.20240315_143022.bak`)
- **Automatic Retention**: Old backups are automatically cleaned up based on retention policy
- **Configurable Retention**: Set how many days to keep backups (default: 7 days)
- **Manual Cleanup**: Use the `clean_backups` tool to manually clean old backups

### Configuration

Set the backup retention period using environment variable:
```bash
# Keep backups for 30 days (default: 7)
export GROK_BACKUP_RETENTION_DAYS=30

# Disable automatic cleanup (keep forever)
export GROK_BACKUP_RETENTION_DAYS=0
```

### Examples

```bash
# List all backups for a file
grok-code prompt -p "List backups for src/main.rs"

# Clean old backups for a specific file
grok-code prompt -p "Clean old backups for config.json"

# Clean all old backups in the project
grok-code prompt -p "Clean all old backups in the project"
```

## Plugin System

Extend grok-code with custom tools using the plugin system. Plugins are external scripts or programs that implement the tool protocol.

### Creating Plugins

1. **Create a plugin script** (Python, Shell, Node.js, etc.):
```python
#!/usr/bin/env python3
import json
import sys

# Read arguments from stdin
args = json.loads(sys.stdin.read())

# Your tool logic here
result = {"output": f"Processed: {args.get('input', '')}"}

# Output JSON result
print(json.dumps(result))
```

2. **Create a plugin configuration** (TOML or JSON):
```toml
[[plugins]]
name = "my_custom_tool"
description = "Description of what the tool does"
type = "script"
command = "python3 /path/to/my_script.py"
parameters = '''
{
  "type": "object",
  "properties": {
    "input": {"type": "string", "description": "Input parameter"}
  },
  "required": ["input"]
}
'''
```

### Plugin Locations

Plugins are loaded from these locations (in order):
1. Custom directory: `$GROK_PLUGIN_DIR`
2. Project directory: `./plugins/`
3. User config: `~/.grok-code/plugins/`
4. System config: `$XDG_CONFIG_HOME/grok-code/plugins/`
5. Single file: `$GROK_PLUGIN_FILE`

### Plugin Configuration

Control plugin loading with environment variables:
```bash
# Disable all plugins
export GROK_PLUGINS=false

# Use custom plugin directory
export GROK_PLUGIN_DIR=/my/custom/plugins

# Load a single plugin file
export GROK_PLUGIN_FILE=/path/to/plugin.toml
```

### Example Plugins

The `plugins/` directory contains example plugins:
- **word_counter**: Count words with pattern matching
- **count_lines**: Count lines using `wc`
- **format_json**: Format JSON using `jq`
- **run_pytest**: Run Python tests

To use them:
```bash
# Copy example plugins
cp plugins/example-plugins.toml plugins/

# Run grok-code - plugins will be loaded automatically
grok-code
```

## Merge Conflict Resolution

The `resolve_merge_conflict` tool offers intelligent conflict resolution with multiple strategies:

- **auto** (default): Intelligently resolves conflicts based on content analysis
  - Chooses non-empty content over empty
  - Merges import statements
  - Selects higher version numbers
  - Prefers code with more comments/documentation
  - Combines both versions when uncertain
- **ours**: Keeps only the current branch version
- **theirs**: Keeps only the incoming branch version
- **both**: Keeps both versions concatenated

Example:
```bash
# Auto-resolve conflicts intelligently
grok-code prompt -p "Resolve merge conflicts in src/main.rs"

# Use specific strategy
grok-code prompt -p "Resolve conflicts in config.json using 'theirs' strategy"
```

## Examples

### Code Review
```bash
grok-code prompt -p "Review the error handling in my Rust code and suggest improvements"
```

### Debugging
```bash
cargo test 2>&1 | grok-code prompt
```

### Automated Refactoring
```bash
grok-code automate -p "Convert all unwrap() calls to proper error handling with ? operator"
```

### Project Analysis
```bash
grok-code --summarize prompt -p "What are the main components of this project?"
```

## Safety Features

- **Backup Creation**: Files are automatically backed up with timestamps before modification
- **Confirmation Prompts**: Dangerous operations require confirmation
- **Dry-Run Mode**: Preview changes before applying them
- **Codebase Isolation**: Operations are restricted to the project directory
- **Smart Error Handling**: Automatic retry logic for transient errors with context-aware error messages

## Error Handling

Grok-code uses structured error handling with enhanced features:

- **Context-Aware Errors**: Errors include contextual information for better debugging
- **Automatic Retries**: Network errors and rate limits are automatically retried with exponential backoff
- **Error Recovery**: Different error types trigger appropriate recovery strategies
- **Detailed Diagnostics**: Verbose mode shows detailed error information

### Retry Logic

The following errors are automatically retried:
- **Rate Limits**: Respects retry-after headers or uses default delays
- **Network Timeouts**: Retries with increasing delays
- **HTTP Errors**: Transient HTTP errors are retried

Use `--verbose` to see retry attempts and error details.

## Development

### Building from Source

```bash
cargo build --release
```

### TODO Comments

The codebase includes TODO comments marking areas for future improvement:
- **External APIs**: Implementation of actual HTTP requests for web search and Jira integration
- **Tool System**: Plugin system, tool dependencies, and inter-tool communication
- **Caching**: LRU eviction, persistence, and compression
- **Error Handling**: Recovery strategies and better error context
- **Testing**: More comprehensive test coverage for all components

### Running Tests

```bash
# Run all tests
cargo test

# Run with verbose output
cargo test -- --nocapture

# Run tests with code coverage
./scripts/coverage.sh

# Or manually with tarpaulin
cargo install cargo-tarpaulin
cargo tarpaulin --out Html --output-dir ./target/coverage

# Profile performance with flamegraph
./scripts/profile.sh
```

The project includes comprehensive test coverage including unit tests, integration tests, and edge case testing. Coverage reports are generated in HTML and LCOV formats.

### Performance Profiling

To identify performance bottlenecks:

```bash
# Run the profiling script
./scripts/profile.sh

# Or manually with cargo flamegraph
cargo install flamegraph
cargo flamegraph --release -- prompt -p "Analyze codebase"
```

The profiling script offers different scenarios to test various aspects of the application performance.

### Verbose Mode

Use the `--verbose` flag to see detailed debugging information:

```bash
# Run with verbose output
grok-code --verbose

# Combine with other flags
grok-code --verbose --dev
grok-code --verbose --claude
```

Verbose mode shows:
- API request/response details
- Cache hit/miss statistics
- Tool execution timing
- File operation details (sizes, line counts)
- Search results count
- Project scanning statistics

### Security

We regularly run `cargo audit` to check for security vulnerabilities. As of the latest check:
- No security vulnerabilities found
- 2 warnings about unmaintained transitive dependencies (`derivative` and `instant`) from the `keyring` crate

To run your own security audit:
```bash
cargo install cargo-audit
cargo audit
```

### API Documentation

Generate and view the API documentation:

```bash
# Generate documentation
./scripts/generate-docs.sh

# Generate and open in browser
./scripts/generate-docs.sh --open

# Generate with private items
./scripts/generate-docs.sh --private

# Prepare for GitHub Pages deployment
./scripts/generate-docs.sh --deploy
```

The documentation includes:
- Complete API reference for all public modules
- Usage examples and code snippets
- Custom styling with the grok-code branding
- Module organization and relationships

### Contributing

Contributions are welcome! Please ensure:
- Code follows Rust conventions
- Run `cargo fmt` before committing
- Run `cargo clippy` and fix any warnings
- All tests pass (`cargo test`)
- New features include documentation

## License

This project is licensed under the MIT License.

## Troubleshooting

### Slow API Responses
**⚠️ Important**: xAI's Grok API can be extremely slow (3-5+ minutes per response). This is normal behavior.

- The program shows progress updates every 15 seconds: `🤔 Thinking... (15s) (30s)`
- Responses are automatically retried if they timeout (>5 minutes)
- For faster responses, use OpenAI instead: `grok-code --dev`

### Configuration Check
Run the check command to verify your setup:
```bash
grok-code check
```

This will show:
- Which API provider you're using
- If your API key is properly configured
- Status of optional environment variables with validation:
  - **GitHub**: Warns if GITHUB_TOKEN is set but GITHUB_REPO is missing
  - **Jira**: Validates URL format and warns if any required variables are missing
  - **Performance**: Shows defaults and validates GROK_CACHE values
- Color-coded output: ✅ green for set, ❌ red for unset, ⚠️ yellow for warnings
- Any configuration issues with helpful suggestions

### API Key Issues
- If you see "API key is required", the tool will show detailed setup instructions
- Ensure your API key is correctly set in the environment
- Check API rate limits if you get 429 errors
- Use `grok-code check` to verify your configuration

### Build Issues
- Requires Rust 1.70 or later
- Ensure all system dependencies for git2 are installed

### Runtime Issues
- The tool requires a Tokio runtime for async operations
- File operations require appropriate permissions
- The program now exits gracefully with helpful error messages instead of panicking

## Future Enhancements

- [ ] Support for more LLM providers
- [ ] Plugin system for custom tools
- [ ] Web UI for better code visualization
- [ ] Integration with more development tools
- [ ] Improved conflict resolution strategies 
