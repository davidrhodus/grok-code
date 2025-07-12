use clap::{Parser, Subcommand};
use colored::*;
use grok_code::agent::GrokAgent;
use grok_code::api::{ApiConfig, Message};
use grok_code::keystore::KeyStore;
use grok_code::tui::{init_terminal, restore_terminal, TuiApp};
use std::env;
use std::io::{self, BufRead, IsTerminal, Read, Write};

#[derive(Parser)]
#[command(name = "grok-code")]
#[command(about = "MVP Claude-like coding agent using Grok API")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    #[arg(long, help = "xAI API key. If not provided, uses XAI_API_KEY env var.")]
    api_key: Option<String>,

    #[arg(
        long,
        help = "Use cheaper OpenAI model for development (requires OPENAI_API_KEY)"
    )]
    dev: bool,

    #[arg(long, help = "Use Anthropic Claude (requires ANTHROPIC_API_KEY)")]
    claude: bool,

    #[arg(long, default_value_t = 3, help = "Max depth for codebase scan")]
    max_depth: usize,

    #[arg(long, help = "Enable dry-run mode (print changes without applying)")]
    dry_run: bool,

    #[arg(
        long,
        help = "Use API to generate enhanced codebase summary on startup"
    )]
    summarize: bool,

    #[arg(long, help = "Skip confirmation prompts for actions")]
    no_confirm: bool,

    #[arg(
        long,
        help = "Automatically run commands without confirmation (same as --no-confirm)"
    )]
    auto_run: bool,

    #[arg(short, long, help = "Enable verbose output (show detailed logs)")]
    verbose: bool,

    #[arg(long, help = "Disable TUI mode and use standard terminal interface")]
    no_tui: bool,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "Run a single prompt non-interactively")]
    Prompt {
        #[arg(
            short,
            long,
            help = "The prompt to execute (if not provided, read from stdin)"
        )]
        prompt: Option<String>,
    },
    #[command(about = "Automate a task non-interactively")]
    Automate {
        #[arg(short, long, help = "The task prompt to automate")]
        prompt: String,
    },
    #[command(about = "Check your configuration and API key setup")]
    Check,
    #[command(about = "Manage API keys in secure storage")]
    Key {
        #[command(subcommand)]
        action: KeyCommands,
    },
}

#[derive(Subcommand)]
enum KeyCommands {
    #[command(about = "Store an API key securely")]
    Set {
        #[arg(help = "Provider (xai or openai)")]
        provider: String,
        #[arg(help = "API key to store")]
        api_key: String,
    },
    #[command(about = "Remove an API key from secure storage")]
    Delete {
        #[arg(help = "Provider (xai or openai)")]
        provider: String,
    },
    #[command(about = "Show stored API key providers")]
    List,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let project_root = match std::env::current_dir() {
        Ok(dir) => dir,
        Err(e) => {
            eprintln!(
                "{} Failed to get current directory: {}",
                "❌".red(),
                e.to_string().red()
            );
            std::process::exit(1);
        }
    };

    println!(
        r#"
 ██████╗ ██████╗  ██████╗ ██╗  ██╗    ██╗  ██╗     ██████╗██╗     ██╗
██╔════╝ ██╔══██╗██╔═══██╗██║ ██╔╝    ██║  ██║    ██╔════╝██║     ██║
██║  ███╗██████╔╝██║   ██║█████╔╝     ███████║    ██║     ██║     ██║
██║   ██║██╔══██╗██║   ██║██╔═██╗     ╚════██║    ██║     ██║     ██║
╚██████╔╝██║  ██║╚██████╔╝██║  ██╗         ██║    ╚██████╗███████╗██║
 ╚═════╝ ╚═╝  ╚═╝ ╚═════╝ ╚═╝  ╚═╝         ╚═╝     ╚═════╝╚══════╝╚═╝
    "#
    );
    println!("Grok Code MVP: Chat and code with Grok 4 in your terminal.");
    println!();

    // Validate CLI flags
    if cli.dev && cli.claude {
        eprintln!(
            "{} Cannot use both --dev and --claude flags together.",
            "❌".red()
        );
        eprintln!("{}", "Choose one:".yellow());
        eprintln!("  {} : Use OpenAI GPT-3.5 (cheap, fast)", "--dev".cyan());
        eprintln!(
            "  {} : Use Anthropic Claude (advanced reasoning)",
            "--claude".cyan()
        );
        eprintln!("  {} : Use xAI Grok (default)", "(neither)".cyan());
        std::process::exit(1);
    }

