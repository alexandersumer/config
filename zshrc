########################################
# Startup profiling (if enabled)
########################################
if [[ -n "$ZSH_STARTUP_PROFILING" ]]; then
  zmodload zsh/zprof
fi

########################################
# Environment variables
########################################
export EDITOR=nvim
export ZSH="$HOME/.oh-my-zsh"
export PYENV_ROOT="$HOME/.pyenv"
export SDKMAN_DIR="$HOME/.sdkman"
export GIT_MERGE_AUTOEDIT=no
export ZSH_DISABLE_COMPFIX=true
export CODEX_PLAIN_RESPONSE=1
export PURE_PROMPT_SYMBOL="$"
export PURE_PROMPT_VICMD_SYMBOL="$"

# Project-specific
export TF_VAR_signalfx_auth_token="orAFA364vIojJCcdphPqMA"

# Cache setup
DISABLE_LS_COLORS=true
export ZSH_CACHE_DIR="${XDG_CACHE_HOME:-$HOME/.cache}/oh-my-zsh"
mkdir -p "$ZSH_CACHE_DIR"
export ZSH_COMPDUMP="$ZSH_CACHE_DIR/.zcompdump-$HOST"

########################################
# Shell configuration
########################################
typeset -U PATH

########################################
# PATH setup
########################################
# Homebrew and util-linux
for __path in /opt/homebrew/bin /usr/local/opt/util-linux/bin /opt/homebrew/opt/util-linux/bin; do
  [[ -d "$__path" ]] && path+=("$__path")
done
unset __path

# Pyenv
command -v pyenv >/dev/null || path=("$PYENV_ROOT/bin" $path)

# Orbit
[[ -d "$HOME/.orbit/bin" ]] && path=("$HOME/.orbit/bin" $path)

# Atlassian tools
[[ -d /opt/atlassian/bin ]] && path=(/opt/atlassian/bin $path)

# Structurizr
if command -v brew >/dev/null 2>&1; then
  __structurizr_prefix="$(brew --prefix structurizr-cli 2>/dev/null)"
  if [[ -n "$__structurizr_prefix" ]]; then
    path=("$__structurizr_prefix/libexec" $path)
  fi
  unset __structurizr_prefix
fi

########################################
# Completion setup
########################################
fpath+=(/opt/homebrew/share/zsh/site-functions)

# Drop broken completion stubs (e.g. orphaned Homebrew symlinks)
typeset -i __zsh_broken_completion_removed=0
for __completion in /opt/homebrew/share/zsh/site-functions/_*(N); do
  if [[ -L "$__completion" && ! -e "$__completion" ]]; then
    rm -f "$__completion"
    __zsh_broken_completion_removed=1
  fi
done
if (( __zsh_broken_completion_removed )) && [[ -n "$ZSH_COMPDUMP" && -f "$ZSH_COMPDUMP" ]]; then
  rm -f "$ZSH_COMPDUMP"
fi
unset __zsh_broken_completion_removed
unset __completion

########################################
# Oh-My-Zsh
########################################
ZSH_THEME=""
plugins=(git zsh-autosuggestions zsh-syntax-highlighting)
source "$ZSH/oh-my-zsh.sh"

########################################
# Prompt
########################################
autoload -U promptinit; promptinit
prompt pure

########################################
# Aliases
########################################
JDK21="21.0.8-amzn"
alias jdk21="sdk use java $JDK21"
alias src="cd $HOME/src"
alias convo-ai-1="cd $HOME/src/convo-ai-1 && sdk use java $JDK21"
alias convo-ai-2="cd $HOME/src/convo-ai-2 && sdk use java $JDK21"
alias convo-ai-3="cd $HOME/src/convo-ai-3 && sdk use java $JDK21"
alias stratus1="cd $HOME/src/stratus-1"
alias stratus2="cd $HOME/src/stratus-2"
alias vim="nvim"
alias vi="nvim"
alias clear="echo 'The clear command is disabled.'"
alias checkfix="$HOME/src/scripts/checkfix.sh"
alias zap="$HOME/src/scripts/zap.sh"

########################################
# Functions
########################################

