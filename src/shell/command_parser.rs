// src/shell/command_parser.rs
use anyhow::{Result, Context};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq)]
pub enum Redirection {
    Input(String),      // <
    Output(String),     // >
    Append(String),     // >>
    ErrorOutput(String), // 2>
    ErrorAppend(String), // 2>>
    Pipe,               // |
}

#[derive(Debug, Clone)]
pub struct SimpleCommand {
    pub program: String,
    pub args: Vec<String>,
    pub redirections: Vec<Redirection>,
}

#[derive(Debug, Clone)]
pub struct Pipeline {
    pub commands: Vec<SimpleCommand>,
    pub background: bool,
}

pub struct CommandParser;

impl CommandParser {
    pub fn parse(input: &str) -> Result<Pipeline> {
        let mut commands = Vec::new();
        let mut current_command = SimpleCommand {
            program: String::new(),
            args: Vec::new(),
            redirections: Vec::new(),
        };
        let mut background = false;
        let mut in_quotes = false;
        let mut quote_char = ' ';
        let mut current_token = String::new();
        let mut i = 0;
        let chars: Vec<char> = input.chars().collect();
        
        while i < chars.len() {
            let c = chars[i];
            
            // Handle quotes
            if (c == '"' || c == '\'') && (!in_quotes || quote_char == c) {
                if in_quotes {
                    in_quotes = false;
                } else {
                    in_quotes = true;
                    quote_char = c;
                }
                i += 1;
                continue;
            }
            
            // Inside quotes, just add the character
            if in_quotes {
                current_token.push(c);
                i += 1;
                continue;
            }
            
            // Handle pipe
            if c == '|' {
                if !current_token.is_empty() {
                    if current_command.program.is_empty() {
                        current_command.program = current_token;
                    } else {
                        current_command.args.push(current_token);
                    }
                    current_token = String::new();
                }
                current_command.redirections.push(Redirection::Pipe);
                commands.push(current_command);
                current_command = SimpleCommand {
                    program: String::new(),
                    args: Vec::new(),
                    redirections: Vec::new(),
                };
                i += 1;
                continue;
            }
            
            // Handle redirections
            if c == '<' || c == '>' {
                if !current_token.is_empty() {
                    if current_command.program.is_empty() {
                        current_command.program = current_token;
                    } else {
                        current_command.args.push(current_token);
                    }
                    current_token = String::new();
                }
                
                // Check for >> or 2> or 2>>
                if c == '>' && i + 1 < chars.len() && chars[i + 1] == '>' {
                    // >>
                    i += 2;
                    // Skip whitespace
                    while i < chars.len() && chars[i].is_whitespace() {
                        i += 1;
                    }
                    // Read the filename
                    let mut filename = String::new();
                    while i < chars.len() && !chars[i].is_whitespace() && chars[i] != '|' && chars[i] != '<' && chars[i] != '>' {
                        filename.push(chars[i]);
                        i += 1;
                    }
                    current_command.redirections.push(Redirection::Append(filename));
                } else if i > 0 && chars[i - 1] == '2' && c == '>' && i + 1 < chars.len() && chars[i + 1] == '>' {
                    // 2>>
                    i += 2;
                    // Skip whitespace
                    while i < chars.len() && chars[i].is_whitespace() {
                        i += 1;
                    }
                    // Read the filename
                    let mut filename = String::new();
                    while i < chars.len() && !chars[i].is_whitespace() && chars[i] != '|' && chars[i] != '<' && chars[i] != '>' {
                        filename.push(chars[i]);
                        i += 1;
                    }
                    current_command.redirections.push(Redirection::ErrorAppend(filename));
                } else if i > 0 && chars[i - 1] == '2' && c == '>' {
                    // 2>
                    i += 1;
                    // Skip whitespace
                    while i < chars.len() && chars[i].is_whitespace() {
                        i += 1;
                    }
                    // Read the filename
                    let mut filename = String::new();
                    while i < chars.len() && !chars[i].is_whitespace() && chars[i] != '|' && chars[i] != '<' && chars[i] != '>' {
                        filename.push(chars[i]);
                        i += 1;
                    }
                    current_command.redirections.push(Redirection::ErrorOutput(filename));
                } else if c == '>' {
                    // >
                    i += 1;
                    // Skip whitespace
                    while i < chars.len() && chars[i].is_whitespace() {
                        i += 1;
                    }
                    // Read the filename
                    let mut filename = String::new();
                    while i < chars.len() && !chars[i].is_whitespace() && chars[i] != '|' && chars[i] != '<' && chars[i] != '>' {
                        filename.push(chars[i]);
                        i += 1;
                    }
                    current_command.redirections.push(Redirection::Output(filename));
                } else if c == '<' {
                    // <
                    i += 1;
                    // Skip whitespace
                    while i < chars.len() && chars[i].is_whitespace() {
                        i += 1;
                    }
                    // Read the filename
                    let mut filename = String::new();
                    while i < chars.len() && !chars[i].is_whitespace() && chars[i] != '|' && chars[i] != '<' && chars[i] != '>' {
                        filename.push(chars[i]);
                        i += 1;
                    }
                    current_command.redirections.push(Redirection::Input(filename));
                }
                continue;
            }
            
            // Handle background
            if c == '&' && i == chars.len() - 1 {
                background = true;
                break;
            }
            
            // Handle whitespace
            if c.is_whitespace() {
                if !current_token.is_empty() {
                    if current_command.program.is_empty() {
                        current_command.program = current_token;
                    } else {
                        current_command.args.push(current_token);
                    }
                    current_token = String::new();
                }
                i += 1;
                continue;
            }
            
            // Add character to current token
            current_token.push(c);
            i += 1;
        }
        
        // Add the last token
        if !current_token.is_empty() {
            if current_command.program.is_empty() {
                current_command.program = current_token;
            } else {
                current_command.args.push(current_token);
            }
        }
        
        // Add the last command
        if !current_command.program.is_empty() {
            commands.push(current_command);
        }
        
        Ok(Pipeline {
            commands,
            background,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_command() {
        let input = "ls -la";
        let pipeline = CommandParser::parse(input).unwrap();
        assert_eq!(pipeline.commands.len(), 1);
        assert_eq!(pipeline.commands[0].program, "ls");
        assert_eq!(pipeline.commands[0].args, vec!["-la"]);
        assert_eq!(pipeline.commands[0].redirections.len(), 0);
        assert_eq!(pipeline.background, false);
    }

    #[test]
    fn test_pipe() {
        let input = "ls -la | grep Cargo";
        let pipeline = CommandParser::parse(input).unwrap();
        assert_eq!(pipeline.commands.len(), 2);
        assert_eq!(pipeline.commands[0].program, "ls");
        assert_eq!(pipeline.commands[0].args, vec!["-la"]);
        assert_eq!(pipeline.commands[0].redirections.len(), 1);
        assert_eq!(pipeline.commands[0].redirections[0], Redirection::Pipe);
        assert_eq!(pipeline.commands[1].program, "grep");
        assert_eq!(pipeline.commands[1].args, vec!["Cargo"]);
        assert_eq!(pipeline.commands[1].redirections.len(), 0);
        assert_eq!(pipeline.background, false);
    }

    #[test]
    fn test_redirections() {
        let input = "cat < input.txt > output.txt";
        let pipeline = CommandParser::parse(input).unwrap();
        assert_eq!(pipeline.commands.len(), 1);
        assert_eq!(pipeline.commands[0].program, "cat");
        assert_eq!(pipeline.commands[0].args.len(), 0);
        assert_eq!(pipeline.commands[0].redirections.len(), 2);
        match &pipeline.commands[0].redirections[0] {
            Redirection::Input(filename) => assert_eq!(filename, "input.txt"),
            _ => panic!("Expected Input redirection"),
        }
        match &pipeline.commands[0].redirections[1] {
            Redirection::Output(filename) => assert_eq!(filename, "output.txt"),
            _ => panic!("Expected Output redirection"),
        }
        assert_eq!(pipeline.background, false);
    }

    #[test]
    fn test_append() {
        let input = "echo hello >> output.txt";
        let pipeline = CommandParser::parse(input).unwrap();
        assert_eq!(pipeline.commands.len(), 1);
        assert_eq!(pipeline.commands[0].program, "echo");
        assert_eq!(pipeline.commands[0].args, vec!["hello"]);
        assert_eq!(pipeline.commands[0].redirections.len(), 1);
        match &pipeline.commands[0].redirections[0] {
            Redirection::Append(filename) => assert_eq!(filename, "output.txt"),
            _ => panic!("Expected Append redirection"),
        }
        assert_eq!(pipeline.background, false);
    }

    #[test]
    fn test_error_redirection() {
        let input = "gcc program.c 2> errors.txt";
        let pipeline = CommandParser::parse(input).unwrap();
        assert_eq!(pipeline.commands.len(), 1);
        assert_eq!(pipeline.commands[0].program, "gcc");
        assert_eq!(pipeline.commands[0].args, vec!["program.c"]);
        assert_eq!(pipeline.commands[0].redirections.len(), 1);
        match &pipeline.commands[0].redirections[0] {
            Redirection::ErrorOutput(filename) => assert_eq!(filename, "errors.txt"),
            _ => panic!("Expected ErrorOutput redirection"),
        }
        assert_eq!(pipeline.background, false);
    }

    #[test]
    fn test_background() {
        let input = "sleep 10 &";
        let pipeline = CommandParser::parse(input).unwrap();
        assert_eq!(pipeline.commands.len(), 1);
        assert_eq!(pipeline.commands[0].program, "sleep");
        assert_eq!(pipeline.commands[0].args, vec!["10"]);
        assert_eq!(pipeline.commands[0].redirections.len(), 0);
        assert_eq!(pipeline.background, true);
    }

    #[test]
    fn test_complex_command() {
        let input = "find . -name \"*.rs\" | xargs grep \"fn main\" > results.txt 2> errors.txt &";
        let pipeline = CommandParser::parse(input).unwrap();
        assert_eq!(pipeline.commands.len(), 2);
        assert_eq!(pipeline.commands[0].program, "find");
        assert_eq!(pipeline.commands[0].args, vec![".", "-name", "*.rs"]);
        assert_eq!(pipeline.commands[0].redirections.len(), 1);
        assert_eq!(pipeline.commands[0].redirections[0], Redirection::Pipe);
        assert_eq!(pipeline.commands[1].program, "xargs");
        assert_eq!(pipeline.commands[1].args, vec!["grep", "fn main"]);
        assert_eq!(pipeline.commands[1].redirections.len(), 2);
        match &pipeline.commands[1].redirections[0] {
            Redirection::Output(filename) => assert_eq!(filename, "results.txt"),
            _ => panic!("Expected Output redirection"),
        }
        match &pipeline.commands[1].redirections[1] {
            Redirection::ErrorOutput(filename) => assert_eq!(filename, "errors.txt"),
            _ => panic!("Expected ErrorOutput redirection"),
        }
        assert_eq!(pipeline.background, true);
    }
}
