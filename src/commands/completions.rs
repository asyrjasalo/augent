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
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    fn test_shell_completions(shell: &str) {
        let args = CompletionsArgs {
            shell: shell.to_string(),
        };

        // Capture output to prevent test noise
        let mut buffer = Vec::new();
        let shell_name = args.shell.to_lowercase();
        let shell_enum = match shell_name.as_str() {
            "bash" => clap_complete::Shell::Bash,
            "elvish" => clap_complete::Shell::Elvish,
            "fish" => clap_complete::Shell::Fish,
            "powershell" | "pwsh" => clap_complete::Shell::PowerShell,
            "zsh" => clap_complete::Shell::Zsh,
            _ => panic!("Unsupported shell"),
        };

        let mut cmd = <crate::cli::Cli as CommandFactory>::command();
        clap_complete::generate(shell_enum, &mut cmd, "augent", &mut buffer);

        // Verify we generated some output
        assert!(
            !buffer.is_empty(),
            "No completion output generated for {}",
            shell
        );
    }

    #[test]
    fn test_completions_bash() {
        test_shell_completions("bash");
    }

    #[test]
    fn test_completions_elvish() {
        test_shell_completions("elvish");
    }

    #[test]
    fn test_completions_fish() {
        test_shell_completions("fish");
    }

    #[test]
    fn test_completions_powershell() {
        test_shell_completions("powershell");
    }

    #[test]
    fn test_completions_pwsh() {
        test_shell_completions("pwsh");
    }

    #[test]
    fn test_completions_zsh() {
        test_shell_completions("zsh");
    }

    #[test]
    fn test_completions_uppercase() {
        test_shell_completions("BASH");
    }

    #[test]
    fn test_completions_mixed_case() {
        test_shell_completions("Zsh");
    }
}
