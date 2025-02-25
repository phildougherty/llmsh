mod command_processor;
mod job_control;
mod suggestions;
mod documentation;
mod shell_env;
mod alias;
mod signal_handler;
mod command_parser;
mod executor;

use std::io::Write;
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use colored::*;
use anyhow::{Result, Context};
use crate::llm::LLMClient;
use crate::terminal::Terminal;
use crate::llm::context_manager::ContextManager;
use crate::shell::suggestions::SuggestionEngine;
use crate::shell::documentation::Documentation;
use crate::utils::performance::PERFORMANCE_MONITOR;
use log::debug;

pub struct Shell {
    terminal: Terminal,
    command_processor: command_processor::CommandProcessor,
    job_control: job_control::JobControl,
    llm_client: LLMClient,
    working_dir: PathBuf,
    suggestion_engine: SuggestionEngine,
    documentation: Documentation,
    context_manager: ContextManager,
    environment: shell_env::Environment,
    alias_manager: alias::AliasManager,
}

impl Shell {
    pub fn new() -> Self {
        let llm_client = LLMClient::new();
        
        // Initialize signal handler
        signal_handler::SignalHandler::initialize().unwrap_or_else(|e| {
            eprintln!("Warning: Failed to initialize signal handlers: {}", e);
        });
        
        // Determine if this is a login shell
        let is_login_shell = std::env::args()
            .next()
            .map(|arg| arg.starts_with('-'))
            .unwrap_or(false);
            
        // Create environment manager
        let mut environment = shell_env::Environment::new(is_login_shell);
        environment.initialize().unwrap_or_else(|e| {
            eprintln!("Warning: Failed to initialize environment: {}", e);
        });
        
        // Create alias manager
        let mut alias_manager = alias::AliasManager::new();
        alias_manager.initialize().unwrap_or_else(|e| {
            eprintln!("Warning: Failed to initialize aliases: {}", e);
        });
        
        Shell {
            terminal: Terminal::new(),
            command_processor: command_processor::CommandProcessor::new(),
            job_control: job_control::JobControl::new(),
            suggestion_engine: SuggestionEngine::new(),
            documentation: Documentation::new(llm_client.clone()),
            context_manager: ContextManager::new(),
            llm_client,
            working_dir: std::env::current_dir().unwrap_or_default(),
            environment,
            alias_manager,
        }
    }

    fn expand_env_vars(&self, value: &str) -> String {
        let mut result = value.to_string();
        let mut i = 0;
        
        while i < result.len() {
            if result[i..].starts_with('$') {
                let var_start = i;
                i += 1; // Skip the $
                
                // Handle ${VAR} format
                if i < result.len() && result[i..].starts_with('{') {
                    i += 1; // Skip the {
                    let var_name_start = i;
                    
                    // Find closing brace
                    while i < result.len() && !result[i..].starts_with('}') {
                        i += 1;
                    }
                    
                    if i < result.len() {
                        let var_name = &result[var_name_start..i];
                        i += 1; // Skip the }
                        
                        if let Ok(value) = std::env::var(var_name) {
                            result.replace_range(var_start..i, &value);
                            i = var_start + value.len();
                        }
                    }
                } 
                // Handle $VAR format
                else {
                    let var_name_start = i;
                    
                    // Find end of variable name (alphanumeric or _)
                    while i < result.len() && (result[i..].chars().next().unwrap().is_alphanumeric() || result[i..].starts_with('_')) {
                        i += 1;
                    }
                    
                    if i > var_name_start {
                        let var_name = &result[var_name_start..i];
                        
                        if let Ok(value) = std::env::var(var_name) {
                            result.replace_range(var_start..i, &value);
                            i = var_start + value.len();
                        }
                    }
                }
            } else {
                i += 1;
            }
        }
        
        result
    }
    
