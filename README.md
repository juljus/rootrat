# rootrat

A simple dotfiles manager. Track files and directories, sync them between your system and a git-backed repo.

## Install

```
cargo install rootrat
```

## Quick start

```bash
# Initialize a new dotfiles repo
mkdir ~/dotfiles && cd ~/dotfiles
rootrat init

# Start tracking files
rootrat add ~/.config/ghostty/config
rootrat add ~/.config/nvim  # directories work too

# See what's tracked
rootrat status

# After editing your dotfiles, collect changes into the repo
rootrat collect

# On another machine, clone and apply
rootrat init github.com/you/dotfiles
```

## Commands

| Command | Description |
|---------|-------------|
| `init [url]` | Initialize repo, or clone from a URL |
| `add <path>` | Track a file or directory |
| `rm <path>` | Stop tracking |
| `status` | Show tracked file states |
| `diff [path]` | Show differences between repo and system |
| `collect` | Sync system changes into the repo |
| `apply` | Sync repo files to the system (interactive) |

## How it works

rootrat keeps two config files:

- **`~/.config/rootrat/rootrat.toml`** -- points to your repo directory
- **`<repo>/rootrat.toml`** -- lists tracked files, directories, and ignore patterns

All repo changes are auto-committed with git. The repo is yours to push, pull, and share.

## License

GPL-3.0
