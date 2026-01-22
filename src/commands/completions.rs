//! Shell completions command

use clap::CommandFactory;
use miette::Result;

use crate::cli::CompletionsArgs;

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
}