    pub async fn run(&mut self) -> Result<()> {
        self.initialize()?;
        
        loop {
            let (input, show_suggestions) = self.terminal.read_line()?;
            let input = input.trim();
            
            // Check for interrupt
            if signal_handler::SignalHandler::was_interrupted() {
                continue;
            }
            
            if input.is_empty() {
                continue;
            }
            
            if input == "exit" {
                break;
            }

            // Handle built-in commands
            if let Some(result) = self.handle_builtin_command(input) {
                match result {
                    Ok(should_exit) => {
                        if should_exit {
                            break;
                        }
                        continue;
                    }
                    Err(e) => {
                        eprintln!("Error: {}", e);
                        continue;
                    }
                }
            }

            // Handle suggestions
            if show_suggestions {
                let command_prefix = input.split_whitespace().next();
                if let Ok(suggestions) = self.show_suggestions(command_prefix).await {
                    println!("{}", suggestions);
                    continue;
                }
            }

            // Expand aliases
            let expanded_input = self.alias_manager.expand(input);

            // Update context
            self.context_manager.update_directory(&self.working_dir.to_string_lossy());
            self.context_manager.add_command(&expanded_input);
            
            let start_time = std::time::Instant::now();
            
            // Process the input
            self.process_input(&expanded_input).await?;
            
            // Record execution time
            let duration = start_time.elapsed();
            PERFORMANCE_MONITOR.lock().unwrap().record_execution(&expanded_input, duration);
            
            // Update working directory
            if let Ok(dir) = std::env::current_dir() {
                self.working_dir = dir;
            }
            
            // Clean up any completed background jobs
            self.job_control.cleanup_completed_jobs();
        }

        Ok(())
    }

