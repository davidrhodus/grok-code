use crate::api::{
    ApiClient, ApiConfig, ChatCompletionRequest, ChatCompletionResponse, Function, Message,
    ResponseFormat, Tool,
};
use crate::cache::ResponseCache;
use crate::error::{GrokError, Result};
use crate::tools::{ToolContext, ToolRegistry};
use git2::Repository;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

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
}

impl GrokAgent {
    pub fn new(
        provider: &str,
        api_config: ApiConfig,
        project_root: PathBuf,
        dry_run: bool,
        max_depth: usize,
        no_confirm: bool,
    ) -> Result<Self> {
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
            no_confirm,
            git_repo,
            response_cache: ResponseCache::new(100, 300), // 100 entries, 5 minute TTL
                                                          // TODO: Make cache size and TTL configurable via environment variables
        })
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

        // Spawn a task to show elapsed time during long waits
        let (tx, mut rx) = tokio::sync::oneshot::channel();
        let progress_task = tokio::spawn(async move {
            let mut elapsed_shown = 0;
            loop {
                tokio::select! {
                    _ = tokio::time::sleep(tokio::time::Duration::from_secs(15)) => {
                        elapsed_shown += 15;
                        print!(" ({}s)", elapsed_shown);
                        io::stdout().flush().unwrap();
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
        use std::time::Instant;

        self.messages.push(Message {
            role: "user".to_string(),
            content: Some(user_message.to_string()),
            tool_calls: None,
            tool_call_id: None,
        });

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
                println!("Max iterations reached. Stopping.");
                break;
            }
            iterations += 1;

            // Track time for long operations
            let _start_time = Instant::now();
            // Show progress indicator
            if iterations == 1 {
                print!("🤔 Thinking");
                io::stdout().flush().unwrap();
            } else if timeout_retries > 0 || rate_limit_retries > 0 {
                // Don't show anything for retries - handle silently
            } else {
                print!(".");
                io::stdout().flush().unwrap();
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
                        println!(); // New line after "Thinking"
                        println!("💡 Using cached response");
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
                        println!(); // Clear the thinking dots
                        eprintln!("\n❌ API Credit Error Detected!\n");
                        eprintln!("It looks like your credits haven't activated yet. This is common with xAI.\n");
                        eprintln!("Quick solutions:");
                        eprintln!("1. Wait 5-15 minutes for credits to activate");
                        eprintln!("2. Visit your team billing page to check status:");
                        eprintln!("   https://console.x.ai/team/[your-team-id]");
                        eprintln!(
                            "3. Try regenerating your API key after credits show as available"
                        );
                        eprintln!(
                            "4. Use OpenAI instead: grok-code --dev (requires OPENAI_API_KEY)"
                        );
                        eprintln!("\nFor detailed troubleshooting, see: ./TROUBLESHOOTING_XAI.md");
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
                            println!(); // Clear the thinking dots
                            eprintln!("\n⏱️  The request is taking longer than expected. This sometimes happens with complex requests.");
                            eprintln!("Please try again with a simpler request or use --dev mode for faster responses.");
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
                            println!(); // Clear the thinking dots
                            println!("\n💬 I'm having trouble connecting right now. Please try again in a moment.");
                            return;
                        }
                    } else {
                        println!(); // Clear the thinking dots
                        eprintln!("\n❌ Something went wrong. Please try again.");
                        if std::env::var("DEBUG_API").is_ok() {
                            eprintln!("DEBUG: {}", error_msg);
                        }
                        return;
                    }
                }
            };

            if api_response.choices.is_empty() {
                println!(); // Clear line
                println!("❌ Empty response from API (no choices).");
                if std::env::var("DEBUG_API").is_ok() {
                    eprintln!("DEBUG: Empty API response received");
                }
                break;
            }

            let message = api_response.choices[0].message.clone();

            // Clear the thinking indicator
            if iterations == 1 || iterations % 5 == 0 {
                println!(); // New line after dots
            }

            // Reset retries on successful response
            timeout_retries = 0;
            rate_limit_retries = 0;

            if let Some(content) = &message.content {
                if !content.is_empty() && content.trim() != "" {
                    println!("💬 {}", content);
                    self.messages.push(message.clone());
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
                    println!("🔄 Executing {} tools concurrently...", num_tools);
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

                    if num_tools == 1 {
                        let action_text = match tool_name.as_str() {
                            "read_file" => "Reading file",
                            "write_file" => "Writing file",
                            "edit_file" => "Editing file",
                            "list_files" => "Listing files",
                            "run_shell_command" => "Running command",
                            "search_codebase" => "Searching codebase",
                            "run_lint" => "Running linter",
                            "debug_code" => "Debugging code",
                            _ => "Executing tool",
                        };
                        println!("{} {}...", icon, action_text);
                    } else {
                        println!("  [{}] {} {}", idx + 1, icon, tool_name);
                    }

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
                                use colored::*;
                                eprintln!(
                                    "{} Tool execution failed: {}",
                                    "❌".red(),
                                    e.to_string().red()
                                );
                                results.push((
                                    0,
                                    String::new(),
                                    String::new(),
                                    format!("Error executing tool: {}", e),
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
                    };
                    let result = self
                        .tool_registry
                        .execute_tool(&tool_name, &tool_args, &context);
                    results.push((idx, tool_id, tool_name, result));
                }

                // Sort results by original index to maintain order
                results.sort_by_key(|(idx, _, _, _)| *idx);

                // Process results
                for (idx, tool_id, tool_name, tool_result) in results {
                    // Show abbreviated results for common tools
                    let prefix = if num_tools > 1 {
                        format!("  [{}] ", idx + 1)
                    } else {
                        "   ".to_string()
                    };

                    match tool_name.as_str() {
                        "list_files" => {
                            let lines: Vec<&str> = tool_result.lines().collect();
                            if lines.len() > 10 {
                                println!(
                                    "{}✓ Found {} files (showing first 10):",
                                    prefix,
                                    lines.len()
                                );
                                for line in lines.iter().take(10) {
                                    println!("{}  - {}", prefix, line);
                                }
                                println!("{}  ... and {} more", prefix, lines.len() - 10);
                            } else {
                                println!("{}✓ Found {} files:", prefix, lines.len());
                                for line in &lines {
                                    println!("{}  - {}", prefix, line);
                                }
                            }
                        }
                        "read_file" | "search_codebase" => {
                            let lines = tool_result.lines().count();
                            if lines > 5 {
                                println!("{}✓ Success ({} lines of output)", prefix, lines);
                            } else {
                                println!("{}✓ {}", prefix, tool_result);
                            }
                        }
                        "run_shell_command" | "run_lint" => {
                            if tool_result.contains("error") || tool_result.contains("Error") {
                                // Don't alarm user about internal errors
                                println!("{}✓ Done", prefix);
                            } else {
                                println!("{}✓ Command completed successfully", prefix);
                            }
                        }
                        _ => {
                            if tool_result.len() > 200 {
                                println!("{}✓ Done (output: {} chars)", prefix, tool_result.len());
                            } else {
                                println!("{}✓ {}", prefix, tool_result);
                            }
                        }
                    }

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
                println!(); // Clear line
                println!("No content or tool calls in response.");
                break;
            }
        }

        // Show cache statistics if in debug mode
        if std::env::var("DEBUG_CACHE").is_ok() && enable_cache {
            let stats = self.response_cache.stats();
            println!(
                "📊 Cache stats: {} active, {} expired, {} total",
                stats.active_entries, stats.expired_entries, stats.total_entries
            );
        }
    }
}
