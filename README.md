### ASSISTANT
# LLM Shell

A powerful shell environment enhanced with Large Language Model capabilities for natural language command processing.

## ⚠️ WARNING ⚠️

**IMPORTANT SECURITY AND SAFETY NOTICE:**

- **This shell can execute arbitrary commands on your system based on natural language input.**
- **LLMs can hallucinate or misinterpret your intent, potentially leading to destructive commands.**
- **Always review translated commands before confirming execution, especially for destructive operations.**
- **Never run this shell with elevated privileges (root/sudo) unless absolutely necessary.**
- **Use in production environments at your own risk - this is primarily a development tool.**
- **The shell may transmit command context to external LLM services.**

By using LLM Shell, you accept full responsibility for any consequences resulting from commands executed through this interface.

## Features

- Natural language command processing
- Command explanations and suggestions
- Environment variable support
- Built-in shell commands
- Command history and completion
- Job control
- LLM-powered assistance

## 10 Cool Ways to Use LLM Shell

1. **Natural Language Commands**
   ```
   find all python files modified in the last week
   ```
   LLM Shell translates this to the appropriate `find` command with the correct syntax.

2. **Ask Questions Directly**
   ```
   ? how do I check disk usage in Linux
   ```
   Get answers to technical questions without leaving your terminal.

3. **Command Suggestions**
   ```
   git ??
   ```
   Append `??` to any command to get contextually relevant suggestions.

4. **Complex Command Generation**
   ```
   create a backup of my home directory excluding node_modules folders
   ```
   Generate complex commands with exclude patterns without memorizing syntax.

5. **Command Explanations**
   ```
   awk '{print $1}' file.txt ??
   ```
   Get explanations of what complex commands actually do.

6. **Data Processing Tasks**
   ```
   extract all email addresses from log.txt
   ```
   Let the LLM generate the appropriate regex and command.

7. **System Administration**
   ```
   show me all processes using more than 1GB of memory
   ```
   Generate and execute system monitoring commands easily.

8. **File Operations**
   ```
   find and delete all empty directories under the current path
   ```
   Perform complex file operations with simple language.

9. **Network Diagnostics**
   ```
   check if port 8080 is open and what process is using it
   ```
   Simplify network troubleshooting with natural language.

10. **Learning Tool**
    ```
    ? what's the difference between grep and egrep
    ```
    Use the shell as a learning platform to understand command-line tools better.

## Installation

```bash
# Clone the repository
git clone https://github.com/yourusername/llm-shell.git

# Build the project
cd llm-shell
cargo build --release

# Install the binary
sudo cp target/release/llm-shell /usr/local/bin/llmsh

# Run the shell
llmsh
```

## Configuration

LLM Shell uses the following environment variables:

- `RUST_LOG`: Set log level (info, warn, error, debug)
- `LLM_HOST`: URL of the LLM service (default: http://localhost:11434)
- `LLM_MODEL`: Model to use (default: qwen2.5:14b)

## Usage

- Regular shell commands work as expected
- Start with `?` to ask a question
- Type natural language for command translation
- Append `??` to any command for suggestions
- Use `help` to see built-in commands

## Built-in Commands

- `cd [dir]`: Change directory
- `pwd`: Print working directory
- `export VAR=VALUE`: Set environment variable
- `echo [text]`: Display text
- `alias [name[=value]]`: Manage aliases
- `history`: View command history
- `help`: Show help information
- And many more standard shell built-ins

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Acknowledgments

- Built with Rust
- Powered by [Ollama](https://ollama.ai/) or other LLM providers
- Inspired by traditional Unix shells and modern AI assistants