    // Enable verbose mode
    if cli.verbose {
        env::set_var("DEBUG_API", "1");
        env::set_var("DEBUG_CACHE", "1");
        println!("{} {}", "🔍".blue(), "Verbose mode enabled".blue().bold());
        println!("   {} API requests/responses will be logged", "-".dimmed());
        println!("   {} Cache statistics will be shown", "-".dimmed());
        println!();
    }

    // Handle key management commands first
    if let Some(Commands::Key { action }) = &cli.command {
        let keystore = KeyStore::new();
        match action {
            KeyCommands::Set { provider, api_key } => {
                match provider.to_lowercase().as_str() {
                    "xai" | "openai" | "anthropic" => {
                        match keystore.set_api_key(provider, api_key) {
                            Ok(_) => {
                                println!(
                                    "{} API key for {} stored securely.",
                                    "✅".green(),
                                    provider.bold()
                                );
                                println!();
                                println!("{}", "You can now use grok-code without setting environment variables!".green());
                            }
                            Err(e) => {
                                eprintln!(
                                    "{} Failed to store API key: {}",
                                    "❌".red(),
                                    e.to_string().red()
                                );
                                std::process::exit(1);
                            }
                        }
                    }
                    _ => {
                        eprintln!("❌ Invalid provider. Use 'xai', 'openai', or 'anthropic'.");
                        std::process::exit(1);
                    }
                }
            }
            KeyCommands::Delete { provider } => match keystore.delete_api_key(provider) {
                Ok(_) => println!("✅ API key for {provider} removed from secure storage."),
                Err(e) => eprintln!("❌ Failed to delete API key: {e}"),
            },
            KeyCommands::List => {
                println!("Stored API key providers:");
                for provider in ["xai", "openai", "anthropic"] {
                    if keystore.has_api_key(provider) {
                        println!("  ✅ {provider}");
                    }
                }
            }
        }
        return Ok(());
    }

    let keystore = KeyStore::new();
    let (api_key, base_url, model, provider_name) = if cli.claude {
        // Try keystore first, then environment variable
        let key = keystore
            .get_api_key("anthropic")
            .ok()
            .or_else(|| std::env::var("ANTHROPIC_API_KEY").ok());

        match key {
            Some(key) => (
                key,
                "https://api.anthropic.com/v1".to_string(),
                "claude-3-opus-20240229".to_string(),
                "anthropic",
            ),
            None => {
                eprintln!("❌ Claude mode requires an Anthropic API key.");
                eprintln!();
                eprintln!("To use Claude mode with Anthropic:");
                eprintln!("  1. Store API key securely (recommended):");
                eprintln!("     grok-code key set anthropic YOUR_API_KEY");
                eprintln!();
                eprintln!("  2. Or set environment variable:");
                eprintln!("     export ANTHROPIC_API_KEY='your-anthropic-api-key'");
                eprintln!();
                eprintln!("Or use other modes:");
                eprintln!("  grok-code       # xAI's Grok (default)");
                eprintln!("  grok-code --dev # OpenAI's GPT-3.5");
                eprintln!();
                eprintln!("Get your API keys from:");
                eprintln!("  Anthropic: https://console.anthropic.com/");
                eprintln!("  OpenAI: https://platform.openai.com/api-keys");
                eprintln!("  xAI: https://x.ai/api");
                std::process::exit(1);
            }
        }
    } else if cli.dev {
        // Try keystore first, then environment variable
        let key = keystore
            .get_api_key("openai")
            .ok()
            .or_else(|| std::env::var("OPENAI_API_KEY").ok());

        match key {
            Some(key) => (
                key,
                "https://api.openai.com/v1".to_string(),
                "gpt-3.5-turbo".to_string(),
                "openai",
            ),
            None => {
                eprintln!("❌ Development mode requires an OpenAI API key.");
                eprintln!();
                eprintln!("To use development mode with OpenAI:");
                eprintln!("  1. Store API key securely (recommended):");
                eprintln!("     grok-code key set openai YOUR_API_KEY");
                eprintln!();
                eprintln!("  2. Or set environment variable:");
                eprintln!("     export OPENAI_API_KEY='your-openai-api-key'");
                eprintln!();
                eprintln!("Or use production mode with xAI's Grok:");
                eprintln!("  grok-code  # without --dev flag");
                eprintln!();
                eprintln!("Get your API keys from:");
                eprintln!("  OpenAI: https://platform.openai.com/api-keys");
                eprintln!("  xAI: https://x.ai/api");
                std::process::exit(1);
            }
        }
    } else {
        // Try command line, then keystore, then environment variable
        let key = cli
            .api_key
            .or_else(|| keystore.get_api_key("xai").ok())
            .or_else(|| std::env::var("XAI_API_KEY").ok());

        match key {
            Some(key) => (
                key,
                "https://api.x.ai/v1".to_string(),
                "grok-4-0709".to_string(),
                "xai",
            ),
            None => {
                eprintln!("❌ API key is required. No API key found.");
                eprintln!();
                eprintln!("To use Grok Code, you need to provide an API key in one of these ways:");
                eprintln!();
                eprintln!("1. Store API key securely (recommended):");
                eprintln!("   grok-code key set xai YOUR_API_KEY");
                eprintln!("   grok-code");
                eprintln!();
                eprintln!("2. Set environment variable:");
                eprintln!("   export XAI_API_KEY='your-xai-api-key'");
                eprintln!("   grok-code");
                eprintln!();
                eprintln!("3. Pass via command line (not recommended for security):");
                eprintln!("   grok-code --api-key 'your-xai-api-key'");
                eprintln!();
                eprintln!("4. Use development mode with OpenAI (cheaper):");
                eprintln!("   grok-code key set openai YOUR_OPENAI_KEY");
                eprintln!("   grok-code --dev");
                eprintln!();
                eprintln!("Get your API keys from:");
                eprintln!("  xAI (Grok): https://x.ai/api");
                eprintln!("  OpenAI: https://platform.openai.com/api-keys");
                eprintln!();
                eprintln!("For more information, see: ./README.md");
                std::process::exit(1);
            }
        }
    };