    fn handle_builtin_command(&mut self, input: &str) -> Option<Result<bool>> {
        let parts: Vec<&str> = input.split_whitespace().collect();
        if parts.is_empty() {
            return None;
        }
    
        match parts[0] {
            // Directory navigation
            "cd" => {
                let dir_to_use = if parts.len() > 1 {
                    parts[1].to_string()
                } else {
                    // Default to home directory
                    dirs::home_dir()
                        .and_then(|p| p.to_str().map(|s| s.to_string()))
                        .unwrap_or_else(|| ".".to_string())
                };
                
                // Handle ~ expansion
                let expanded_dir = if dir_to_use.starts_with('~') {
                    if let Some(home) = dirs::home_dir() {
                        if dir_to_use.len() == 1 {
                            home.to_string_lossy().to_string()
                        } else {
                            home.join(&dir_to_use[2..]).to_string_lossy().to_string()
                        }
                    } else {
                        dir_to_use
                    }
                } else {
                    dir_to_use
                };
                
                match std::env::set_current_dir(&expanded_dir) {
                    Ok(_) => {
                        if let Ok(new_dir) = std::env::current_dir() {
                            self.working_dir = new_dir;
                            self.context_manager.update_directory(&self.working_dir.to_string_lossy());
                        }
                        Some(Ok(false))
                    }
                    Err(e) => Some(Err(anyhow::anyhow!("cd: {}: {}", expanded_dir, e))),
                }
            },
            
            "pwd" => {
                println!("{}", self.working_dir.display());
                Some(Ok(false))
            },
            
            // Environment variables
            "export" => {
                if parts.len() == 1 {
                    // Just 'export' - list all environment variables
                    for (key, value) in std::env::vars() {
                        println!("{}={}", key, value);
                    }
                } else {
                    // Handle export VAR=VALUE
                    let export_str = input["export ".len()..].trim();
                    if let Some(equals_pos) = export_str.find('=') {
                        let name = export_str[..equals_pos].trim();
                        let value = export_str[equals_pos + 1..].trim();
                        
                        // Remove quotes if present
                        let clean_value = value.trim_matches('"').trim_matches('\'');
                        
                        // Expand variables in the value
                        let expanded_value = self.expand_env_vars(clean_value);
                        
                        // Set the environment variable
                        std::env::set_var(name, expanded_value);
                    } else {
                        eprintln!("Invalid export format. Use: export VAR=VALUE");
                    }
                }
                Some(Ok(false))
            },
            
            "unset" => {
                if parts.len() > 1 {
                    for var in &parts[1..] {
                        std::env::remove_var(var);
                    }
                } else {
                    eprintln!("unset: missing variable name");
                }
                Some(Ok(false))
            },
            
            "set" => {
                if parts.len() == 1 {
                    // Just 'set' - list all environment variables
                    for (key, value) in std::env::vars() {
                        println!("{}={}", key, value);
                    }
                } else {
                    // Handle shell options (simplified)
                    // In a real shell, this would handle options like -e, -x, etc.
                    eprintln!("Note: shell options not fully implemented");
                }
                Some(Ok(false))
            },
            
            // Output and redirection
            "echo" => {
                if parts.len() > 1 {
                    // Check for -n option (no newline)
                    let no_newline = parts[1] == "-n";
                    let start_idx = if no_newline { 2 } else { 1 };
                    
                    // Join all arguments and expand variables
                    let echo_str = parts[start_idx..].join(" ");
                    let expanded = self.expand_env_vars(&echo_str);
                    
                    if no_newline {
                        print!("{}", expanded);
                        std::io::stdout().flush().unwrap_or(());
                    } else {
                        println!("{}", expanded);
                    }
                } else {
                    // Just echo a newline
                    println!();
                }
                Some(Ok(false))
            },
            
            "printf" => {
                if parts.len() > 1 {
                    // Very simplified printf implementation
                    let format_str = self.expand_env_vars(parts[1]);
                    let args: Vec<String> = parts[2..].iter()
                        .map(|arg| self.expand_env_vars(arg))
                        .collect();
                    
                    // Basic % substitution (simplified)
                    let mut result = format_str.clone();
                    for arg in args {
                        if let Some(pos) = result.find('%') {
                            let end = pos + 2.min(result.len() - pos);
                            result.replace_range(pos..end, &arg);
                        }
                    }
                    
                    print!("{}", result);
                    std::io::stdout().flush().unwrap_or(());
                } else {
                    eprintln!("printf: missing format string");
                }
                Some(Ok(false))
            },
            
            // Job control
            "jobs" => {
                match self.job_control.list_jobs() {
                    Ok(_) => {},
                    Err(e) => eprintln!("Error listing jobs: {}", e),
                }
                Some(Ok(false))
            },
            
            "fg" => {
                let args = parts.iter().map(|s| s.to_string()).collect::<Vec<_>>();
                match self.job_control.bring_to_foreground(&args) {
                    Ok(_) => {},
                    Err(e) => eprintln!("Error bringing job to foreground: {}", e),
                }
                Some(Ok(false))
            },
            
            "bg" => {
                let args = parts.iter().map(|s| s.to_string()).collect::<Vec<_>>();
                match self.job_control.continue_in_background(&args) {
                    Ok(_) => {},
                    Err(e) => eprintln!("Error continuing job in background: {}", e),
                }
                Some(Ok(false))
            },
            
            "kill" => {
                if parts.len() < 2 {
                    eprintln!("kill: usage: kill [-s sigspec | -n signum | -sigspec] pid | jobspec ... or kill -l [sigspec]");
                    return Some(Ok(false));
                }
                
                // Handle -l option to list signals
                if parts[1] == "-l" {
                    println!("HUP INT QUIT ILL TRAP ABRT BUS FPE KILL USR1 SEGV USR2 PIPE ALRM TERM STKFLT CHLD CONT STOP TSTP TTIN TTOU URG XCPU XFSZ VTALRM PROF WINCH POLL PWR SYS");
                    return Some(Ok(false));
                }
                
                // Parse signal if provided
                let mut signal = 15; // Default to SIGTERM
                let mut arg_start = 1;
                
                if parts[1].starts_with('-') {
                    if let Ok(sig) = parts[1][1..].parse::<i32>() {
                        signal = sig;
                        arg_start = 2;
                    } else if parts[1] == "-KILL" || parts[1] == "-9" {
                        signal = 9;
                        arg_start = 2;
                    } else if parts[1] == "-HUP" || parts[1] == "-1" {
                        signal = 1;
                        arg_start = 2;
                    } else if parts[1] == "-INT" || parts[1] == "-2" {
                        signal = 2;
                        arg_start = 2;
                    } else if parts[1] == "-TERM" || parts[1] == "-15" {
                        signal = 15;
                        arg_start = 2;
                    }
                }
                
                // Send signal to each PID
                for pid_str in &parts[arg_start..] {
                    if let Ok(pid) = pid_str.parse::<i32>() {
                        unsafe {
                            if libc::kill(pid, signal) != 0 {
                                eprintln!("kill: ({}) - No such process", pid);
                            }
                        }
                    } else {
                        eprintln!("kill: ({}) - Invalid process id", pid_str);
                    }
                }
                
                Some(Ok(false))
            },
            
            "wait" => {
                if parts.len() > 1 {
                    for pid_str in &parts[1..] {
                        if let Ok(pid) = pid_str.parse::<i32>() {
                            unsafe {
                                let mut status = 0;
                                libc::waitpid(pid, &mut status, 0);
                            }
                        } else {
                            eprintln!("wait: {}: invalid process id", pid_str);
                        }
                    }
                } else {
                    // Wait for all children
                    unsafe {
                        libc::wait(std::ptr::null_mut());
                    }
                }
                Some(Ok(false))
            },
            
            // Aliases
            "alias" => {
                if parts.len() == 1 {
                    // List all aliases
                    for (name, value) in self.alias_manager.list_aliases() {
                        println!("alias {}='{}'", name, value);
                    }
                } else if parts.len() == 2 && !parts[1].contains('=') {
                    // Show specific alias
                    let aliases = self.alias_manager.list_aliases();
                    let name = parts[1];
                    let found = aliases.iter().find(|(n, _)| n == name);
                    if let Some((_, value)) = found {
                        println!("alias {}='{}'", name, value);
                    } else {
                        println!("alias: {} not found", name);
                    }
                } else {
                    // Define new alias
                    let alias_def = input["alias ".len()..].trim();
                    if let Some(equals_pos) = alias_def.find('=') {
                        let name = alias_def[..equals_pos].trim();
                        let mut value = alias_def[equals_pos + 1..].trim();
                        // Remove surrounding quotes if present
                        if (value.starts_with('\'') && value.ends_with('\'')) || 
                           (value.starts_with('"') && value.ends_with('"')) {
                            value = &value[1..value.len() - 1];
                        }
                        match self.alias_manager.add_alias(name, value) {
                            Ok(_) => {},
                            Err(e) => eprintln!("Error adding alias: {}", e),
                        }
                    } else {
                        eprintln!("Invalid alias format. Use: alias name='value'");
                    }
                }
                Some(Ok(false))
            },
            
            "unalias" => {
                if parts.len() > 1 {
                    for name in &parts[1..] {
                        match self.alias_manager.remove_alias(name) {
                            Ok(_) => {},
                            Err(e) => eprintln!("Error removing alias {}: {}", name, e),
                        }
                    }
                } else {
                    eprintln!("unalias: missing alias name");
                }
                Some(Ok(false))
            },
            
            // History
            "history" => {
                let entries = self.terminal.get_history().get_entries();
                let count = if parts.len() > 1 {
                    parts[1].parse::<usize>().unwrap_or(entries.len())
                } else {
                    entries.len()
                };
                
                for (i, entry) in entries.iter().rev().take(count).rev().enumerate() {
                    println!("{:5} {}", entries.len() - count + i + 1, entry);
                }
                Some(Ok(false))
            },
            
            // File operations
            "touch" => {
                if parts.len() > 1 {
                    for file in &parts[1..] {
                        let path = std::path::Path::new(file);
                        if !path.exists() {
                            if let Err(e) = std::fs::File::create(path) {
                                eprintln!("touch: cannot touch '{}': {}", file, e);
                            }
                        } else {
                            // Update file times (simplified - just recreates the file)
                            let content = std::fs::read(path).unwrap_or_default();
                            if let Err(e) = std::fs::write(path, content) {
                                eprintln!("touch: cannot touch '{}': {}", file, e);
                            }
                        }
                    }
                } else {
                    eprintln!("touch: missing file operand");
                }
                Some(Ok(false))
            },
            
            "mkdir" => {
                if parts.len() > 1 {
                    let mut create_parents = false;
                    let mut dirs_start = 1;
                    
                    if parts[1] == "-p" {
                        create_parents = true;
                        dirs_start = 2;
                    }
                    
                    for dir in &parts[dirs_start..] {
                        let path = std::path::Path::new(dir);
                        let result = if create_parents {
                            std::fs::create_dir_all(path)
                        } else {
                            std::fs::create_dir(path)
                        };
                        
                        if let Err(e) = result {
                            eprintln!("mkdir: cannot create directory '{}': {}", dir, e);
                        }
                    }
                } else {
                    eprintln!("mkdir: missing operand");
                }
                Some(Ok(false))
            },
            
            "rmdir" => {
                if parts.len() > 1 {
                    for dir in &parts[1..] {
                        if let Err(e) = std::fs::remove_dir(dir) {
                            eprintln!("rmdir: failed to remove '{}': {}", dir, e);
                        }
                    }
                } else {
                    eprintln!("rmdir: missing operand");
                }
                Some(Ok(false))
            },
            
            // Shell control
            "exit" | "logout" | "bye" => {
                let exit_code = if parts.len() > 1 {
                    parts[1].parse::<i32>().unwrap_or(0)
                } else {
                    0
                };
                
                if exit_code != 0 {
                    eprintln!("Exit code: {}", exit_code);
                }
                
                Some(Ok(true)) // Signal to exit the shell
            },
            
            "source" | "." => {
                if parts.len() > 1 {
                    let path = std::path::Path::new(parts[1]);
                    if let Ok(content) = std::fs::read_to_string(path) {
                        for line in content.lines() {
                            let line = line.trim();
                            if line.is_empty() || line.starts_with('#') {
                                continue;
                            }
                            
                            // Process each line as a command
                            // Note: This will be handled by the caller since process_input is async
                            return Some(Err(anyhow::anyhow!("source: async operations not supported in built-ins")));
                        }
                    } else {
                        eprintln!("{}: cannot open {}: No such file or directory", parts[0], parts[1]);
                    }
                } else {
                    eprintln!("{}: filename argument required", parts[0]);
                }
                Some(Ok(false))
            },
            
            "eval" => {
                if parts.len() > 1 {
                    let cmd = parts[1..].join(" ");
                    // Note: This will be handled by the caller since process_input is async
                    return Some(Err(anyhow::anyhow!("eval: async operations not supported in built-ins")));
                }
                Some(Ok(false))
            },
            
            // Information and help
            "type" => {
                if parts.len() > 1 {
                    for cmd in &parts[1..] {
                        // Check if it's a built-in
                        let is_builtin = matches!(*cmd, 
                            "cd" | "pwd" | "export" | "unset" | "set" | "echo" | "printf" |
                            "jobs" | "fg" | "bg" | "kill" | "wait" | "alias" | "unalias" |
                            "history" | "touch" | "mkdir" | "rmdir" | "exit" | "logout" |
                            "source" | "." | "eval" | "type" | "help" | "true" | "false" |
                            "test" | "time" | "umask" | "ulimit" | "read" | "exec"
                        );
                        
                        if is_builtin {
                            println!("{} is a shell builtin", cmd);
                        } else if let Some(path) = crate::utils::path_utils::find_executable(cmd) {
                            println!("{} is {}", cmd, path.display());
                        } else if self.alias_manager.list_aliases().iter().any(|(name, _)| name == cmd) {
                            println!("{} is an alias", cmd);
                        } else {
                            println!("{}: not found", cmd);
                        }
                    }
                } else {
                    eprintln!("type: missing argument");
                }
                Some(Ok(false))
            },
            
            "help" => {
                self.show_help();
                Some(Ok(false))
            },
            
            // Simple utilities
            "true" => {
                Some(Ok(false))
            },
            
            "false" => {
                // In a real shell, this would set the exit status to 1
                Some(Ok(false))
            },
            
            "test" | "[" => {
                // Very simplified test implementation
                if parts.len() < 2 {
                    eprintln!("test: missing argument");
                    return Some(Ok(false));
                }
                
                // Handle the closing bracket for [ command
                let test_parts = if parts[0] == "[" {
                    if parts[parts.len() - 1] != "]" {
                        eprintln!("[: missing closing ]");
                        return Some(Ok(false));
                    }
                    &parts[1..parts.len() - 1]
                } else {
                    &parts[1..]
                };
                
                if test_parts.is_empty() {
                    // Empty test is false
                    eprintln!("Test failed");
                    return Some(Ok(false));
                }
                
                // Handle simple file tests
                if test_parts.len() == 2 && test_parts[0] == "-f" {
                    let path = std::path::Path::new(test_parts[1]);
                    if !path.is_file() {
                        eprintln!("Test failed: {} is not a file", test_parts[1]);
                    }
                } else if test_parts.len() == 2 && test_parts[0] == "-d" {
                    let path = std::path::Path::new(test_parts[1]);
                    if !path.is_dir() {
                        eprintln!("Test failed: {} is not a directory", test_parts[1]);
                    }
                } else if test_parts.len() == 3 && test_parts[1] == "=" {
                    if test_parts[0] != test_parts[2] {
                        eprintln!("Test failed: {} != {}", test_parts[0], test_parts[2]);
                    }
                } else if test_parts.len() == 3 && test_parts[1] == "!=" {
                    if test_parts[0] == test_parts[2] {
                        eprintln!("Test failed: {} == {}", test_parts[0], test_parts[2]);
                    }
                }
                
                Some(Ok(false))
            },
            
            "time" => {
                if parts.len() > 1 {
                    let cmd = parts[1..].join(" ");
                    // Note: This will be handled by the caller since process_input is async
                    return Some(Err(anyhow::anyhow!("time: async operations not supported in built-ins")));
                } else {
                    eprintln!("time: missing command");
                }
                Some(Ok(false))
            },
            
            // System control
            "umask" => {
                if parts.len() > 1 {
                    // Set umask (simplified)
                    if let Ok(mask) = u32::from_str_radix(parts[1], 8) {
                        unsafe {
                            libc::umask(mask);
                        }
                    } else {
                        eprintln!("umask: invalid octal number: {}", parts[1]);
                    }
                } else {
                    // Get current umask
                    unsafe {
                        // Save current umask
                        let current = libc::umask(0);
                        // Restore it
                        libc::umask(current);
                        println!("{:04o}", current);
                    }
                }
                Some(Ok(false))
            },
            
            "ulimit" => {
                // Simplified ulimit implementation
                if parts.len() == 1 {
                    // Show file size limit
                    unsafe {
                        let mut rlim: libc::rlimit = std::mem::zeroed();
                        if libc::getrlimit(libc::RLIMIT_FSIZE, &mut rlim) == 0 {
                            if rlim.rlim_cur == libc::RLIM_INFINITY {
                                println!("unlimited");
                            } else {
                                println!("{}", rlim.rlim_cur);
                            }
                        } else {
                            eprintln!("ulimit: error getting limit");
                        }
                    }
                } else if parts[1] == "-a" {
                    // Show all limits
                    println!("core file size          (blocks, -c) unlimited");
                    println!("data seg size           (kbytes, -d) unlimited");
                    println!("scheduling priority             (-e) 0");
                    println!("file size               (blocks, -f) unlimited");
                    println!("pending signals                 (-i) 15169");
                    println!("max locked memory       (kbytes, -l) 65536");
                    println!("max memory size         (kbytes, -m) unlimited");
                    println!("open files                      (-n) 1024");
                    println!("pipe size            (512 bytes, -p) 8");
                    println!("POSIX message queues     (bytes, -q) 819200");
                    println!("real-time priority              (-r) 0");
                    println!("stack size              (kbytes, -s) 8192");
                    println!("cpu time               (seconds, -t) unlimited");
                    println!("max user processes              (-u) 15169");
                    println!("virtual memory          (kbytes, -v) unlimited");
                    println!("file locks                      (-x) unlimited");
                }
                Some(Ok(false))
            },
            
            // Input/output
            "read" => {
                if parts.len() > 1 {
                    let mut input = String::new();
                    if std::io::stdin().read_line(&mut input).is_ok() {
                        input = input.trim().to_string();
                        
                        // Handle -p prompt option
                        let mut var_start = 1;
                        if parts[1] == "-p" && parts.len() > 3 {
                            print!("{}", parts[2]);
                            std::io::stdout().flush().unwrap_or(());
                            var_start = 3;
                        }
                        
                        // Assign to variables
                        if parts.len() > var_start {
                            let var_name = parts[var_start];
                            std::env::set_var(var_name, input);
                        }
                    }
                } else {
                    eprintln!("read: missing variable name");
                }
                Some(Ok(false))
            },
            
            "exec" => {
                if parts.len() > 1 {
                    let cmd = parts[1].to_string();
                    let args: Vec<String> = parts[1..].iter().map(|s| s.to_string()).collect();
                    
                    if let Some(path) = crate::utils::path_utils::find_executable(&cmd) {
                        use std::os::unix::process::CommandExt;
                        let err = std::process::Command::new(path)
                            .args(&args[1..])
                            .exec();
                        
                        // If we get here, exec failed
                        eprintln!("exec: failed to execute {}: {}", cmd, err);
                    } else {
                        eprintln!("exec: {}: command not found", cmd);
                    }
                } else {
                    // No command specified, just continue
                }
                Some(Ok(false))
            },
            
            // Not a built-in command
            _ => None,
        }
    }

