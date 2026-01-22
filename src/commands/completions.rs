//! Shell completions command

use clap::CommandFactory;

use crate::cli::CompletionsArgs;
use crate::error::Result;

/// Generate shell completions
pub fn run(args: CompletionsArgs) -> Result<()> {
    let shell_name = args.shell.to_lowercase();
    let shell = match shell_name.as_str() {
        "bash" => clap_complete::Shell::Bash,
        "elvish" => clap_complete::Shell::Elvish,
        "fish" => clap_complete::Shell::Fish,
        "powershell" | "pwsh" => clap_complete::Shell::PowerShell,
        "zsh" => clap_complete::Shell::Zsh,
        _ => {
            eprintln!("Unknown shell: {}", args.shell);
            eprintln!("Supported shells: bash, elvish, fish, powershell, zsh");
            std::process::exit(1);
        }
    };

    let mut cmd = <crate::cli::Cli as CommandFactory>::command();
    clap_complete::generate(shell, &mut cmd, "augent", &mut std::io::stdout().lock());

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_completions_bash() {
        let args = CompletionsArgs {
            shell: "bash".to_string(),
        };
        assert!(run(args).is_ok());
    }

    #[test]
    fn test_completions_elvish() {
        let args = CompletionsArgs {
            shell: "elvish".to_string(),
        };
        assert!(run(args).is_ok());
    }

    #[test]
    fn test_completions_fish() {
        let args = CompletionsArgs {
            shell: "fish".to_string(),
        };
        assert!(run(args).is_ok());
    }

    #[test]
    fn test_completions_powershell() {
        let args = CompletionsArgs {
            shell: "powershell".to_string(),
        };
        assert!(run(args).is_ok());
    }

    #[test]
    fn test_completions_pwsh() {
        let args = CompletionsArgs {
            shell: "pwsh".to_string(),
        };
        assert!(run(args).is_ok());
    }

    #[test]
    fn test_completions_zsh() {
        let args = CompletionsArgs {
            shell: "zsh".to_string(),
        };
        assert!(run(args).is_ok());
    }

    #[test]
    fn test_completions_uppercase() {
        let args = CompletionsArgs {
            shell: "BASH".to_string(),
        };
        assert!(run(args).is_ok());
    }

    #[test]
    fn test_completions_mixed_case() {
        let args = CompletionsArgs {
            shell: "Zsh".to_string(),
        };
        assert!(run(args).is_ok());
    }
}