    // Handle check command first before creating agent
    if let Some(Commands::Check) = &cli.command {
        println!(
            "{} {}",
            "✅".green(),
            "Configuration check passed!".green().bold()
        );
        println!();
        let display_name = match provider_name {
            "anthropic" => "Anthropic Claude",
            "openai" => "OpenAI",
            "xai" => "xAI",
            _ => provider_name,
        };
        println!("Using {display_name} API at {base_url}");
        println!("Model: {model}");
        if provider_name == "xai" && model != "grok-4-0709" {
            println!("⚠️  Note: You're using model '{model}' but xAI typically uses 'grok-4-0709'");
        }
        println!(
            "API key: {}...{}",
            &api_key[..4],
            &api_key[api_key.len() - 4..]
        );
        println!();
        println!("{}", "Secure key storage:".bold());
        println!(
            "  xAI key in keystore: {}",
            if keystore.has_api_key("xai") {
                "✅ Set".green()
            } else {
                "❌ Not set".red()
            }
        );
        println!(
            "  OpenAI key in keystore: {}",
            if keystore.has_api_key("openai") {
                "✅ Set".green()
            } else {
                "❌ Not set".red()
            }
        );
        println!(
            "  Anthropic key in keystore: {}",
            if keystore.has_api_key("anthropic") {
                "✅ Set".green()
            } else {
                "❌ Not set".red()
            }
        );
        println!();
        println!("{}", "Optional environment variables:".bold());

        // GitHub configuration
        let github_token_set = env::var("GITHUB_TOKEN").is_ok();
        let github_repo = env::var("GITHUB_REPO").ok();
        println!(
            "  GITHUB_TOKEN: {}",
            if github_token_set {
                "✅ Set".green()
            } else {
                "❌ Not set".red()
            }
        );
        println!(
            "  GITHUB_REPO: {}",
            github_repo
                .as_ref()
                .map(|v| v.cyan().to_string())
                .unwrap_or_else(|| "Not set".dimmed().to_string())
        );

        if github_token_set && github_repo.is_none() {
            println!("    {} GITHUB_TOKEN is set but GITHUB_REPO is not. Both are needed for GitHub integration.", "⚠️".yellow());
        }

        // Jira configuration
        let jira_key_set = env::var("JIRA_API_KEY").is_ok();
        let jira_url = env::var("JIRA_URL").ok();
        let jira_project = env::var("JIRA_PROJECT").ok();

        println!(
            "  JIRA_API_KEY: {}",
            if jira_key_set {
                "✅ Set".green()
            } else {
                "❌ Not set".red()
            }
        );
        println!(
            "  JIRA_URL: {}",
            jira_url
                .as_ref()
                .map(|v| {
                    // Validate URL format
                    if v.starts_with("https://") && v.contains("atlassian.net") {
                        v.cyan().to_string()
                    } else {
                        format!(
                            "{} ({})",
                            v.yellow(),
                            "should be https://your-company.atlassian.net".dimmed()
                        )
                    }
                })
                .unwrap_or_else(|| "Not set".dimmed().to_string())
        );
        println!(
            "  JIRA_PROJECT: {}",
            jira_project
                .as_ref()
                .map(|v| v.cyan().to_string())
                .unwrap_or_else(|| "Not set".dimmed().to_string())
        );
        println!(
            "  JIRA_EMAIL: {}",
            env::var("JIRA_EMAIL")
                .map(|v| v.cyan().to_string())
                .unwrap_or_else(|_| "Not set (defaults to user@email.com)".dimmed().to_string())
        );

        if (jira_key_set || jira_url.is_some() || jira_project.is_some())
            && (!jira_key_set || jira_url.is_none() || jira_project.is_none())
        {
            println!("    {} Jira integration requires JIRA_API_KEY, JIRA_URL, and JIRA_PROJECT to all be set.", "⚠️".yellow());
        }

        // Performance tuning
        println!();
        println!("{}", "Performance tuning:".bold());
        let default_timeout = match provider_name {
            "openai" | "anthropic" => 60,
            _ => 300,
        };
        println!(
            "  API_TIMEOUT_SECS: {}",
            env::var("API_TIMEOUT_SECS")
                .map(|v| v.cyan().to_string())
                .unwrap_or_else(|_| format!("Not set (default: {default_timeout})")
                    .dimmed()
                    .to_string())
        );
        println!(
            "  API_MAX_RETRIES: {}",
            env::var("API_MAX_RETRIES")
                .map(|v| v.cyan().to_string())
                .unwrap_or_else(|_| "Not set (default: 3)".dimmed().to_string())
        );
        println!(
            "  GROK_CACHE: {}",
            env::var("GROK_CACHE")
                .map(|v| {
                    if v == "true" || v == "false" {
                        v.cyan().to_string()
                    } else {
                        format!(
                            "{} ({})",
                            v.yellow(),
                            "should be 'true' or 'false'".dimmed()
                        )
                    }
                })
                .unwrap_or_else(|_| "Not set (default: true)".dimmed().to_string())
        );
        println!(
            "  DEBUG_API: {}",
            env::var("DEBUG_API")
                .map(|v| v.cyan().to_string())
                .unwrap_or_else(|_| "Not set".dimmed().to_string())
        );
        println!(
            "  DEBUG_CACHE: {}",
            env::var("DEBUG_CACHE")
                .map(|v| v.cyan().to_string())
                .unwrap_or_else(|_| "Not set".dimmed().to_string())
        );

        println!();
        println!(
            "{} Run {} to start using grok-code!",
            "💡".blue(),
            "grok-code".green().bold()
        );
        return Ok(());
    }