    fn show_help(&self) {
        println!("\n{}", "LLM Shell Help".bright_green());
        println!("{}", "=============".bright_green());
        
        println!("\n{}", "Basic Commands:".bright_yellow());
        println!("  cd [dir]              - Change directory");
        println!("  alias [name[=value]]  - List or set aliases");
        println!("  unalias name          - Remove an alias");
        println!("  jobs                  - List background jobs");
        println!("  fg [job_id]           - Bring job to foreground");
        println!("  bg [job_id]           - Continue job in background");
        println!("  exit                  - Exit the shell");
        
        println!("\n{}", "Special Features:".bright_yellow());
        println!("  command??             - Show command suggestions");
        println!("  ?query                - Ask a question to the LLM");
        println!("  use natural language  - Type commands in plain English");
        
        println!("\n{}", "Examples:".bright_yellow());
        println!("  ? How do I find large files in Linux?");
        println!("  find all python files modified in the last week");
        println!("  ps ??                 - Show suggestions for ps command");
        
        println!("\n{}", "For more information, visit: https://github.com/yourusername/llm-shell".bright_blue());
    }

    async fn process_input(&mut self, input: &str) -> Result<()> {
        // Expand environment variables
        let expanded_input = self.expand_env_vars(input);
        // Check for chat prefix
        if input.starts_with('?') {
            let question = input[1..].trim();
            if !question.is_empty() {
                println!("\n{}", "Thinking...".bright_blue());
                match self.llm_client.chat(question).await {
                    Ok(response) => {
                        println!("\n{}", "Answer:".bright_green());
                        println!("{}\n", response);
                    }
                    Err(e) => println!("Error getting response: {}", e),
                }
                return Ok(());
            }
        }
    
        // Check for natural language patterns
        let natural_language_patterns = [
            "show me", "find all", "list all", "get all", "display", "create a", 
            "make a", "tell me", "give me", "use the", "how do", "what is", "where is",
            "can you", "could you", "would you", "should I", "explain", "help me",
            "search for", "look for", "find files", "count", "calculate", "summarize",
            "who are", "what are", "which", "when", "why", "how many", "how much",
            "get the", "list", "show", "find", "tell", "give", "display", "print",
        ];
        
        let is_natural_language = natural_language_patterns.iter()
            .any(|pattern| input.to_lowercase().starts_with(pattern)) ||
            (input.split_whitespace().count() >= 4);
    
        if is_natural_language {
            debug!("Processing as natural language: {}", input);
            println!("Processing as natural language: {}", input.bright_yellow());
            
            let shell_command = self.llm_client.translate_command(input).await?;
            
            println!("\nTranslated command: {}", shell_command.bright_green());
            
            if let Ok(explanation) = self.documentation.get_command_help(&shell_command).await {
                println!("Explanation: {}", explanation.bright_blue());
            }
            
            // Only ask for confirmation if it's a destructive command
            if self.is_destructive_command(&shell_command) {
                println!("\nWarning: This command may modify or delete data.");
                print!("Proceed? [y/N] ");
                std::io::stdout().flush()?;
                
                let mut response = String::new();
                std::io::stdin().read_line(&mut response)?;
                
                if !response.trim().eq_ignore_ascii_case("y") {
                    println!("Command aborted.");
                    return Ok(());
                }
            }
            
            return self.execute_command(&shell_command);
        }
    
        // Regular command processing
        let commands = self.command_processor.parse(input)?;
        
        for cmd in commands {
            if cmd.is_natural_language {
                debug!("Detected natural language: {}", cmd.command);
                println!("Detected natural language: {}", cmd.command.bright_yellow());
                
                let shell_command = self.llm_client.translate_command(&cmd.command).await?;
                
                println!("\nTranslated command: {}", shell_command.bright_green());
                
                if let Ok(explanation) = self.documentation.get_command_help(&shell_command).await {
                    println!("Explanation: {}", explanation.bright_blue());
                }
                
                // Only ask for confirmation if it's a destructive command
                if self.is_destructive_command(&shell_command) {
                    println!("\nWarning: This command may modify or delete data.");
                    print!("Proceed? [y/N] ");
                    std::io::stdout().flush()?;
                    
                    let mut response = String::new();
                    std::io::stdin().read_line(&mut response)?;
                    
                    if !response.trim().eq_ignore_ascii_case("y") {
                        println!("Command aborted.");
                        continue;
                    }
                }
                
                self.execute_command(&shell_command)?;
            } else {
                // Only ask for confirmation if it's a destructive command
                if self.is_destructive_command(&cmd.command) {
                    println!("\nWarning: This command may modify or delete data.");
                    print!("Proceed? [y/N] ");
                    std::io::stdout().flush()?;
                    
                    let mut response = String::new();
                    std::io::stdin().read_line(&mut response)?;
                    
                    if !response.trim().eq_ignore_ascii_case("y") {
                        println!("Command aborted.");
                        continue;
                    }
                }
                self.execute_command(&cmd.command)?;
            }
        }
        
        Ok(())
    }