# AI tooling updater
update_ai_tools() {
  setopt LOCAL_OPTIONS NO_MONITOR

  local npm_packages=(
    "@openai/codex"
  )

  local brew_packages=(
    "acli"
  )

  local native_tools=(
    "claude"
  )

  case "$1" in
    -l|--list) printf '%s\n' "${npm_packages[@]}" "${brew_packages[@]}" "${native_tools[@]}"; return 0 ;;
    -h|--help) printf 'Usage: update_ai_tools [-l|--list] [-h|--help]\n'; return 0 ;;
  esac

  local tmpdir
  tmpdir=$(mktemp -d) || return 1

  {
    local pids=() pkg slug failed=0
    local -a attempted_npm=() attempted_brew=() attempted_native=()

    # Update npm packages
    if command -v npm &>/dev/null; then
      for pkg in "${npm_packages[@]}"; do
        slug=${pkg//\//_}
        attempted_npm+=("$pkg")
        ( npm install -g "$pkg@latest" &> "$tmpdir/$slug.log" && echo ok || echo fail ) > "$tmpdir/$slug.status" &
        pids+=($!)
      done
    else
      printf 'update_ai_tools: npm not found, skipping npm packages\n' >&2
    fi

    # Update brew packages (install if missing, upgrade otherwise)
    if command -v brew &>/dev/null; then
      for pkg in "${brew_packages[@]}"; do
        slug="brew_$pkg"
        attempted_brew+=("$pkg")
        (
          if brew list "$pkg" &>/dev/null; then
            # Package installed; upgrade it
            # Note: modern brew returns 0 if already at latest version
            if brew upgrade "$pkg" &> "$tmpdir/$slug.log"; then
              echo ok
            else
              # Check if failure was just "already installed" warning (older brew versions)
              # Pattern matches: "Warning: pkg x.y.z already installed"
              if grep -qE '^Warning:.* already installed' "$tmpdir/$slug.log" 2>/dev/null; then
                echo ok
              else
                echo fail
              fi
            fi
          else
            # Package not installed; install it
            brew install "$pkg" &> "$tmpdir/$slug.log" && echo ok || echo fail
          fi
        ) > "$tmpdir/$slug.status" &
        pids+=($!)
      done
    else
      printf 'update_ai_tools: brew not found, skipping brew packages\n' >&2
    fi

    # Update native tools (Claude Code uses native installer)
    if command -v curl &>/dev/null; then
      for pkg in "${native_tools[@]}"; do
        slug="native_$pkg"
        attempted_native+=("$pkg")
        (
          case "$pkg" in
            claude)
              curl -fsSL https://claude.ai/install.sh | bash &> "$tmpdir/$slug.log" && echo ok || echo fail
              ;;
            *)
              echo fail
              ;;
          esac
        ) > "$tmpdir/$slug.status" &
        pids+=($!)
      done
    else
      printf 'update_ai_tools: curl not found, skipping native tools\n' >&2
    fi

    (( ${#pids[@]} > 0 )) && wait "${pids[@]}"

    # Report npm packages (only those attempted)
    for pkg in "${attempted_npm[@]}"; do
      slug=${pkg//\//_}
      if [[ "$(< "$tmpdir/$slug.status" 2>/dev/null)" == "ok" ]]; then
        printf '✓ %s\n' "$pkg"
      else
        printf '✗ %s\n' "$pkg"
        [[ -f "$tmpdir/$slug.log" ]] && cat "$tmpdir/$slug.log" >&2
        failed=1
      fi
    done

    # Report brew packages (only those attempted)
    for pkg in "${attempted_brew[@]}"; do
      slug="brew_$pkg"
      if [[ "$(< "$tmpdir/$slug.status" 2>/dev/null)" == "ok" ]]; then
        printf '✓ %s (brew)\n' "$pkg"
      else
        printf '✗ %s (brew)\n' "$pkg"
        [[ -f "$tmpdir/$slug.log" ]] && cat "$tmpdir/$slug.log" >&2
        failed=1
      fi
    done

    # Report native tools (only those attempted)
    for pkg in "${attempted_native[@]}"; do
      slug="native_$pkg"
      if [[ "$(< "$tmpdir/$slug.status" 2>/dev/null)" == "ok" ]]; then
        printf '✓ %s (native)\n' "$pkg"
      else
        printf '✗ %s (native)\n' "$pkg"
        [[ -f "$tmpdir/$slug.log" ]] && cat "$tmpdir/$slug.log" >&2
        failed=1
      fi
    done

    return $failed
  } always {
    rm -rf "$tmpdir"
  }
}

# Git helpers
if [[ -r "$HOME/.zsh/git-functions.zsh" ]]; then
  source "$HOME/.zsh/git-functions.zsh"
else
  printf '\033[33mwarning: git helper functions file missing: %s\033[0m\n' "$HOME/.zsh/git-functions.zsh" >&2
fi

########################################
# Tool initialization (lazy loading)
########################################

# Pyenv (lazy init to avoid blocking startup)
if command -v pyenv >/dev/null 2>&1; then
  pyenv() {
    unset -f pyenv
    eval "$(command pyenv init - --no-rehash --no-push-path zsh)"
    pyenv "$@"
  }
fi

# SDKMAN (lazy init to avoid blocking startup)
typeset -g __SDKMAN_INIT_SCRIPT="$SDKMAN_DIR/bin/sdkman-init.sh"
if [[ -s "$__SDKMAN_INIT_SCRIPT" ]]; then
  typeset -gi __SDKMAN_LOADED=0

  __sdkman_lazy_load() {
    (( __SDKMAN_LOADED )) && return 0
    if ! source "$__SDKMAN_INIT_SCRIPT"; then
      printf '\033[31msdkman init failed: %s\033[0m\n' "$__SDKMAN_INIT_SCRIPT" >&2
      return 1
    fi
    __SDKMAN_LOADED=1
    return 0
  }

  sdk() {
    if (( ! __SDKMAN_LOADED )); then
      __sdkman_lazy_load || return $?
    fi
    sdk "$@"
    return $?
  }

  autoload -Uz add-zsh-hook 2>/dev/null

  __sdkman_maybe_load_for_dir() {
    if (( __SDKMAN_LOADED )); then
      add-zsh-hook -d chpwd __sdkman_maybe_load_for_dir 2>/dev/null
      return 0
    fi
    if [[ -f .sdkmanrc ]]; then
      __sdkman_lazy_load || return $?
      add-zsh-hook -d chpwd __sdkman_maybe_load_for_dir 2>/dev/null
    fi
    return 0
  }

  add-zsh-hook chpwd __sdkman_maybe_load_for_dir
  __sdkman_maybe_load_for_dir
fi

########################################
# External configuration
########################################
if [[ -r "$HOME/.local/bin/env" ]]; then
  . "$HOME/.local/bin/env"
fi

########################################
# Startup profiling output
########################################
if [[ -n "$ZSH_STARTUP_PROFILING" ]]; then
  zprof
fi
