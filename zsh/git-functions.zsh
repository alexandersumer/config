# shellcheck shell=zsh
# Git helper functions used across interactive shells.

function _branch_from_remote_head_ref() {
    local remote="$1"
    local remote_head_ref="$2"
    local prefix="refs/remotes/$remote/"

    [[ "$remote_head_ref" == "$prefix"* ]] || return 1
    echo "${remote_head_ref#$prefix}"
}

function _get_default_branch() {
    local remote="${1:-origin}"
    local remote_head_ref
    local branch_candidate

    # First try to get from remote HEAD
    remote_head_ref=$(git symbolic-ref --quiet "refs/remotes/$remote/HEAD" 2>/dev/null)
    if [[ -n "$remote_head_ref" ]]; then
        if branch_candidate=$(_branch_from_remote_head_ref "$remote" "$remote_head_ref"); then
            echo "$branch_candidate"
            return 0
        fi
    fi

    # Fall back to checking for main or master
    for branch_candidate in main master; do
        if git show-ref --verify --quiet "refs/remotes/$remote/$branch_candidate"; then
            echo "$branch_candidate"
            return 0
        fi
    done

    return 1
}

function _get_ticket_from_branch() {
    local branch
    branch=$(git rev-parse --abbrev-ref HEAD 2>/dev/null) || return 1
    if [[ "$branch" =~ ^([A-Za-z]+-[0-9]+) ]]; then
        echo "${match[1]}"
    fi
}

function _build_commit_message() {
    local first_arg="$1"
    local ticket

    if [[ "$first_arg" =~ ^[A-Za-z]+-[0-9]+$ ]]; then
        echo "$*"
        return 0
    fi

    ticket=$(_get_ticket_from_branch)
    if [[ -n "$ticket" ]]; then
        echo "$ticket $*"
    else
        echo "$*"
    fi
}