    fn is_destructive_command(&self, command: &str) -> bool {
        let destructive_patterns = [
            "rm", "rmdir", "dd", "mkfs", 
            "format", "fdisk", "mkfs",
            ">", "truncate", "shred",
            "mv", "chmod", "chown",
            "sudo rm", "sudo dd", "sudo mkfs",
            "sudo fdisk", "sudo chown", "sudo chmod",
            "pkill", "kill", "killall",
        ];

        let command_words: Vec<&str> = command.split_whitespace().collect();
        if command_words.is_empty() {
            return false;
        }
        
        // Check for redirection that would overwrite files
        if command.contains('>') && !command.contains(">>") {
            return true;
        }
        
        // Check for destructive commands
        for pattern in &destructive_patterns {
            if command.starts_with(pattern) {
                return true;
            }
        }
        
        // Special case for rm with -rf flags
        if command_words[0] == "rm" && 
           (command.contains(" -rf ") || 
            command.contains(" -fr ") || 
            command.contains(" -f ") || 
            command.contains(" --force")) {
            return true;
        }
        
        false
    }

    async fn show_suggestions(&self, command_prefix: Option<&str>) -> Result<String> {
        let suggestions = self.llm_client
            .suggest_commands(&self.context_manager.get_context(), command_prefix)
            .await?;
            
        if suggestions.is_empty() {
            Ok("No suggestions available.".to_string())
        } else {
            Ok(format!("\nSuggested commands:\n{}", 
                suggestions.iter()
                    .map(|s| format!("  {}", s.bright_cyan()))
                    .collect::<Vec<_>>()
                    .join("\n")
            ))
        }
    }

