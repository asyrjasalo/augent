use clap::Parser;

/// Arguments for completions command
#[derive(Parser, Debug)]
#[command(after_help = "EXAMPLES:\n  \
                  Generate bash completions:\n    augent completions bash > ~/.bash_completion.d/augent\n\n\
                  Generate zsh completions:\n    augent completions zsh > ~/.zfunc/_augent\n\n\
                  Generate fish completions:\n    augent completions fish > ~/.config/fish/completions/augent.fish\n\n\
                  Generate PowerShell completions:\n    augent completions powershell")]
pub struct CompletionsArgs {
    /// Shell type (bash, elvish, fish, powershell, zsh)
    pub shell: String,
}