function _remove_worktrees_for_branches() {
    local -a branches=("$@")
    local wt_output
    wt_output=$(git worktree list --porcelain 2>/dev/null)

    # More than one "worktree" line means linked worktrees exist.
    (( $(echo "$wt_output" | grep -c '^worktree ') > 1 )) || return 0

    local wt_path="" wt_line wt_branch branch
    while IFS= read -r wt_line; do
        if [[ "$wt_line" == worktree\ * ]]; then
            wt_path="${wt_line#worktree }"
        elif [[ "$wt_line" == branch\ refs/heads/* ]]; then
            wt_branch="${wt_line#branch refs/heads/}"
            for branch in "${branches[@]}"; do
                if [[ "$wt_branch" == "$branch" ]]; then
                    printf '\033[33mremoving worktree using branch %s: %s\033[0m\n' "$branch" "$wt_path"
                    git worktree remove --force "$wt_path" 2>/dev/null
                    break
                fi
            done
            wt_path=""
        fi
    done <<< "$wt_output"
    git worktree prune 2>/dev/null
}

function _cleanup_fetch_ref_failure() {
    local git_dir="$1"
    local remote="$2"
    local fetch_output="$3"
    local cleanup_round="$4"
    local refs_to_delete ref ref_path log_path _p _dir _stop _cleanup_dir
    local -a refs_array=()
    local -a cleaned_refs=()

    if (( cleanup_round == 0 )); then
        printf '\033[33mfetch failed, cleaning up stale refs and retrying…\033[0m\n' >&2

        # Remove leftover lock files first. Avoid broad remote-tracking ref
        # deletion here because the fast path fetches only one branch; targeted
        # cleanup below removes only refs named in git's failure output.
        for _cleanup_dir in "$git_dir/refs/remotes/$remote" "$git_dir/logs/refs/remotes/$remote"; do
            [[ -d "$_cleanup_dir" ]] || continue
            find "$_cleanup_dir" -name "*.lock" -delete 2>/dev/null
        done
    fi

    refs_to_delete=$(printf '%s\n' "$fetch_output" \
        | grep -E "(cannot lock ref|cannot update the ref|removing stale tracking ref|is at [0-9a-f]+ but expected)" \
        | grep -oE "refs/remotes/${remote}/[^'[:space:]\":]+" \
        | awk '!seen[$0]++')

    if [[ -z "$refs_to_delete" ]]; then
        return 0
    fi

    refs_array=(${(f)refs_to_delete})
    for ref in "${refs_array[@]}"; do
        ref_path="$git_dir/$ref"
        log_path="$git_dir/logs/$ref"

        rm -f "${ref_path}.lock" 2>/dev/null

        if [[ -f "$ref_path" ]]; then
            printf '\033[33mwarning: removing stale ref %s\033[0m\n' "$ref" >&2
            rm -f "$ref_path"
            cleaned_refs+=("$ref")
        elif [[ -d "$ref_path" ]]; then
            printf '\033[33mwarning: removing stale ref directory %s\033[0m\n' "$ref" >&2
            rm -rf "$ref_path"
            cleaned_refs+=("$ref")
        fi

        if [[ -f "$git_dir/packed-refs" ]] && grep -q " ${ref}$" "$git_dir/packed-refs" 2>/dev/null; then
            printf '\033[33mwarning: removing stale packed ref %s\033[0m\n' "$ref" >&2
            sed -i '' "\| ${ref}$|d" "$git_dir/packed-refs"
            if [[ "${cleaned_refs[-1]:-}" != "$ref" ]]; then
                cleaned_refs+=("$ref")
            fi
        fi

        rm -f "${log_path}.lock" "$log_path" 2>/dev/null
        [[ -d "$log_path" ]] && rm -rf "$log_path"

        # Resolve D/F conflicts: a parent of the failing ref may exist as a
        # file (old branch) when a child path (new branch) needs it to be a
        # directory.
        for _p in "$ref_path" "$log_path"; do
            _dir="${_p%/*}"
            _stop="$git_dir/refs/remotes/$remote"
            [[ "$_p" == "$log_path" ]] && _stop="$git_dir/logs/refs/remotes/$remote"
            while [[ "$_dir" != "$_stop" && "$_dir" == "${_stop}/"* ]]; do
                if [[ -f "$_dir" ]]; then
                    printf '\033[33mwarning: removing file blocking directory %s\033[0m\n' "${_dir#$git_dir/}" >&2
                    rm -f "$_dir"
                fi
                _dir="${_dir%/*}"
            done
        done
    done

    if (( ${#cleaned_refs[@]} > 0 )); then
        printf '%s\n' "${cleaned_refs[@]}"
    fi
}

function _fetch_with_ref_cleanup() {
    local git_dir="$1"
    local remote="$2"
    shift 2
    local fetch_output fetch_status=0 stale_output cleaned_output
    local -i cleanup_rounds=0
    local -i max_cleanup_rounds=3
    local -a fetch_cmd=("$@")
    local -a manual_deleted_refs=()
    local -a stale_fetch_refs=()
    local -a cleaned_refs=()

    while true; do
        fetch_output=$("${fetch_cmd[@]}" 2>&1)
        fetch_status=$?

        if (( fetch_status == 0 )); then
            stale_output=$(printf '%s\n' "$fetch_output" \
                | grep "removing stale tracking ref" \
                | grep -oE "refs/remotes/${remote}/[^'[:space:]\":]+" \
                | awk '!seen[$0]++')
            if [[ -n "$stale_output" ]]; then
                stale_fetch_refs=(${(f)stale_output})
            fi
            break
        fi

        if (( cleanup_rounds >= max_cleanup_rounds )); then
            printf '%s\n' "$fetch_output" >&2
            break
        fi

        printf '%s\n' "$fetch_output" >&2
        cleaned_output=$(_cleanup_fetch_ref_failure "$git_dir" "$remote" "$fetch_output" "$cleanup_rounds")
        if [[ -n "$cleaned_output" ]]; then
            cleaned_refs=(${(f)cleaned_output})
            manual_deleted_refs+=("${cleaned_refs[@]}")
        fi

        (( cleanup_rounds++ ))
        if (( cleanup_rounds < max_cleanup_rounds )); then
            sleep $(( cleanup_rounds < 3 ? cleanup_rounds : 3 ))
        fi
    done

    if (( fetch_status != 0 )); then
        printf '\033[31merror: git fetch failed after cleanup attempts\033[0m\n' >&2
        return $fetch_status
    fi

    if (( ${#manual_deleted_refs[@]} > 0 )); then
        printf '\033[33mcleaned up stale tracking refs:\033[0m\n'
        printf '  %s\n' "${manual_deleted_refs[@]}"
    fi

    if (( ${#stale_fetch_refs[@]} > 0 )); then
        printf '\033[33mgit fetch pruned stale tracking refs:\033[0m\n'
        printf '  %s\n' "${stale_fetch_refs[@]}"
    fi

    return 0
}

function _print_reset_to_remote_default_dangerous_error() {
    printf '\033[31merror: reset_to_remote_default would discard local changes\033[0m\n' >&2
    printf '\033[33mcommit, stash, or explicitly discard your changes before retrying\033[0m\n' >&2
}

function _reset_to_remote_default_ref_has_unpublished_commits() {
    local ref="$1"

    [[ -n "$(git rev-list --max-count=1 "$ref" --not --remotes 2>/dev/null)" ]]
}

function _ensure_reset_to_remote_default_worktree_clean() {
    local git_status

    git_status=$(git status --porcelain --untracked-files=all 2>/dev/null)
    if [[ -n "$git_status" ]]; then
        _print_reset_to_remote_default_dangerous_error
        return 1
    fi

    return 0
}

function _ensure_reset_to_remote_default_is_safe() {
    _ensure_reset_to_remote_default_worktree_clean || return $?

    if ! git rev-parse --verify --quiet HEAD >/dev/null 2>&1; then
        return 0
    fi

    if _reset_to_remote_default_ref_has_unpublished_commits HEAD; then
        _print_reset_to_remote_default_dangerous_error
        return 1
    fi

    return 0
}

function _ensure_reset_to_remote_default_prune_safe() {
    local keep_branch="$1"
    local branch

    while IFS= read -r branch; do
        [[ "$branch" == "$keep_branch" ]] && continue
        if _reset_to_remote_default_ref_has_unpublished_commits "$branch"; then
            _print_reset_to_remote_default_dangerous_error
            return 1
        fi
    done < <(git for-each-ref --format='%(refname:short)' refs/heads)

    return 0
}

function _reset_to_remote_default_single() {
    local remote="origin"
    local branch=""
    local arg
    local -a positional_args=()
    local remote_head_ref branch_candidate upstream_ref
    local -a branch_candidates=()
    local -i branch_from_arg=0
    local -i sync_fetch=0
    local -i do_prune=1

    for arg in "$@"; do
        case "$arg" in
            --sync)
                sync_fetch=1
                ;;
            --no-prune)
                do_prune=0
                ;;
            *)
                positional_args+=("$arg")
                ;;
        esac
    done

    if (( ${#positional_args[@]} > 2 )); then
        printf '\033[31merror: too many positional arguments\033[0m\n' >&2
        return 1
    fi

    if (( ${#positional_args[@]} >= 1 )); then
        remote="${positional_args[1]}"
    fi

    if (( ${#positional_args[@]} == 2 )); then
        branch="${positional_args[2]}"
        branch_from_arg=1
    fi

    local git_dir
    git_dir=$(git rev-parse --git-common-dir 2>/dev/null)
    if [[ -z "$git_dir" ]]; then
        git_dir=$(git rev-parse --git-dir 2>/dev/null)
    fi
    if [[ -z "$git_dir" ]]; then
        printf '\033[31merror: not in a git repository\033[0m\n' >&2
        return 1
    fi

    if ! git remote get-url "$remote" >/dev/null 2>&1; then
        printf '\033[31merror: remote %s is not configured\033[0m\n' "$remote" >&2
        return 1
    fi

    _ensure_reset_to_remote_default_worktree_clean || return $?

    if (( sync_fetch )); then
        # ── Synchronous full fetch ────────────────────────────────────
        # Fetch with retry and cleanup of transient/stale remote-tracking
        # ref state, including "cannot lock ref ... is at X but expected Y".
        _fetch_with_ref_cleanup "$git_dir" "$remote" git fetch --prune "$remote" || return $?

        # ── Resolve target branch from full fetch ──────────────────────
        if (( branch_from_arg == 0 )); then
            remote_head_ref=$(git symbolic-ref --quiet "refs/remotes/$remote/HEAD" 2>/dev/null)
            if [[ -n "$remote_head_ref" ]]; then
                branch=$(_branch_from_remote_head_ref "$remote" "$remote_head_ref")
            fi
        fi

        if [[ -z "$branch" ]]; then
            branch_candidates=(main master)
            for branch_candidate in "${branch_candidates[@]}"; do
                if git show-ref --verify --quiet "refs/remotes/$remote/$branch_candidate"; then
                    branch="$branch_candidate"
                    break
                fi
            done
        fi
    else
        # ── Fast single-branch fetch ──────────────────────────────────
        if (( branch_from_arg == 0 )); then
            branch=$(_get_default_branch "$remote")
        fi

        if [[ -n "$branch" ]]; then
            _fetch_with_ref_cleanup "$git_dir" "$remote" git fetch "$remote" "$branch" || return $?
        fi
    fi

    if [[ -z "$branch" ]]; then
        printf '\033[31merror: unable to determine default branch for %s\033[0m\n' "$remote" >&2
        return 1
    fi

    if ! git show-ref --verify --quiet "refs/remotes/$remote/$branch"; then
        printf '\033[31merror: remote %s does not have branch %s\033[0m\n' "$remote" "$branch" >&2
        return 1
    fi

    _ensure_reset_to_remote_default_is_safe || return $?
    if (( do_prune )); then
        _fetch_with_ref_cleanup "$git_dir" "$remote" git fetch --prune --no-tags "$remote" || return $?
    fi
    if git rev-parse --verify --quiet "refs/heads/$branch" >/dev/null 2>&1; then
        if _reset_to_remote_default_ref_has_unpublished_commits "$branch"; then
            _print_reset_to_remote_default_dangerous_error
            return 1
        fi
    fi
    if (( do_prune )); then
        _ensure_reset_to_remote_default_prune_safe "$branch" || return $?
    fi

    # ── Reset to remote branch ─────────────────────────────────────────
    printf 'resetting to \033[32m%s/%s\033[0m\n' "$remote" "$branch"

    if git rev-parse --verify --quiet "refs/heads/$branch" >/dev/null 2>&1; then
        git switch "$branch" || return $?
    else
        git switch --create "$branch" "$remote/$branch" || return $?
    fi

    upstream_ref=$(git rev-parse --symbolic-full-name "$branch@{upstream}" 2>/dev/null)
    if [[ "$upstream_ref" != "refs/remotes/$remote/$branch" ]]; then
        if ! git branch --set-upstream-to "$remote/$branch" "$branch" >/dev/null 2>&1; then
            printf '\033[33mwarning: unable to set upstream to %s/%s\033[0m\n' "$remote" "$branch"
        fi
    fi

    git reset --hard "$remote/$branch" || return $?

    # ── Prune local branches ──────────────────────────────────────────
    if (( do_prune )); then
        prune_all_except_remote_default "$branch" || return $?
    fi

    # ── Background fetch for remote branch availability ───────────────
    if (( ! sync_fetch )); then
        printf '\033[33mupdating remote branches in background…\033[0m\n'
        # Run the background fetch in its own subshell so it fully detaches
        # from the parent job table.  This avoids `disown: no current job`
        # errors when reset_to_remote_default itself is called from inside a
        # subshell/pipeline (e.g. by reset_all_to_remote_default), which would
        # otherwise leak a non-zero exit status out of this function.
        ( git fetch --prune --no-tags "$remote" &>/dev/null & ) 2>/dev/null
    fi

    return 0
}

function rebase_on_remote_default() {
    local remote="origin"
    local default_branch
    local current_branch
    local git_status
    local git_dir
    local rebase_output

    git_dir=$(git rev-parse --git-dir 2>/dev/null)
    if [[ -z "$git_dir" ]]; then
        printf '\033[31merror: not in a git repository\033[0m\n'
        return 1
    fi

    current_branch=$(git rev-parse --abbrev-ref HEAD)

    if [[ "$current_branch" == "HEAD" ]]; then
        printf '\033[31merror: you are in a detached HEAD state\033[0m\n'
        printf '\033[33mcheckout a branch first:\033[0m \033[32mgit checkout -b <branch-name>\033[0m\n'
        return 1
    fi

    default_branch=$(_get_default_branch "$remote")
    if [[ -z "$default_branch" ]]; then
        printf '\033[31merror: could not detect default branch (tried remote HEAD, main, master)\033[0m\n'
        return 1
    fi

    if [[ -d "$git_dir/rebase-merge" ]] || [[ -d "$git_dir/rebase-apply" ]]; then
        printf '\033[31m✗ a rebase is already in progress\033[0m\n'
        printf '\033[33moptions:\033[0m\n'
        printf '  1. continue rebase: \033[32mgit rebase --continue\033[0m\n'
        printf '  2. skip this patch: \033[32mgit rebase --skip\033[0m\n'
        printf '  3. abort rebase:    \033[32mgit rebase --abort\033[0m\n'
        return 1
    fi

    git_status=$(git status --porcelain 2>/dev/null)
    if [[ -n "$git_status" ]]; then
        local file_count=${#${(f)git_status}}
        printf '\033[31m✗ cannot rebase: you have uncommitted changes\033[0m\n'
        printf '\033[33mmodified files:\033[0m\n'
        printf '%s\n' "${(f)git_status}" | head -20
        if (( file_count > 20 )); then
            printf '\033[33m... and %s more files\033[0m\n' "$((file_count - 20))"
        fi
        printf '\n'
        printf '\033[33moptions:\033[0m\n'
        printf '  1. commit your changes: \033[32mgit add -A && git commit -m '\''your message'\''\033[0m\n'
        printf '  2. stash your changes:  \033[32mgit stash\033[0m\n'
        printf '  3. discard changes:     \033[32mgit reset --hard\033[0m (warning: this will lose changes)\n'
        return 1
    fi

    printf '\033[33mcurrent branch:\033[0m %s\n' "$current_branch"
    printf '\033[33mrebasing onto:\033[0m %s/%s\n' "$remote" "$default_branch"
    printf 'fetching and rebasing: \033[32mgit fetch %s %s && git rebase %s/%s\033[0m\n' "$remote" "$default_branch" "$remote" "$default_branch"

    if ! git fetch "$remote" "$default_branch" 2>&1; then
        printf '\033[31m✗ failed to fetch %s/%s\033[0m\n' "$remote" "$default_branch"
        printf '\033[33mcheck your network connection and remote configuration\033[0m\n'
        return 1
    fi

    printf '\033[32m✓ fetched latest %s\033[0m\n' "$default_branch"

    rebase_output=$(git rebase "$remote/$default_branch" 2>&1)
    local rebase_exit_code=$?

    if [[ $rebase_exit_code -eq 0 ]]; then
        printf '\033[32m✓ successfully rebased %s onto %s/%s\033[0m\n' "$current_branch" "$remote" "$default_branch"
        return 0
    fi

    echo "$rebase_output"

    if [[ "$rebase_output" == *"error: cannot rebase: Your index contains uncommitted changes"* ]]; then
        printf '\033[31m✗ cannot rebase: uncommitted changes detected\033[0m\n'
        printf '\033[33mthis shouldn'\''t happen - please report this issue\033[0m\n'
        printf '\033[33mtry:\033[0m \033[32mgit status\033[0m to see what'\''s wrong\n'
    elif [[ "$rebase_output" == *CONFLICT* ]]; then
        printf '\033[31m✗ rebase encountered merge conflicts\033[0m\n'
        printf '\033[33mresolve conflicts in the files listed above, then:\033[0m\n'
        printf '  1. stage resolved files: \033[32mgit add <resolved-files>\033[0m\n'
        printf '  2. continue rebase:      \033[32mgit rebase --continue\033[0m\n'
        printf '  3. or abort rebase:      \033[32mgit rebase --abort\033[0m\n'
    elif [[ -d "$git_dir/rebase-merge" ]] || [[ -d "$git_dir/rebase-apply" ]]; then
        printf '\033[31m✗ rebase stopped (possibly due to conflicts)\033[0m\n'
        printf '\033[33mcheck status and resolve any issues:\033[0m\n'
        printf '  1. check status:    \033[32mgit status\033[0m\n'
        printf '  2. continue rebase: \033[32mgit rebase --continue\033[0m\n'
        printf '  3. or abort rebase: \033[32mgit rebase --abort\033[0m\n'
    else
        printf '\033[31m✗ rebase failed\033[0m\n'
        printf '\033[33mcheck the error messages above for details\033[0m\n'
    fi

    return 1
}

function restore_from_remote_default() {
    if [ $# -eq 0 ]; then
        printf '\033[31merror: no file path provided\033[0m\n'
        return 1
    fi

    if ! git rev-parse --git-dir >/dev/null 2>&1; then
        printf '\033[31merror: not in a git repository\033[0m\n'
        return 1
    fi

    local remote="origin"
    local default_branch
    default_branch=$(_get_default_branch "$remote")
    if [[ -z "$default_branch" ]]; then
        printf '\033[31merror: could not detect default branch (tried remote HEAD, main, master)\033[0m\n'
        return 1
    fi

    local file_path filename
    for file_path in "$@"; do
        filename="$(basename "$file_path")"
        if git restore --source "$remote/$default_branch" "$file_path"; then
            printf '\033[32m✓ restored '\''%s'\'' from %s/%s\033[0m\n' "$filename" "$remote" "$default_branch"
        else
            printf '\033[31m✗ failed to restore '\''%s'\''\033[0m\n' "$filename"
            return 1
        fi
    done
}

function branch_create() {
    if [ $# -eq 0 ]; then
        printf '\033[31merror: please provide a branch name\033[0m\n'
        return 1
    fi

    local joined="$*"
    local branch_name=${joined// /-}
    printf 'creating branch: \033[32m%s\033[0m\n' "$branch_name"
    git switch -c "${branch_name}"
}

function prune_all_except_remote_default() {
    local keep_branch="$1"
    local remote="origin"
    local current_branch
    local -a branches_to_delete=()

    if ! git rev-parse --git-dir >/dev/null 2>&1; then
        printf '\033[31merror: not in a git repository\033[0m\n'
        return 1
    fi

    if [[ -z "$keep_branch" ]]; then
        keep_branch=$(_get_default_branch "$remote")
        if [[ -z "$keep_branch" ]]; then
            printf '\033[31merror: could not detect default branch (tried remote HEAD, main, master)\033[0m\n'
            return 1
        fi
    fi

    if ! git show-ref --verify --quiet "refs/heads/$keep_branch"; then
        printf '\033[31merror: local branch '\''%s'\'' not found\033[0m\n' "${keep_branch}"
        printf '\033[33mtry:\033[0m \033[32mgit fetch %s %s:%s\033[0m\n' "$remote" "${keep_branch}" "${keep_branch}"
        return 1
    fi

    current_branch=$(git rev-parse --abbrev-ref HEAD 2>/dev/null)

    if [[ "$current_branch" != "$keep_branch" ]]; then
        if git switch --quiet "$keep_branch"; then
            printf 'switched to keep branch: \033[32m%s\033[0m\n' "${keep_branch}"
        else
            printf '\033[31merror: unable to switch to '\''%s'\''\033[0m\n' "${keep_branch}"
            return 1
        fi
    fi

    while IFS= read -r branch; do
        [[ "$branch" == "$keep_branch" ]] && continue
        branches_to_delete+=("$branch")
    done < <(git for-each-ref --format='%(refname:short)' refs/heads)

    _ensure_reset_to_remote_default_prune_safe "$keep_branch" || return $?

    if (( ${#branches_to_delete[@]} == 0 )); then
        printf '\033[33mno local branches to delete\033[0m\n'
        return 0
    fi

    # Branches came from git for-each-ref — known to exist, safe to skip checks.
    prune_branch --force "${branches_to_delete[@]}"
    return $?
}


function prune_branch() {
    local -i force=0
    local -a raw_args=()
    local arg

    for arg in "$@"; do
        if [[ "$arg" == "--force" ]]; then
            force=1
        else
            raw_args+=("$arg")
        fi
    done

    if (( ${#raw_args[@]} == 0 )); then
        printf '\033[31merror: provide at least one branch to prune\033[0m\n'
        return 1
    fi

    local current_branch
    local branch
    local -a targets=()

    if ! git rev-parse --git-dir >/dev/null 2>&1; then
        printf '\033[31merror: not in a git repository\033[0m\n'
        return 1
    fi

    current_branch=$(git rev-parse --abbrev-ref HEAD 2>/dev/null)

    if (( force )); then
        # Caller already validated branches — skip per-branch checks.
        for branch in "${raw_args[@]}"; do
            [[ "$branch" == "$current_branch" ]] && continue
            targets+=("$branch")
        done
    else
        for branch in "${raw_args[@]}"; do
            if [[ -z "$branch" ]]; then
                printf '\033[31merror: branch name cannot be empty\033[0m\n'
                return 1
            fi

            if [[ "$branch" == "main" || "$branch" == "master" ]]; then
                printf '\033[31merror: refusing to prune protected branch '\''%s'\''\033[0m\n' "${branch}"
                return 1
            fi

            if [[ "$branch" == "$current_branch" ]]; then
                printf '\033[31merror: cannot prune the current branch '\''%s'\''\033[0m\n' "${branch}"
                printf '\033[33mswitch to another branch first\033[0m\n'
                return 1
            fi

            if ! git show-ref --verify --quiet "refs/heads/$branch"; then
                printf '\033[31merror: local branch '\''%s'\'' not found\033[0m\n' "${branch}"
                return 1
            fi

            targets+=("$branch")
        done
    fi

    if (( ${#targets[@]} == 0 )); then
        printf '\033[33mnothing to prune\033[0m\n'
        return 0
    fi

    _remove_worktrees_for_branches "${targets[@]}"

    printf 'pruning local branches: \033[32m%s\033[0m\n' "${(j: :)targets}"
    if git branch -D "${targets[@]}"; then
        return 0
    fi

    printf '\033[31merror: failed to prune one or more branches\033[0m\n'
    return 1
}

function force_push() {
    printf 'force pushing with lease: \033[32mgit push --force-with-lease\033[0m\n'
    git push --force-with-lease
}

function _stage_commit_git_impl() {
    local message=$1
    local skip_detekt=$2
    git add .
    if git diff --cached --quiet; then
        printf 'Nothing to commit.\n'
        return 2
    fi
    if [[ "$skip_detekt" = "1" ]]; then
        SKIP_DETEKT=1 git commit -m "$message"
    else
        git commit -m "$message"
    fi
}

function _gsc_impl() {
    local skip_detekt=$1
    local do_push=$2
    shift 2

    if [ $# -eq 0 ]; then
        printf '\033[31merror: please provide a commit message\033[0m\n'
        return 1
    fi

    local message
    message=$(_build_commit_message "$@")

    local label="staging and committing"
    (( skip_detekt )) && label+=" (skip detekt)"
    (( do_push )) && label="${label/and committing/committing and pushing}"
    printf '%s: \033[32m\"%s\"\033[0m\n' "${label}" "$message"

    _stage_commit_git_impl "$message" "$skip_detekt"
    local commit_status=$?

    if (( commit_status == 2 )); then
        return 0
    fi

    if (( commit_status != 0 )); then
        if ! git diff --quiet || ! git diff --cached --quiet; then
            printf '\033[33mHooks modified files, staging changes and retrying commit...\033[0m\n'
            git add -u
            _stage_commit_git_impl "$message" "$skip_detekt"
            commit_status=$?
        fi
    fi

    if (( commit_status != 0 && commit_status != 2 )); then
        return $commit_status
    fi

    if (( do_push )); then
        if ! git push; then
            printf '\033[31m✗ push failed\033[0m\n'
            printf '\033[33moptions:\033[0m\n'
            printf '  1. pull and retry:  \033[32mgit pull --rebase && git push\033[0m\n'
            printf '  2. force push:      \033[32mforce_push\033[0m (use with caution)\n'
            return 1
        fi
    fi
}

function gsc()       { _gsc_impl 0 0 "$@"; }
function fast_gsc()  { _gsc_impl 1 0 "$@"; }
function gscp()      { _gsc_impl 0 1 "$@"; }
function fast_gscp() { _gsc_impl 1 1 "$@"; }

function merge_from_remote_default() {
    local remote="origin"
    local default_branch

    default_branch=$(_get_default_branch "$remote")
    if [[ -z "$default_branch" ]]; then
        printf '\033[31merror: could not detect default branch (tried remote HEAD, main, master)\033[0m\n'
        return 1
    fi

    printf 'fetching and merging: \033[32mgit fetch %s %s && git merge %s/%s\033[0m\n' "$remote" "$default_branch" "$remote" "$default_branch"
    git fetch "$remote" "$default_branch" && git merge "$remote/$default_branch"
}

function quit_merge() {
    printf 'quitting merge: \033[32mgit merge --quit\033[0m\n'
    git merge --quit
}

function abort_merge() {
    printf 'aborting merge: \033[32mgit merge --abort\033[0m\n'
    git merge --abort
}

function hard_reset_head() {
    if ! git rev-parse --git-dir > /dev/null 2>&1; then
        printf '\033[31merror: not in a git repository\033[0m\n'
        return 1
    fi
    printf 'hard reset to HEAD: \033[32mgit reset --hard\033[0m\n'
    git reset --hard
}

function soft_reset_remote_default() {
    if ! git rev-parse --git-dir > /dev/null 2>&1; then
        printf '\033[31merror: not in a git repository\033[0m\n'
        return 1
    fi

    local remote="origin"
    local default_branch
    default_branch=$(_get_default_branch "$remote")
    if [[ -z "$default_branch" ]]; then
        printf '\033[31merror: could not detect default branch (tried remote HEAD, main, master)\033[0m\n'
        return 1
    fi

    if ! git show-ref --verify --quiet "refs/heads/$default_branch"; then
        printf '\033[31merror: local '\''%s'\'' branch not found\033[0m\n' "${default_branch}"
        printf '\033[33mtry:\033[0m \033[32mgit fetch %s %s:%s\033[0m\n' "$remote" "${default_branch}" "${default_branch}"
        return 1
    fi
    printf 'soft reset onto %s: \033[32mgit reset --soft %s\033[0m\n' "${default_branch}" "${default_branch}"
    git reset --soft "$default_branch"
}

function _is_transient_git_failure() {
    local log_file="$1"
    [[ -s "$log_file" ]] || return 1
    grep -q -E \
        -e 'Permission denied \(publickey' \
        -e 'kex_exchange_identification' \
        -e 'Could not resolve host' \
        -e 'Connection (timed out|reset|refused|closed)' \
        -e 'Operation timed out' \
        -e 'early EOF' \
        -e 'RPC failed' \
        -e 'unable to access .*(Couldn'\''t connect|Failed to connect|Could not resolve)' \
        -e 'remote end hung up unexpectedly' \
        -e 'fetch-pack: unexpected disconnect' \
        -e 'TLS connection was non-properly terminated' \
        -e 'HTTP/[0-9.]+ 5[0-9]{2}' \
        -- "$log_file"
}

function _reset_to_remote_default_multi() {
    local root=""
    local arg
    local -a reset_args=()
    local -a positional=()
    local -a repos=()
    local -a failed_repos=()
    local entry repo_name log_file
    local -i total=0 ok_count=0 fail_count=0 skipped=0 retried_count=0
    local -i index=0 status_code=0 attempt=0 delay=0
    local -i max_attempts=3 retry_base_delay=2
    local -i parsing=1

    while (( parsing && $# > 0 )); do
        arg="$1"
        case "$arg" in
            --retries)
                if [[ -z "${2-}" || "$2" != <-> ]]; then
                    printf '\033[31merror: --retries requires a non-negative integer\033[0m\n' >&2
                    return 1
                fi
                max_attempts=$2
                (( max_attempts < 1 )) && max_attempts=1
                shift 2
                ;;
            --retry-delay)
                if [[ -z "${2-}" || "$2" != <-> ]]; then
                    printf '\033[31merror: --retry-delay requires a non-negative integer\033[0m\n' >&2
                    return 1
                fi
                retry_base_delay=$2
                shift 2
                ;;
            --)
                shift
                reset_args=("$@")
                parsing=0
                ;;
            --*)
                printf '\033[31merror: unknown option %s\033[0m\n' "$arg" >&2
                return 1
                ;;
            *)
                positional+=("$arg")
                shift
                ;;
        esac
    done

    if (( ${#positional[@]} > 1 )); then
        printf '\033[31merror: too many positional arguments\033[0m\n' >&2
        return 1
    fi

    root="${positional[1]:-$PWD}"

    if [[ -d "$root" ]]; then
        root="${root:A}"
    else
        printf '\033[31merror: %s is not a directory\033[0m\n' "$root" >&2
        return 1
    fi

    for entry in "$root"/*(N-/); do
        repos+=("$entry")
    done

    if (( ${#repos[@]} == 0 )); then
        printf '\033[33mno subdirectories found in %s\033[0m\n' "$root"
        return 0
    fi

    total=${#repos[@]}
    printf 'scanning \033[32m%s\033[0m (%d entries, retries=%d)\n' \
        "$root" "$total" "$max_attempts"

    log_file=$(mktemp -t reset_all_to_remote_default.XXXXXX) || {
        printf '\033[31merror: could not create temp log file\033[0m\n' >&2
        return 1
    }
    trap 'rm -f "$log_file"; trap - INT TERM; return 130' INT TERM

    for entry in "${repos[@]}"; do
        (( index++ ))
        repo_name="${entry##*/}"

        if ! git -C "$entry" rev-parse --git-dir >/dev/null 2>&1; then
            (( skipped++ ))
            printf '\n[%d/%d] \033[33mskip\033[0m %s (not a git repository)\n' \
                "$index" "$total" "$repo_name"
            continue
        fi

        printf '\n[%d/%d] \033[32m%s\033[0m\n' "$index" "$total" "$repo_name"
        printf -- '----------------------------------------\n'

        attempt=1
        status_code=0
        while (( attempt <= max_attempts )); do
            : > "$log_file"
            if (( ${#reset_args[@]} > 0 )); then
                ( cd -- "$entry" && _reset_to_remote_default_single "${reset_args[@]}" ) 2>&1 | tee "$log_file"
                status_code=${pipestatus[1]}
            else
                ( cd -- "$entry" && _reset_to_remote_default_single ) 2>&1 | tee "$log_file"
                status_code=${pipestatus[1]}
            fi

            if (( status_code == 0 )); then
                break
            fi

            if (( attempt >= max_attempts )) || ! _is_transient_git_failure "$log_file"; then
                break
            fi

            delay=$(( retry_base_delay * (1 << (attempt - 1)) ))
            printf '\033[33mretry attempt %d/%d in %ds\033[0m\n' \
                "$(( attempt + 1 ))" "$max_attempts" "$delay"
            printf -- '----------------------------------------\n'
            sleep "$delay"
            (( attempt++ ))
        done

        if (( status_code == 0 )); then
            (( ok_count++ ))
            if (( attempt > 1 )); then
                (( retried_count++ ))
                printf '\033[32mrecovered after %d attempts\033[0m\n' "$attempt"
            fi
        else
            (( fail_count++ ))
            local plural=s
            (( attempt == 1 )) && plural=
            failed_repos+=("$repo_name (exit $status_code after $attempt attempt$plural)")
        fi
    done

    rm -f "$log_file"
    trap - INT TERM

    printf '\n========================================\n'
    printf 'summary: %d total, \033[32m%d ok\033[0m (\033[33m%d retried\033[0m), \033[31m%d failed\033[0m, \033[33m%d skipped\033[0m\n' \
        "$total" "$ok_count" "$retried_count" "$fail_count" "$skipped"

    if (( fail_count > 0 )); then
        printf '\033[31mfailed repositories:\033[0m\n'
        for entry in "${failed_repos[@]}"; do
            printf '  %s\n' "$entry"
        done
        return 1
    fi

    return 0
}

function reset_to_remote_default() {
    local arg
    local -a forwarded_args=()
    local -a multi_options=()
    local -a positional=()
    local -i parsing=1
    local -i mode_force=0
    local mode=""
    local target=""
    local target_abs=""

    while (( parsing && $# > 0 )); do
        arg="$1"
        case "$arg" in
            --help|-h)
                printf 'usage: reset_to_remote_default [--single|--multi] [--retries N] [--retry-delay SECS] [path] [-- single-repo args...]\n'
                printf '\n'
                printf 'Reset git repositories to their default branch on the remote.\n'
                printf '\n'
                printf 'If PATH (or the current directory) is itself a git repository,\n'
                printf 'runs in single-repo mode.\n'
                printf 'Otherwise, scans every immediate subdirectory of PATH (default: cwd)\n'
                printf 'and resets each one that is a git repository, with retries on\n'
                printf 'transient ssh/network failures.\n'
                printf '\n'
                printf 'options:\n'
                printf '  --single        force single-repo mode for the resolved path\n'
                printf '  --multi         force multi-repo mode for the resolved path\n'
                printf '  --retries N     multi-repo: max attempts per repo on transient failure (default: 3)\n'
                printf '  --retry-delay S multi-repo: base seconds between retries, doubled each time (default: 2)\n'
                printf '  --              forward remaining args to single-repo mode (per repo in multi)\n'
                printf '\n'
                printf 'single-repo args:\n'
                printf '  --sync          fetch all remote refs synchronously before resetting\n'
                printf '  --no-prune      skip pruning other local branches\n'
                printf '  [remote]        remote name (default: origin)\n'
                printf '  [branch]        target branch (default: remote default branch)\n'
                return 0
                ;;
            --single)
                mode="single"
                mode_force=1
                shift
                ;;
            --multi)
                mode="multi"
                mode_force=1
                shift
                ;;
            --retries|--retry-delay)
                if [[ -z "${2-}" ]]; then
                    printf '\033[31merror: %s requires a value\033[0m\n' "$arg" >&2
                    return 1
                fi
                multi_options+=("$arg" "$2")
                shift 2
                ;;
            --)
                shift
                forwarded_args=("$@")
                parsing=0
                ;;
            --*)
                # Unknown leading option: treat as forwarded single-repo arg.
                forwarded_args+=("$arg")
                shift
                ;;
            *)
                positional+=("$arg")
                shift
                ;;
        esac
    done

    # Auto-detect mode if not forced.
    if (( ! mode_force )); then
        if (( ${#positional[@]} >= 1 )) && [[ -d "${positional[1]}" ]]; then
            target="${positional[1]}"
            target_abs="${target:A}"
            if git -C "$target_abs" rev-parse --git-dir >/dev/null 2>&1; then
                mode="single"
            else
                mode="multi"
            fi
        else
            if git rev-parse --git-dir >/dev/null 2>&1; then
                mode="single"
            else
                mode="multi"
            fi
        fi
    fi

    if [[ "$mode" == "single" ]]; then
        if (( ${#multi_options[@]} > 0 )); then
            printf '\033[31merror: --retries and --retry-delay only apply in multi-repo mode\033[0m\n' >&2
            return 1
        fi
        if (( ${#positional[@]} >= 1 )) && [[ -d "${positional[1]}" ]]; then
            target="${positional[1]}"
            target_abs="${target:A}"
            shift_positional=("${positional[@]:1}")
            ( cd -- "$target_abs" && _reset_to_remote_default_single "${shift_positional[@]}" "${forwarded_args[@]}" )
            return $?
        fi
        _reset_to_remote_default_single "${positional[@]}" "${forwarded_args[@]}"
        return $?
    fi

    # multi mode: rebuild argv exactly as _reset_to_remote_default_multi expects:
    # [root] [--retries N] [--retry-delay S] [-- forwarded_args...]
    local -a multi_argv=()
    if (( ${#positional[@]} >= 1 )); then
        if (( ${#positional[@]} > 1 )); then
            printf '\033[31merror: too many positional arguments\033[0m\n' >&2
            return 1
        fi
        multi_argv+=("${positional[1]}")
    fi
    if (( ${#multi_options[@]} > 0 )); then
        multi_argv+=("${multi_options[@]}")
    fi
    if (( ${#forwarded_args[@]} > 0 )); then
        multi_argv+=("--" "${forwarded_args[@]}")
    fi

    _reset_to_remote_default_multi "${multi_argv[@]}"
}

function reset_all_to_remote_default() {
    reset_to_remote_default --multi "$@"
}

# Backward-compatible aliases for the previous origin-named entry points.
function reset_to_origin()              { reset_to_remote_default "$@"; }
function reset_all_to_origin()          { reset_all_to_remote_default "$@"; }
function rebase_on_origin()             { rebase_on_remote_default "$@"; }
function restore_from_origin()          { restore_from_remote_default "$@"; }
function prune_all_except_origin()      { prune_all_except_remote_default "$@"; }
function merge_from_origin()            { merge_from_remote_default "$@"; }
function soft_reset_origin()            { soft_reset_remote_default "$@"; }
