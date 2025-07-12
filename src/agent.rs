use crate::api::{
    ApiClient, ApiConfig, ChatCompletionRequest, ChatCompletionResponse, Function, Message,
    ResponseFormat, Tool,
};
use crate::cache::ResponseCache;
use crate::error::{GrokError, Result};
use crate::tools::{ToolContext, ToolRegistry};
use git2::Repository;
use std::path::{Path, PathBuf};
use tokio::sync::mpsc;
use walkdir::WalkDir;

/// Message types for TUI communication
#[derive(Debug, Clone)]
pub enum TuiUpdate {
    Message(Message),
    ToolStart { name: String, icon: String },
    ToolResult { name: String, result: String },
    Processing { message: String },
    Error { message: String },
    Complete,
}

pub struct GrokAgent {
    api_client: Box<dyn ApiClient>,
    messages: Vec<Message>,
    tool_registry: ToolRegistry,
    temperature: f64,
    max_tokens: u32,
    project_root: PathBuf,
    codebase_summary: String,
    dry_run: bool,
    no_confirm: bool,
    git_repo: Option<Repository>,
    response_cache: ResponseCache,
    tui_sender: Option<mpsc::UnboundedSender<TuiUpdate>>,
}

impl GrokAgent {
    /// Create a new agent
    pub fn new(
        provider: &str,
        api_config: ApiConfig,
        project_root: PathBuf,
        dry_run: bool,
        max_depth: usize,
        auto_approve: bool,
    ) -> std::result::Result<Self, Box<dyn std::error::Error>> {
        // Create the tool registry
        let tool_registry = ToolRegistry::new();

        let codebase_summary = Self::generate_codebase_summary(&project_root, max_depth);

        let system_message = format!(
            "You are Grok Code, a helpful coding agent. You have access to the user's project at {}. Summary: {}\n\nUse the available tools to help the user. Be smart about which tools to use - for simple summaries, you might only need to read key files. For deeper analysis or when asked to find issues, you can use lint or search tools. Match your tool usage to what the user actually asked for.",
            project_root.display(),
            codebase_summary
        );

        let messages = vec![Message {
            role: "system".to_string(),
            content: Some(system_message),
            tool_calls: None,
            tool_call_id: None,
        }];

        let git_repo = Repository::open(&project_root).ok();

        // Create the API client
        let api_client = crate::api::create_client(provider, api_config)?;

        Ok(GrokAgent {
            api_client,
            messages,
            tool_registry,
            temperature: 0.7,
            max_tokens: 4096,
            project_root,
            codebase_summary,
            dry_run,
            no_confirm: auto_approve,
            git_repo,
            response_cache: ResponseCache::new(100, 300), // 100 entries, 5 minute TTL
            tui_sender: None,
        })
    }

    /// Set the TUI update channel
    pub fn set_tui_sender(&mut self, sender: mpsc::UnboundedSender<TuiUpdate>) {
        self.tui_sender = Some(sender);
    }

    /// Send update to TUI if available, otherwise print to stdout
    fn send_update(&self, update: TuiUpdate) {
        if let Some(sender) = &self.tui_sender {
            let _ = sender.send(update);
        } else {
            // Fallback to stdout when not in TUI mode
            match update {
                TuiUpdate::Message(msg) => {
                    if let Some(content) = &msg.content {
                        println!("💬 {}", content);
                    }
                }
                TuiUpdate::ToolStart { name, icon } => {
                    println!("{} {}...", icon, name);
                }
                TuiUpdate::ToolResult { name: _, result } => {
                    println!("{}", result);
                }
                TuiUpdate::Processing { message } => {
                    print!("{}", message);
                    use std::io::{self, Write};
                    io::stdout().flush().unwrap();
                }
                TuiUpdate::Error { message } => {
                    eprintln!("❌ {}", message);
                }
                TuiUpdate::Complete => {
                    // No-op for stdout
                }
            }
        }
    }