    fn initialize(&mut self) -> Result<()> {
        // Process login shell initialization if needed
        if self.is_login_shell() {
            self.process_profile_files()?;
        }
        
        // Set up environment
        self.setup_environment()?;
        
        // Handle SIGCHLD for job control
        self.job_control.handle_sigchld()?;
        
        // Print welcome message
        self.print_welcome_message();
        
        Ok(())
    }

    fn print_welcome_message(&self) {
        println!("{}", "\n╭───────────────────────────────────────────╮".bright_blue());
        println!("{}", "│           Welcome to LLM Shell            │".bright_green());
        println!("{}", "│                                           │".bright_blue());
        println!("{}", "│  • Use natural language for commands      │".bright_blue());
        println!("{}", "│  • Type '??' after a command for help     │".bright_blue());
        println!("{}", "│  • Start with '?' to ask a question       │".bright_blue());
        println!("{}", "│  • Type 'help' for more information       │".bright_blue());
        println!("{}", "╰───────────────────────────────────────────╯".bright_blue());
        println!();
    }

    fn is_login_shell(&self) -> bool {
        std::env::args()
            .next()
            .map(|arg| arg.starts_with('-'))
            .unwrap_or(false)
    }

    fn process_profile_files(&self) -> Result<()> {
        let home = dirs::home_dir().context("Could not determine home directory")?;
        
        // Process global profile
        if let Ok(contents) = std::fs::read_to_string("/etc/profile") {
            self.process_profile_content(&contents)?;
        }

        // Process user profile
        let profile_path = home.join(".profile");
        if let Ok(contents) = std::fs::read_to_string(profile_path) {
            self.process_profile_content(&contents)?;
        }

        // Process .bash_profile or .bash_login if they exist
        let bash_profile = home.join(".bash_profile");
        let bash_login = home.join(".bash_login");
        
        if bash_profile.exists() {
            if let Ok(contents) = std::fs::read_to_string(bash_profile) {
                self.process_profile_content(&contents)?;
            }
        } else if bash_login.exists() {
            if let Ok(contents) = std::fs::read_to_string(bash_login) {
                self.process_profile_content(&contents)?;
            }
        }

        Ok(())
    }

