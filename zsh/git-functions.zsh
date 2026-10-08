# shellcheck shell=zsh
# Git helper functions used across interactive shells.

function _branch_from_remote_head_ref() {
    local remote="$1"
    local remote_head_ref="$2"
    local prefix="refs/remotes/$remote/"

    [[ "$remote_head_ref" == "$prefix"* ]] || return 1
    echo "${remote_head_ref#$prefix}"
}

function _branch_from_remote_symref_output() {
    local line="$1"
    local prefix="ref: refs/heads/"
    local suffix=$'\tHEAD'

    [[ "$line" == "$prefix"*"$suffix" ]] || return 1
    line="${line#$prefix}"
    echo "${line%$suffix}"
}

function _get_default_branch_from_remote() {
    local remote="${1:-origin}"
    local line
    local branch_candidate

    while IFS= read -r line; do
        if branch_candidate=$(_branch_from_remote_symref_output "$line"); then
            echo "$branch_candidate"
            return 0
        fi
    done < <(git ls-remote --symref "$remote" HEAD 2>/dev/null)

    return 1
}

function _set_remote_head_ref() {
    local remote="${1:-origin}"
    local branch="$2"

    [[ -n "$branch" ]] || return 1
    git symbolic-ref "refs/remotes/$remote/HEAD" "refs/remotes/$remote/$branch" >/dev/null 2>&1 || true
}

function _get_default_branch() {
    local remote="${1:-origin}"
    local remote_head_ref
    local branch_candidate

    # Prefer the remote's current HEAD. Local refs/remotes/<remote>/HEAD can be
    # stale after a default-branch rename (for example master -> main), and git
    # fetch/prune does not update it automatically.
    if branch_candidate=$(_get_default_branch_from_remote "$remote"); then
        _set_remote_head_ref "$remote" "$branch_candidate"
        echo "$branch_candidate"
        return 0
    fi

    # Fall back to local remote HEAD only when its target exists locally.
    remote_head_ref=$(git symbolic-ref --quiet "refs/remotes/$remote/HEAD" 2>/dev/null)
    if [[ -n "$remote_head_ref" ]]; then
        if branch_candidate=$(_branch_from_remote_head_ref "$remote" "$remote_head_ref"); then
            if git show-ref --verify --quiet "refs/remotes/$remote/$branch_candidate"; then
                echo "$branch_candidate"
                return 0
            fi
        fi
    fi

    # Fall back to checking common default branch names locally.
    for branch_candidate in main master; do
        if git show-ref --verify --quiet "refs/remotes/$remote/$branch_candidate"; then
            _set_remote_head_ref "$remote" "$branch_candidate"
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

    local wt_output wt_line
    local -A checked_out=()
    local -a deletable=()
    wt_output=$(git worktree list --porcelain) || return $?
    for wt_line in "${(@f)wt_output}"; do
        if [[ "$wt_line" == branch\ refs/heads/* ]]; then
            checked_out[${wt_line#branch refs/heads/}]=1
        fi
    done
    for branch in "${targets[@]}"; do
        if [[ -n "${checked_out[$branch]-}" ]]; then
            printf 'keeping branch %s (checked out in a worktree)\n' "$branch"
        else
            deletable+=("$branch")
        fi
    done
    targets=("${deletable[@]}")
    if (( ${#targets[@]} == 0 )); then
        printf 'no unused branches to prune\n'
        return 0
    fi

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

# Origin-named aliases for non-reset helpers. Reset is a standalone Rust CLI.
function rebase_on_origin()             { rebase_on_remote_default "$@"; }
function restore_from_origin()          { restore_from_remote_default "$@"; }
function prune_all_except_origin()      { prune_all_except_remote_default "$@"; }
function merge_from_origin()            { merge_from_remote_default "$@"; }
function soft_reset_origin()            { soft_reset_remote_default "$@"; }
