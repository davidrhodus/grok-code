# Quick Start Guide for Grok Code

## 1. Build the Project

```bash
cargo build --release
```

## 2. Set Up Your API Key

Choose one of these options:

### Option A: Use xAI's Grok (Production)
```bash
export XAI_API_KEY='your-xai-api-key'
```
Get your key at: https://x.ai/api

### Option B: Use OpenAI (Development/Cheaper)
```bash
export OPENAI_API_KEY='your-openai-api-key'
```
Get your key at: https://platform.openai.com/api-keys

## 3. Verify Your Setup

```bash
./target/release/grok-code check
```

You should see:
```
✅ Configuration check passed!
Using xAI API at https://api.x.ai/v1
Model: grok-4
API key: your...key
```

## 4. Start Using Grok Code

### Interactive Mode
```bash
./target/release/grok-code
```

### Quick Command
```bash
./target/release/grok-code prompt -p "What does this codebase do?"
```

### Development Mode (OpenAI)
```bash
./target/release/grok-code --dev
```

## Common First Commands

1. **Understand your project:**
   ```
   You: Give me an overview of this codebase
   ```

2. **Find specific code:**
   ```
   You: Where is error handling implemented?
   ```

3. **Make changes:**
   ```
   You: Add input validation to the main function
   ```

4. **Debug issues:**
   ```
   You: Help me fix the compilation error in line 500
   ```

## Tips

- Use `--dry-run` to preview changes before applying them
- Files are automatically backed up with `.bak` extension
- Type `exit` to quit interactive mode
- Run `./demo.sh` for more usage examples

## Need Help?

- Run without API key to see setup instructions
- Check `./README.md` for full documentation
- Use `grok-code --help` for all options 