    match provider_name {
        "anthropic" => {
            println!("🤖 CLAUDE MODE: Using Anthropic Claude-3 Opus");
            println!("   Claude offers excellent reasoning and coding capabilities.");
        }
        "openai" => {
            println!("🚧 DEVELOPMENT MODE: Using OpenAI gpt-3.5-turbo (cheaper)");
        }
        "xai" => {
            println!("⚠️  Note: xAI's Grok API can be slow. Responses may take 3-5 minutes.");
            println!("   The program will show progress updates every 15 seconds.");
            println!("   Use --dev flag for faster responses with OpenAI.");
            println!("   Use --claude flag for Claude's advanced capabilities.");
        }
        _ => {}
    }

    println!(
        "Type 'exit' to quit. Use 'grok-code prompt -p \"Your prompt\"' for non-interactive mode."
    );
    println!("Use --no-tui flag to disable the TUI interface.");

    // Create API configuration
    let provider = provider_name;

    // Get timeout and retry configuration from environment
    let timeout_secs = env::var("API_TIMEOUT_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(match provider {
            "openai" | "anthropic" => 60,
            _ => 300,
        }); // Default: 60s for OpenAI/Anthropic, 300s for xAI

    let max_retries = env::var("API_MAX_RETRIES")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(3); // Default: 3 retries

    let api_config = ApiConfig {
        api_key,
        base_url,
        model,
        timeout_secs,
        max_retries,
    };

    let mut agent = match GrokAgent::new(
        provider,
        api_config,
        project_root,
        cli.dry_run,
        cli.max_depth,
        cli.no_confirm || cli.auto_run, // Use either flag
    ) {
        Ok(agent) => agent,
        Err(e) => {
            eprintln!("❌ Failed to create agent: {e}");
            std::process::exit(1);
        }
    };

    if cli.summarize {
        if let Err(e) = agent.enhance_summary().await {
            println!("Failed to enhance summary: {e}");
        }
    }

    match cli.command {
        Some(Commands::Check) => {
            // Already handled above
            unreachable!();
        }
        Some(Commands::Key { .. }) => {
            // Already handled above
            unreachable!();
        }
        Some(Commands::Prompt { prompt }) => {
            let mut user_prompt = prompt.unwrap_or_default();
            if user_prompt.is_empty() && !io::stdin().is_terminal() {
                let mut stdin_content = String::new();
                io::stdin()
                    .read_to_string(&mut stdin_content)
                    .expect("Failed to read stdin");
                user_prompt = stdin_content.trim().to_string();
            }
            agent.process_prompt(&user_prompt, false).await;
        }
        Some(Commands::Automate { prompt }) => {
            let auto_prompt = format!("Automate task: {prompt}");
            agent.process_prompt(&auto_prompt, false).await;
        }
        None => {
            if !cli.no_tui {
                // Run in TUI mode (default)
                // Initialize terminal
                let mut terminal = match init_terminal() {
                    Ok(term) => term,
                    Err(e) => {
                        eprintln!("❌ Failed to initialize TUI: {e}");
                        std::process::exit(1);
                    }
                };

                // Create TUI app
                let mut tui_app = TuiApp::new();

                // Add initial system message
                tui_app.add_message(&Message {
                    role: "system".to_string(),
                    content: Some(format!(
                        "Welcome to Grok Code TUI mode! Using {provider_name} API."
                    )),
                    tool_calls: None,
                    tool_call_id: None,
                });

                // Run TUI loop
                loop {
                    match tui_app.run(&mut terminal).await {
                        Ok(Some(input)) => {
                            // Add user message to TUI
                            tui_app.add_message(&Message {
                                role: "user".to_string(),
                                content: Some(input.clone()),
                                tool_calls: None,
                                tool_call_id: None,
                            });

                            // Mark as processing
                            tui_app.set_processing(true);
                            terminal.draw(|f| tui_app.draw(f))?;

                            // Create channel for agent updates
                            let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

                            // Set up TUI to receive updates
                            tui_app.set_update_receiver(rx);

                            // Set up agent to send updates
                            agent.set_tui_sender(tx);

                            // Process the prompt - agent will send updates through channel
                            agent.process_prompt(&input, true).await;

                            // Clear the TUI sender so agent doesn't send updates when not in TUI
                            agent.set_tui_sender(tokio::sync::mpsc::unbounded_channel().0);
                        }
                        Ok(None) => {
                            // User quit TUI
                            break;
                        }
                        Err(e) => {
                            eprintln!("TUI error: {e}");
                            break;
                        }
                    }
                }

                // Restore terminal
                if let Err(e) = restore_terminal(&mut terminal) {
                    eprintln!("Failed to restore terminal: {e}");
                }

                println!("Goodbye!");
            } else {
                // Standard interactive mode
                let stdin = io::stdin();
                loop {
                    print!("You: ");
                    io::stdout().flush().unwrap();
                    let mut user_input = String::new();
                    stdin.lock().read_line(&mut user_input).unwrap();
                    let user_input = user_input.trim();

                    if user_input.to_lowercase() == "exit" {
                        println!("Goodbye!");
                        break;
                    }

                    agent.process_prompt(user_input, true).await;
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {

    use grok_code::agent::GrokAgent;
    use std::env;
    use std::fs;

    /// Test codebase summary generation
    #[test]
    fn test_generate_codebase_summary() {
        let temp_dir = env::temp_dir().join("grok_test_summary");
        let _ = fs::remove_dir_all(&temp_dir); // Clean up if exists
        fs::create_dir_all(&temp_dir).unwrap();

        // Create test files
        fs::write(temp_dir.join("README.md"), "# Test Project").unwrap();
        fs::create_dir_all(temp_dir.join("src")).unwrap();
        fs::write(temp_dir.join("src/lib.rs"), "// Library code").unwrap();

        let summary = GrokAgent::generate_codebase_summary(&temp_dir, 2);

        assert!(summary.contains("Project structure:"));
        assert!(summary.contains("README.md"));
        assert!(summary.contains("src/lib.rs"));

        // Cleanup
        fs::remove_dir_all(&temp_dir).unwrap();
    }

    // TODO: Add more tests once we have a mock API client for testing
    // TODO: Test CLI argument parsing and validation
    // TODO: Test API key management (set/get/delete)
    // TODO: Test error handling for various failure scenarios
    // TODO: Integration tests for the complete workflow
}