    /// Get the recent messages for TUI display
    pub fn get_recent_messages(&self, count: usize) -> Vec<Message> {
        let start = self.messages.len().saturating_sub(count);
        self.messages[start..].to_vec()
    }

    /// Get the last assistant message from the conversation
    pub fn get_last_assistant_message(&self) -> Option<&Message> {
        self.messages
            .iter()
            .rev()
            .find(|m| m.role == "assistant")
    }

    /// Get all messages since a given index
    pub fn get_messages_since(&self, index: usize) -> &[Message] {
        if index < self.messages.len() {
            &self.messages[index..]
        } else {
            &[]
        }
    }

    /// Get the current message count
    pub fn message_count(&self) -> usize {
        self.messages.len()
    }

    pub fn generate_codebase_summary(project_root: &Path, max_depth: usize) -> String {
        let mut summary = String::new();
        summary.push_str("Project structure:\n");
        let mut file_count = 0;
        let mut dir_count = 0;

        if std::env::var("DEBUG_API").is_ok() {
            use colored::*;
            eprintln!(
                "{}: Scanning project at {} with max_depth {}",
                "DEBUG".blue().bold(),
                project_root.display().to_string().cyan(),
                max_depth.to_string().yellow()
            );
        }

        for e in WalkDir::new(project_root)
            .max_depth(max_depth)
            .into_iter()
            .flatten()
        {
            if e.file_type().is_file() {
                summary.push_str(&format!("- {}\n", e.path().display()));
                file_count += 1;
                if file_count > 200 {
                    summary.push_str("... (truncated for size)\n");
                    break;
                }
            } else if e.file_type().is_dir() {
                dir_count += 1;
            }
        }

        if std::env::var("DEBUG_API").is_ok() {
            use colored::*;
            eprintln!(
                "{}: Found {} files and {} directories",
                "DEBUG".blue().bold(),
                file_count.to_string().green(),
                dir_count.to_string().green()
            );
        }

        summary
    }

    pub async fn enhance_summary(&mut self) -> Result<()> {
        let prompt = format!("Provide a concise summary of this project structure for an AI coding agent to use as context. Highlight key files, directories, and potential main components:\n{}", self.codebase_summary);

        let temp_messages = vec![
            Message {
                role: "system".to_string(),
                content: Some("You are an expert at summarizing codebases concisely.".to_string()),
                tool_calls: None,
                tool_call_id: None,
            },
            Message {
                role: "user".to_string(),
                content: Some(prompt),
                tool_calls: None,
                tool_call_id: None,
            },
        ];

        let body = ChatCompletionRequest {
            model: self.api_client.config().model.clone(),
            messages: temp_messages,
            tools: None,
            tool_choice: "none".to_string(),
            temperature: 0.5,
            max_tokens: 1000,
            response_format: None,
        };

        let api_resp = self.api_client.chat_completion(body).await?;

        if let Some(choice) = api_resp.choices.first() {
            if let Some(content) = &choice.message.content {
                self.codebase_summary = content.clone();
                self.messages[0].content = Some(format!(
                    "You are Grok Code, a helpful coding agent. You have access to the user's project at {}. Updated Summary: {}\n\nUse the available tools to help the user. Be smart about which tools to use - for simple summaries, you might only need to read key files. For deeper analysis or when asked to find issues, you can use lint or search tools. Match your tool usage to what the user actually asked for.",
                    self.project_root.display(),
                    self.codebase_summary
                ));
                return Ok(());
            }
        }
        Err(GrokError::NoSummaryGenerated)
    }

    fn get_api_tools(&self) -> Vec<Tool> {
        self.tool_registry
            .get_tools()
            .iter()
            .map(|tool| Tool {
                r#type: "function".to_string(),
                function: Function {
                    name: tool.name().to_string(),
                    description: tool.description().to_string(),
                    parameters: tool.parameters(),
                },
            })
            .collect()
    }

