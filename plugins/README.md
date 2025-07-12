# Grok-Code Plugins

This directory contains example plugins that extend grok-code with custom tools.

## Plugin File Format

Plugin configuration files can be in TOML or JSON format. For TOML files, use the following structure:

```toml
[[plugins]]
name = "tool_name"
description = "What this tool does"
type = "script"  # or "binary"
command = "python script.py"  # Command to execute
parameters = '''
{
  "type": "object",
  "properties": {
    "input": {"type": "string", "description": "Input parameter"}
  },
  "required": ["input"]
}
'''

# Optional fields
working_dir = "/path/to/dir"  # Working directory for the command
env = { KEY = "value" }       # Environment variables
```

## Example Plugins

### word-counter.toml
A Python-based tool that counts words in text with optional pattern filtering.

### example-plugins.toml
Contains several example plugin configurations:
- `count_lines` - Uses `wc -l` to count lines in files
- `format_json` - Uses `jq` to format JSON
- `run_pytest` - Runs Python tests with pytest
- `disk_usage` - Shows disk usage with `du`

## Creating Your Own Plugins

1. Create a new TOML or JSON file in this directory
2. Define your plugin configuration using the format above
3. Ensure the command in your plugin is available in the system PATH
4. The plugin will be automatically loaded when grok-code starts

## Plugin Protocol

Plugins communicate via stdin/stdout:
- **Input**: JSON arguments are sent to the plugin's stdin
- **Output**: The plugin should write its response to stdout
- **Errors**: Error messages should be written to stderr

Example Python plugin:
```python
#!/usr/bin/env python3
import json
import sys

# Read arguments from stdin
args = json.loads(sys.stdin.read())

# Process the arguments
result = f"Processed: {args.get('input', '')}"

# Output the result
print(result)
```

## Testing Plugins

To test if your plugins are loading correctly:
```bash
cargo run -- --dev
# Look for "Loaded N plugins from plugins/..." messages
```

To use a plugin:
```bash
cargo run -- prompt -p "Use the word_counter tool to count words in 'hello world'"
``` 