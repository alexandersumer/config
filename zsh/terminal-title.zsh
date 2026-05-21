# shellcheck shell=zsh
# Keep terminal tab titles focused on the current repo/directory. Ghostty uses
# the terminal title when it is set; otherwise it falls back to process names.

__terminal_title_sanitize() {
  local title="$1"

  title=${title//$'\e'/}
  title=${title//$'\a'/}
  title=${title//$'\r'/}
  title=${title//$'\n'/ }
  title=${title//$'\t'/ }

  print -r -- "$title"
}

__terminal_title_truncate_path() {
  local path="$1"
  local max_segments="${TERMINAL_TITLE_MAX_PATH_SEGMENTS:-2}"

  [[ "$max_segments" == <-> ]] || max_segments=2
  if (( max_segments > 0 )); then
    local -a segments
    segments=(${(s:/:)path})
    if (( ${#segments} > max_segments )); then
      path="…/${(j:/:)segments[-$max_segments,-1]}"
    fi
  fi

  print -r -- "$path"
}

__terminal_title_current() {
  local git_root repo relative title

  if command -v git >/dev/null 2>&1; then
    git_root=$(command git rev-parse --show-toplevel 2>/dev/null)
    if [[ -n "$git_root" ]]; then
      repo="${git_root:t}"
      relative=$(command git rev-parse --show-prefix 2>/dev/null)
      relative="${relative%/}"
      if [[ -n "$relative" ]]; then
        title="$repo/$( __terminal_title_truncate_path "$relative" )"
      else
        title="$repo"
      fi
    fi
  fi

  if [[ -z "$title" ]]; then
    if [[ -n "${PWD:-}" ]]; then
      if [[ "$PWD" == "$HOME" ]]; then
        title="~"
      elif [[ "$PWD" == "$HOME"/* ]]; then
        title="~/$( __terminal_title_truncate_path "${PWD#$HOME/}" )"
      else
        title="${PWD:t}"
        [[ -n "$title" ]] || title="/"
      fi
    else
      title="shell"
    fi
  fi

  __terminal_title_sanitize "$title"
}

__terminal_title_set() {
  local title
  title="$(__terminal_title_current)"
  [[ -n "$title" ]] || title="shell"

  # OSC 0 sets both icon and window/tab title; BEL terminates the sequence.
  print -rn -- $'\e]0;'"$title"$'\a'
}

__terminal_title_precmd() { __terminal_title_set }
__terminal_title_chpwd() { __terminal_title_set }
__terminal_title_preexec() { __terminal_title_set }

if [[ -o interactive ]]; then
  autoload -Uz add-zsh-hook 2>/dev/null || return 0
  add-zsh-hook precmd __terminal_title_precmd
  add-zsh-hook chpwd __terminal_title_chpwd
  add-zsh-hook preexec __terminal_title_preexec
  __terminal_title_set
fi