    async fn call_api(&self, use_structured: bool) -> Result<ChatCompletionResponse> {
        let mut body = ChatCompletionRequest {
            model: self.api_client.config().model.clone(),
            messages: self.messages.clone(),
            tools: Some(self.get_api_tools()),
            tool_choice: "auto".to_string(),
            temperature: self.temperature,
            max_tokens: self.max_tokens,
            response_format: None,
        };

        if use_structured {
            body.response_format = Some(ResponseFormat {
                r#type: "json_object".to_string(),
            });
        }

        self.api_client.chat_completion(body).await
    }

    async fn make_api_call_with_progress(
        &mut self,
        use_structured: bool,
        cache_key: &Option<String>,
    ) -> Result<ChatCompletionResponse> {
        use std::io::{self, Write};

        // Only show progress if not in TUI mode
        let (tx, mut rx) = tokio::sync::oneshot::channel();
        let tui_sender = self.tui_sender.clone();
        let progress_task = tokio::spawn(async move {
            let mut elapsed_shown = 0;
            loop {
                tokio::select! {
                    _ = tokio::time::sleep(tokio::time::Duration::from_secs(15)) => {
                        elapsed_shown += 15;
                        if let Some(sender) = &tui_sender {
                            let _ = sender.send(TuiUpdate::Processing {
                                message: format!(" ({}s)", elapsed_shown),
                            });
                        } else {
                            print!(" ({elapsed_shown}s)");
                            io::stdout().flush().unwrap();
                        }
                    }
                    _ = &mut rx => {
                        break;
                    }
                }
            }
        });

        let api_result = self.call_api(use_structured).await;
        let _ = tx.send(()); // Stop the progress task
        let _ = progress_task.await; // Clean up the task

        // Cache successful response
        if let Ok(ref response) = api_result {
            if let Some(key) = cache_key {
                if let Ok(serialized) = serde_json::to_string(response) {
                    self.response_cache.put(key.clone(), serialized);
                }
            }
        }

        api_result
    }