    fn process_profile_content(&self, content: &str) -> Result<()> {
        for line in content.lines() {
            let line = line.trim();
            
            // Skip comments and empty lines
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            
            if line.starts_with("export ") {
                let parts: Vec<&str> = line["export ".len()..].splitn(2, '=').collect();
                if parts.len() == 2 {
                    let key = parts[0].trim();
                    let value = parts[1].trim().trim_matches('"').trim_matches('\'');
                    
                    // Handle variable expansion in values
                    let expanded_value = self.expand_env_vars(value);
                    std::env::set_var(key, expanded_value);
                }
            }
        }
        Ok(())
    }

    fn setup_environment(&self) -> Result<()> {
        // Set basic environment variables
        if std::env::var("PATH").is_err() {
            std::env::set_var("PATH", "/usr/local/bin:/usr/bin:/bin");
        }
        
        if std::env::var("HOME").is_err() {
            if let Some(home) = dirs::home_dir() {
                std::env::set_var("HOME", home.to_string_lossy().as_ref());
            }
        }
        
        // Set SHELL to point to our shell
        if let Ok(exe) = std::env::current_exe() {
            std::env::set_var("SHELL", exe.to_string_lossy().as_ref());
        }
        
        // Set basic terminal variables
        if std::env::var("TERM").is_err() {
            std::env::set_var("TERM", "xterm-256color");
        }
        
        Ok(())
    }

    fn execute_command(&mut self, command: &str) -> Result<()> {
        // Parse the command
        let pipeline = crate::shell::command_parser::CommandParser::parse(command)?;
        
        // Execute the pipeline
        let exit_code = crate::shell::executor::Executor::execute(&pipeline)?;
        
        if exit_code != 0 {
            eprintln!("Command failed with exit code: {}", exit_code);
        }
        
        Ok(())
    }
}