    pub async fn process_prompt(&mut self, user_message: &str, interactive: bool) {

        self.messages.push(Message {
            role: "user".to_string(),
            content: Some(user_message.to_string()),
            tool_calls: None,
            tool_call_id: None,
        });

        // Send user message to TUI
        if self.tui_sender.is_some() {
            self.send_update(TuiUpdate::Message(self.messages.last().unwrap().clone()));
        }

        let mut iterations = 0;
        let max_iterations = 15;
        let mut timeout_retries = 0;
        let max_timeout_retries = 3;
        let mut rate_limit_retries = 0;
        let max_rate_limit_retries = 5; // More retries since we handle silently
                                        // TODO: Make iteration and retry limits configurable
                                        // TODO: Add exponential backoff with jitter for retries

        // Track tool results for cache key generation
        let mut tool_results: Vec<String> = Vec::new();
        let enable_cache =
            std::env::var("GROK_CACHE").unwrap_or_else(|_| "true".to_string()) == "true";

        loop {
            if iterations >= max_iterations {
                self.send_update(TuiUpdate::Processing {
                    message: "Max iterations reached. Stopping.".to_string(),
                });
                break;
            }
            iterations += 1;

            // Show progress indicator
            if iterations == 1 {
                self.send_update(TuiUpdate::Processing {
                    message: "🤔 Thinking".to_string(),
                });
            } else if timeout_retries == 0 && rate_limit_retries == 0 {
                self.send_update(TuiUpdate::Processing {
                    message: ".".to_string(),
                });
            }

            let use_structured = iterations == max_iterations;

            // Check cache before making API call
            let cache_key = if enable_cache && !tool_results.is_empty() {
                Some(ResponseCache::generate_key(user_message, &tool_results))
            } else {
                None
            };

            // Try to get cached response
            let api_result = if let Some(ref key) = cache_key {
                if let Some(cached_response) = self.response_cache.get(key) {
                                    // Clear thinking indicator for cached response
                if iterations == 1 {
                    self.send_update(TuiUpdate::Processing {
                        message: "\n💡 Using cached response".to_string(),
                    });
                }

                    // Deserialize cached response
                    match serde_json::from_str::<ChatCompletionResponse>(&cached_response) {
                        Ok(resp) => Ok(resp),
                        Err(_) => {
                            // Cache hit but deserialization failed, make API call
                            self.make_api_call_with_progress(use_structured, &cache_key)
                                .await
                        }
                    }
                } else {
                    // No cache hit, make API call
                    self.make_api_call_with_progress(use_structured, &cache_key)
                        .await
                }
            } else {
                // Caching disabled or first iteration, make API call
                self.make_api_call_with_progress(use_structured, &cache_key)
                    .await
            };

            let api_response = match api_result {
                Ok(resp) => resp,
                Err(e) => {
                                    let error_msg = e.to_string();
                if error_msg.contains("403") && error_msg.contains("credits") {
                    let error_text = format!(
                        "\n❌ API Credit Error Detected!\n\n\
                        It looks like your credits haven't activated yet. This is common with xAI.\n\n\
                        Quick solutions:\n\
                        1. Wait 5-15 minutes for credits to activate\n\
                        2. Visit your team billing page to check status:\n\
                           https://console.x.ai/team/[your-team-id]\n\
                        3. Try regenerating your API key after credits show as available\n\
                        4. Use OpenAI instead: grok-code --dev (requires OPENAI_API_KEY)\n\n\
                        For detailed troubleshooting, see: ./TROUBLESHOOTING_XAI.md"
                    );
                    self.send_update(TuiUpdate::Error { message: error_text });
                    self.send_update(TuiUpdate::Complete);
                    return;
                    } else if error_msg.contains("Rate limit exceeded") {
                        rate_limit_retries += 1;
                        if rate_limit_retries > max_rate_limit_retries {
                            // After exhausting retries, just continue to let AI try again or give up gracefully
                            // Don't show error to user - this is our problem to handle
                            continue;
                        } else {
                            // Exponential backoff: 5s, 10s, 20s, 40s
                            let wait_seconds = 5 * (1 << (rate_limit_retries - 1));
                            // Silently wait without showing anything to user
                            tokio::time::sleep(tokio::time::Duration::from_secs(wait_seconds))
                                .await;
                            // Don't increment iterations for rate limit retries
                            iterations -= 1;
                            continue;
                        }
                    } else if error_msg.contains("timeout") {
                        timeout_retries += 1;
                        if timeout_retries > max_timeout_retries {
                            self.send_update(TuiUpdate::Error {
                                message: "\n⏱️  The request is taking longer than expected. This sometimes happens with complex requests.\n\
                                         Please try again with a simpler request or use --dev mode for faster responses.".to_string(),
                            });
                            self.send_update(TuiUpdate::Complete);
                            return;
                        } else {
                            // Silently retry without bothering the user
                            iterations -= 1;
                            continue;
                        }
                    } else if error_msg.contains("502")
                        || error_msg.contains("503")
                        || error_msg.contains("504")
                    {
                        // Server errors - silently retry a few times
                        if iterations < 5 {
                            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                            iterations -= 1;
                            continue;
                        } else {
                            self.send_update(TuiUpdate::Error {
                                message: "\n💬 I'm having trouble connecting right now. Please try again in a moment.".to_string(),
                            });
                            self.send_update(TuiUpdate::Complete);
                            return;
                        }
                    } else {
                        let mut error_text = "\n❌ Something went wrong. Please try again.".to_string();
                        if std::env::var("DEBUG_API").is_ok() {
                            error_text.push_str(&format!("\nDEBUG: {error_msg}"));
                        }
                        self.send_update(TuiUpdate::Error { message: error_text });
                        self.send_update(TuiUpdate::Complete);
                        return;
                    }
                }
            };

            if api_response.choices.is_empty() {
                let mut error_text = "❌ Empty response from API (no choices).".to_string();
                if std::env::var("DEBUG_API").is_ok() {
                    error_text.push_str("\nDEBUG: Empty API response received");
                }
                self.send_update(TuiUpdate::Error { message: error_text });
                break;
            }

            let message = api_response.choices[0].message.clone();

            // Reset retries on successful response
            timeout_retries = 0;
            rate_limit_retries = 0;

            if let Some(content) = &message.content {
                if !content.is_empty() && content.trim() != "" {
                    self.messages.push(message.clone());
                    self.send_update(TuiUpdate::Message(message.clone()));
                    if !interactive {
                        break;
                    }
                } else if message.tool_calls.is_none() {
                    // Empty response with no tools - continue to let AI think more
                    self.messages.push(message.clone());
                    continue;
                }
            }

            if let Some(tool_calls) = message.tool_calls.as_ref() {
                self.messages.push(message.clone());

                // Execute tools concurrently when possible
                let num_tools = tool_calls.len();
                if num_tools > 1 {
                    self.send_update(TuiUpdate::Processing {
                        message: format!("🔄 Executing {num_tools} tools concurrently..."),
                    });
                }

                // Prepare tool information
                let mut tool_infos = Vec::new();
                for (idx, tool_call) in tool_calls.iter().enumerate() {
                    let tool_name = tool_call.function.name.clone();
                    let tool_args = tool_call.function.arguments.clone();
                    let tool_id = tool_call.id.clone();

                    // Show what tool is being executed
                    let icon = match tool_name.as_str() {
                        "read_file" => "📖",
                        "write_file" => "✏️",
                        "edit_file" => "📝",
                        "list_files" => "📁",
                        "run_shell_command" => "🖥️",
                        "search_codebase" => "🔍",
                        "run_lint" => "🔧",
                        "debug_code" => "🐛",
                        _ => "⚙️",
                    };

                    let action_text = match tool_name.as_str() {
                        "read_file" => "Reading file",
                        "write_file" => "Writing file",
                        "edit_file" => "Editing file",
                        "list_files" => "Listing files",
                        "run_shell_command" => "Running command",
                        "search_codebase" => "Searching codebase",
                        "run_lint" => "Running linter",
                        "debug_code" => "Debugging code",
                        _ => &tool_name,
                    };

                    self.send_update(TuiUpdate::ToolStart {
                        name: action_text.to_string(),
                        icon: icon.to_string(),
                    });

                    tool_infos.push((idx, tool_id, tool_name, tool_args));
                }

                // Execute tools (can be parallelized for tools that don't need git repo)
                let mut results = Vec::new();

                // Separate tools that can run in parallel (non-git tools)
                let mut parallel_tools = Vec::new();
                let mut sequential_tools = Vec::new();

                for (idx, tool_id, tool_name, tool_args) in tool_infos {
                    // Git operations must be sequential
                    if matches!(
                        tool_name.as_str(),
                        "create_commit" | "submit_pr" | "resolve_merge_conflict"
                    ) {
                        sequential_tools.push((idx, tool_id, tool_name, tool_args));
                    } else {
                        parallel_tools.push((idx, tool_id, tool_name, tool_args));
                    }
                }

                // Execute parallel tools
                if !parallel_tools.is_empty() {
                    let project_root = self.project_root.clone();
                    let dry_run = self.dry_run;
                    let no_confirm = self.no_confirm;
                    let registry = self.tool_registry.clone();
                    let tui_mode = self.tui_sender.is_some();

                    let mut tasks = Vec::new();
                    for (idx, tool_id, tool_name, tool_args) in parallel_tools {
                        let project_root = project_root.clone();
                        let registry = registry.clone();

                        let task = tokio::spawn(async move {
                            let context = ToolContext {
                                project_root,
                                dry_run,
                                no_confirm,
                                git_repo: None, // Non-git tools don't need repo
                                tui_mode,
                            };
                            let result = registry.execute_tool(&tool_name, &tool_args, &context);
                            (idx, tool_id, tool_name, result)
                        });
                        tasks.push(task);
                        // TODO: Add timeout for individual tool execution
                        // TODO: Implement cancellation tokens for long-running tools
                    }

                    // Wait for parallel tasks
                    for task in tasks {
                        match task.await {
                            Ok(result) => results.push(result),
                            Err(e) => {
                                self.send_update(TuiUpdate::Error {
                                    message: format!("Tool execution failed: {}", e),
                                });
                                results.push((
                                    0,
                                    String::new(),
                                    String::new(),
                                    format!("Error executing tool: {e}"),
                                ));
                            }
                        }
                    }
                }

                // Execute sequential tools
                for (idx, tool_id, tool_name, tool_args) in sequential_tools {
                    let context = ToolContext {
                        project_root: self.project_root.clone(),
                        dry_run: self.dry_run,
                        no_confirm: self.no_confirm,
                        git_repo: self.git_repo.as_ref(),
                        tui_mode: self.tui_sender.is_some(),
                    };
                    let result = self
                        .tool_registry
                        .execute_tool(&tool_name, &tool_args, &context);
                    results.push((idx, tool_id, tool_name, result));
                }

                // Sort results by original index to maintain order
                results.sort_by_key(|(idx, _, _, _)| *idx);

                // Process results
                for (_idx, tool_id, tool_name, tool_result) in results {
                    // Format tool result for display
                    let display_result = match tool_name.as_str() {
                        "list_files" => {
                            let lines: Vec<&str> = tool_result.lines().collect();
                            if lines.len() > 10 {
                                format!(
                                    "✓ Found {} files (showing first 10):\n{}",
                                    lines.len(),
                                    lines.iter().take(10).map(|l| format!("  - {}", l)).collect::<Vec<_>>().join("\n")
                                    + &format!("\n  ... and {} more", lines.len() - 10)
                                )
                            } else {
                                format!(
                                    "✓ Found {} files:\n{}",
                                    lines.len(),
                                    lines.iter().map(|l| format!("  - {}", l)).collect::<Vec<_>>().join("\n")
                                )
                            }
                        }
                        "read_file" | "search_codebase" => {
                            let lines = tool_result.lines().count();
                            if lines > 5 {
                                format!("✓ Success ({} lines of output)", lines)
                            } else {
                                format!("✓ {}", tool_result)
                            }
                        }
                        "run_shell_command" | "run_lint" => {
                            if tool_result.contains("error") || tool_result.contains("Error") {
                                "✓ Done".to_string()
                            } else {
                                "✓ Command completed successfully".to_string()
                            }
                        }
                        _ => {
                            if tool_result.len() > 200 {
                                format!("✓ Done (output: {} chars)", tool_result.len())
                            } else {
                                format!("✓ {}", tool_result)
                            }
                        }
                    };

                    self.send_update(TuiUpdate::ToolResult {
                        name: tool_name.clone(),
                        result: display_result,
                    });

                    // Add tool result to cache key generation
                    tool_results.push(tool_result.clone());

                    self.messages.push(Message {
                        role: "tool".to_string(),
                        content: Some(tool_result),
                        tool_calls: None,
                        tool_call_id: Some(tool_id),
                    });
                }
            } else {
                self.send_update(TuiUpdate::Processing {
                    message: "No content or tool calls in response.".to_string(),
                });
                break;
            }
        }

        // Send completion signal
        self.send_update(TuiUpdate::Complete);

        // Show cache statistics if in debug mode
        if std::env::var("DEBUG_CACHE").is_ok() && enable_cache {
            let stats = self.response_cache.stats();
            self.send_update(TuiUpdate::Processing {
                message: format!(
                    "📊 Cache stats: {} active, {} expired, {} total",
                    stats.active_entries, stats.expired_entries, stats.total_entries
                ),
            });
        }
    }
}
