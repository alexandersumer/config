# shellcheck shell=zsh
# Git helper functions used across interactive shells.

function _get_default_branch() {
    local remote="${1:-origin}"
    local remote_head_ref
    local branch_candidate

    # First try to get from remote HEAD
    remote_head_ref=$(git symbolic-ref --quiet "refs/remotes/$remote/HEAD" 2>/dev/null)
    if [[ -n "$remote_head_ref" ]]; then
        echo "${remote_head_ref##*/}"
        return 0
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

function origin_reset_hard() {
    local remote="origin"
    local branch=""
    local arg
    local -a positional_args=()
    local fetch_output
    local fetch_status=0
    local refs_to_delete
    local stale_output
    local ref
    local remote_head_ref
    local branch_candidate
    local upstream_ref
    local -a stale_fetch_refs=()
    local -a manual_deleted_refs=()
    local -a refs_array=()
    local -a branch_candidates=()
    local -i cleanup_rounds=0
    local -i max_cleanup_rounds=3
    local -i branch_from_arg=0

    for arg in "$@"; do
        case "$arg" in
            --help|-h)
                printf 'usage: origin_reset_hard [remote] [branch]\n'
                return 0
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
    git_dir=$(git rev-parse --git-dir 2>/dev/null)
    if [[ -z "$git_dir" ]]; then
        printf '\033[31merror: not in a git repository\033[0m\n' >&2
        return 1
    fi

    if ! git remote get-url "$remote" >/dev/null 2>&1; then
        printf '\033[31merror: remote %s is not configured\033[0m\n' "$remote" >&2
        return 1
    fi

    # Remove loose remote tracking ref subdirectories before fetch. This
    # prevents stale lock files and case-conflicting directories from blocking
    # git fetch. Preserves flat files (e.g. HEAD) since git fetch doesn't
    # recreate the HEAD symbolic ref. Packed refs are unaffected.
    if [[ -d "$git_dir/refs/remotes/$remote" ]]; then
        find "$git_dir/refs/remotes/$remote" -mindepth 1 -maxdepth 1 -type d -exec rm -rf {} + 2>/dev/null
        find "$git_dir/refs/remotes/$remote" -name "*.lock" -delete 2>/dev/null
    fi

    # Same cleanup for reflogs — D/F conflicts here also block git fetch.
    if [[ -d "$git_dir/logs/refs/remotes/$remote" ]]; then
        find "$git_dir/logs/refs/remotes/$remote" -mindepth 1 -maxdepth 1 -type d -exec rm -rf {} + 2>/dev/null
        find "$git_dir/logs/refs/remotes/$remote" -name "*.lock" -delete 2>/dev/null
    fi

    if (( branch_from_arg == 0 )) && [[ -z "$branch" ]]; then
        remote_head_ref=$(git symbolic-ref --quiet "refs/remotes/$remote/HEAD" 2>/dev/null)
        if [[ -n "$remote_head_ref" ]]; then
            branch=${remote_head_ref##*/}
        fi
    fi

    while true; do
        fetch_output=$(git fetch --prune "$remote" 2>&1)
        fetch_status=$?

        if (( fetch_status == 0 )); then
            stale_output=$(FETCH_OUTPUT="$fetch_output" REMOTE_NAME="$remote" python3 - <<'PYFETCH'
import os
import re

text = os.environ["FETCH_OUTPUT"]
remote = re.escape(os.environ["REMOTE_NAME"])
pattern = re.compile(r"removing stale tracking ref (refs/remotes/%s/[^\s\"']+)" % remote)
seen = []
for ref in pattern.findall(text):
    if ref not in seen:
        seen.append(ref)
if seen:
    print("\n".join(seen))
PYFETCH
)
            if [[ -n "$stale_output" ]]; then
                stale_fetch_refs=(${(f)stale_output})
            fi
            break
        fi

        # Fetch failed — bail out if we've exhausted cleanup attempts.
        if (( cleanup_rounds >= max_cleanup_rounds )); then
            printf '%s\n' "$fetch_output" >&2
            break
        fi

        printf '%s\n' "$fetch_output" >&2

        refs_to_delete=$(FETCH_OUTPUT="$fetch_output" REMOTE_NAME="$remote" python3 - <<'PYDELETE'
import os
import re

text = os.environ["FETCH_OUTPUT"]
remote = re.escape(os.environ["REMOTE_NAME"])
patterns = [
    r"cannot lock ref '(refs/remotes/%s/[^']+)'",
    r"cannot update the ref '(refs/remotes/%s/[^']+)'",
    r"removing stale tracking ref (refs/remotes/%s/[^\s\"']+)",
    r"(refs/remotes/%s/[^\s']+): is at [0-9a-f]+ but expected [0-9a-f]+",
]
seen = []
for raw in patterns:
    pattern = re.compile(raw % remote)
    for ref in pattern.findall(text):
        if ref not in seen:
            seen.append(ref)
if seen:
    print("\n".join(seen))
PYDELETE
)
        if [[ -z "$refs_to_delete" ]]; then
            break
        fi

        refs_array=(${(f)refs_to_delete})
        if (( ${#refs_array[@]} == 0 )); then
            break
        fi

        for ref in "${refs_array[@]}"; do
            local ref_path="$git_dir/$ref"
            local log_path="$git_dir/logs/$ref"
            rm -f "${ref_path}.lock" 2>/dev/null
            if [[ -f "$ref_path" ]]; then
                printf '\033[33mwarning: removing stale ref %s\033[0m\n' "$ref"
                rm -f "$ref_path"
                manual_deleted_refs+=("$ref")
            elif [[ -d "$ref_path" ]]; then
                printf '\033[33mwarning: removing stale ref directory %s\033[0m\n' "$ref"
                rm -rf "$ref_path"
                manual_deleted_refs+=("$ref")
            fi
            # Clean corresponding reflog entry
            rm -f "${log_path}.lock" "$log_path" 2>/dev/null
            [[ -d "$log_path" ]] && rm -rf "$log_path"
            # Resolve D/F conflicts: a parent of the failing ref may
            # exist as a file (old branch) when a child path (new
            # branch) needs it to be a directory.
            local _p _dir _stop
            for _p in "$ref_path" "$log_path"; do
                _dir="${_p%/*}"
                _stop="$git_dir/refs/remotes/$remote"
                [[ "$_p" == "$log_path" ]] && _stop="$git_dir/logs/refs/remotes/$remote"
                while [[ "$_dir" != "$_stop" && "$_dir" == "${_stop}/"* ]]; do
                    if [[ -f "$_dir" ]]; then
                        printf '\033[33mwarning: removing file blocking directory %s\033[0m\n' "${_dir#$git_dir/}"
                        rm -f "$_dir"
                    fi
                    _dir="${_dir%/*}"
                done
            done
        done

        (( cleanup_rounds++ ))
    done

    if (( fetch_status != 0 )); then
        printf '\033[31merror: git fetch failed after cleanup attempts\033[0m\n' >&2
        return $fetch_status
    fi

    if (( branch_from_arg == 0 )); then
        remote_head_ref=$(git symbolic-ref --quiet "refs/remotes/$remote/HEAD" 2>/dev/null)
        if [[ -n "$remote_head_ref" ]]; then
            branch=${remote_head_ref##*/}
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

    if [[ -z "$branch" ]]; then
        printf '\033[31merror: unable to determine default branch for %s\033[0m\n' "$remote" >&2
        return 1
    fi

    if ! git show-ref --verify --quiet "refs/remotes/$remote/$branch"; then
        printf '\033[31merror: remote %s does not have branch %s\033[0m\n' "$remote" "$branch" >&2
        return 1
    fi

    printf 'resetting to %s/%s: \033[32mgit fetch --prune %s && git switch --force %s && git reset --hard %s/%s\033[0m\n' \
        "$remote" "$branch" "$remote" "$branch" "$remote" "$branch"

    if (( ${#manual_deleted_refs[@]} > 0 )); then
        printf '\033[33mmanually removed stale tracking refs:\033[0m\n'
        for ref in "${manual_deleted_refs[@]}"; do
            printf '  %s\n' "$ref"
        done
    fi

    if (( ${#stale_fetch_refs[@]} > 0 )); then
        printf '\033[33mgit fetch pruned stale tracking refs:\033[0m\n'
        for ref in "${stale_fetch_refs[@]}"; do
            printf '  %s\n' "$ref"
        done
    fi

    if git rev-parse --verify --quiet "refs/heads/$branch"; then
        git switch --force "$branch" || return $?
    else
        git switch --force-create "$branch" "$remote/$branch" || return $?
    fi

    upstream_ref=$(git rev-parse --symbolic-full-name "$branch@{upstream}" 2>/dev/null)
    if [[ "$upstream_ref" != "refs/remotes/$remote/$branch" ]]; then
        if ! git branch --set-upstream-to "$remote/$branch" "$branch" >/dev/null 2>&1; then
            printf '\033[33mwarning: unable to set upstream to %s/%s\033[0m\n' "$remote" "$branch"
        fi
    fi

    git reset --hard "$remote/$branch" || return $?
}

function rebase_on_origin() {
    local remote="origin"
    local main_branch
    local current_branch
    local git_status
    local git_dir
    local rebase_output

    git_dir=$(git rev-parse --git-dir 2>/dev/null)
    if [[ -z "$git_dir" ]]; then
        echo -e "\033[31merror: not in a git repository\033[0m"
        return 1
    fi

    current_branch=$(git rev-parse --abbrev-ref HEAD)

    if [[ "$current_branch" == "HEAD" ]]; then
        echo -e "\033[31merror: you are in a detached HEAD state\033[0m"
        echo -e "\033[33mcheckout a branch first:\033[0m \033[32mgit checkout -b <branch-name>\033[0m"
        return 1
    fi

    main_branch=$(_get_default_branch "$remote")
    if [[ -z "$main_branch" ]]; then
        echo -e "\033[31merror: could not detect main branch (tried main, master)\033[0m"
        return 1
    fi

    if [[ -d "$git_dir/rebase-merge" ]] || [[ -d "$git_dir/rebase-apply" ]]; then
        echo -e "\033[31m✗ a rebase is already in progress\033[0m"
        echo -e "\033[33moptions:\033[0m"
        echo -e "  1. continue rebase: \033[32mgit rebase --continue\033[0m"
        echo -e "  2. skip this patch: \033[32mgit rebase --skip\033[0m"
        echo -e "  3. abort rebase:    \033[32mgit rebase --abort\033[0m"
        return 1
    fi

    git_status=$(git status --porcelain 2>/dev/null)
    if [[ -n "$git_status" ]]; then
        local file_count=${#${(f)git_status}}
        echo -e "\033[31m✗ cannot rebase: you have uncommitted changes\033[0m"
        echo -e "\033[33mmodified files:\033[0m"
        printf '%s\n' "${(f)git_status}" | head -20
        if (( file_count > 20 )); then
            echo -e "\033[33m... and $((file_count - 20)) more files\033[0m"
        fi
        echo ""
        echo -e "\033[33moptions:\033[0m"
        echo -e "  1. commit your changes: \033[32mgit add -A && git commit -m 'your message'\033[0m"
        echo -e "  2. stash your changes:  \033[32mgit stash\033[0m"
        echo -e "  3. discard changes:     \033[32mgit reset --hard\033[0m (warning: this will lose changes)"
        return 1
    fi

    echo -e "\033[33mcurrent branch:\033[0m $current_branch"
    echo -e "\033[33mrebasing onto:\033[0m $remote/$main_branch"
    echo -e "fetching and rebasing: \033[32mgit fetch $remote $main_branch && git rebase $remote/$main_branch\033[0m"

    if ! git fetch "$remote" "$main_branch" 2>&1; then
        echo -e "\033[31m✗ failed to fetch $remote/$main_branch\033[0m"
        echo -e "\033[33mcheck your network connection and remote configuration\033[0m"
        return 1
    fi

    echo -e "\033[32m✓ fetched latest $main_branch\033[0m"

    rebase_output=$(git rebase "$remote/$main_branch" 2>&1)
    local rebase_exit_code=$?

    if [[ $rebase_exit_code -eq 0 ]]; then
        echo -e "\033[32m✓ successfully rebased $current_branch onto $remote/$main_branch\033[0m"
        return 0
    fi

    echo "$rebase_output"

    if echo "$rebase_output" | grep -q "error: cannot rebase: Your index contains uncommitted changes"; then
        echo -e "\033[31m✗ cannot rebase: uncommitted changes detected\033[0m"
        echo -e "\033[33mthis shouldn't happen - please report this issue\033[0m"
        echo -e "\033[33mtry:\033[0m \033[32mgit status\033[0m to see what's wrong"
    elif echo "$rebase_output" | grep -q "CONFLICT"; then
        echo -e "\033[31m✗ rebase encountered merge conflicts\033[0m"
        echo -e "\033[33mresolve conflicts in the files listed above, then:\033[0m"
        echo -e "  1. stage resolved files: \033[32mgit add <resolved-files>\033[0m"
        echo -e "  2. continue rebase:      \033[32mgit rebase --continue\033[0m"
        echo -e "  3. or abort rebase:      \033[32mgit rebase --abort\033[0m"
    elif [[ -d "$git_dir/rebase-merge" ]] || [[ -d "$git_dir/rebase-apply" ]]; then
        echo -e "\033[31m✗ rebase stopped (possibly due to conflicts)\033[0m"
        echo -e "\033[33mcheck status and resolve any issues:\033[0m"
        echo -e "  1. check status:    \033[32mgit status\033[0m"
        echo -e "  2. continue rebase: \033[32mgit rebase --continue\033[0m"
        echo -e "  3. or abort rebase: \033[32mgit rebase --abort\033[0m"
    else
        echo -e "\033[31m✗ rebase failed\033[0m"
        echo -e "\033[33mcheck the error messages above for details\033[0m"
    fi

    return 1
}

function restore_from_origin() {
    if [ $# -eq 0 ]; then
        echo -e "\033[31merror: no file path provided\033[0m"
        return 1
    fi

    if ! git rev-parse --git-dir >/dev/null 2>&1; then
        echo -e "\033[31merror: not in a git repository\033[0m"
        return 1
    fi

    local remote="origin"
    local main_branch
    main_branch=$(_get_default_branch "$remote")
    if [[ -z "$main_branch" ]]; then
        echo -e "\033[31merror: could not detect main branch (tried main, master)\033[0m"
        return 1
    fi

    local file_path_from_repository_root=$*
    local full_filename=$(basename "$file_path_from_repository_root")
    if git restore --source "$remote/$main_branch" "${file_path_from_repository_root}"; then
        echo -e "\033[32m✓ restored '${full_filename}' from $remote/${main_branch}\033[0m"
    else
        echo -e "\033[31m✗ failed to restore '${full_filename}'\033[0m"
        return 1
    fi
}

function branch_create() {
    if [ $# -eq 0 ]; then
        echo -e "\033[31merror: please provide a branch name\033[0m"
        return 1
    fi

    local branch_name=${@// /-}
    echo -e "creating branch: \033[32m${branch_name}\033[0m"
    git switch -c "${branch_name}"
}

function prune_all_except_origin() {
    local keep_branch="$1"
    local remote="origin"
    local current_branch
    local -a branches_to_delete=()

    if ! git rev-parse --git-dir >/dev/null 2>&1; then
        echo -e "\033[31merror: not in a git repository\033[0m"
        return 1
    fi

    # Auto-detect main branch if not specified
    if [[ -z "$keep_branch" ]]; then
        keep_branch=$(_get_default_branch "$remote")
        if [[ -z "$keep_branch" ]]; then
            echo -e "\033[31merror: could not detect main branch (tried main, master)\033[0m"
            return 1
        fi
    fi

    if ! git show-ref --verify --quiet "refs/heads/$keep_branch"; then
        echo -e "\033[31merror: local branch '${keep_branch}' not found\033[0m"
        echo -e "\033[33mtry:\033[0m \033[32mgit fetch $remote ${keep_branch}:${keep_branch}\033[0m"
        return 1
    fi

    current_branch=$(git rev-parse --abbrev-ref HEAD 2>/dev/null)

    if [[ "$current_branch" != "$keep_branch" ]]; then
        if git switch --quiet "$keep_branch"; then
            echo -e "switched to keep branch: \033[32m${keep_branch}\033[0m"
        else
            echo -e "\033[31merror: unable to switch to '${keep_branch}'\033[0m"
            return 1
        fi
    fi

    while IFS= read -r branch; do
        [[ "$branch" == "$keep_branch" ]] && continue
        branches_to_delete+=("$branch")
    done < <(git for-each-ref --format='%(refname:short)' refs/heads)

    if (( ${#branches_to_delete[@]} == 0 )); then
        echo -e "\033[33mno local branches to delete\033[0m"
        return 0
    fi

    prune_branch "${branches_to_delete[@]}"
    return $?
}

function prune_branch() {
    if (( $# == 0 )); then
        echo -e "\033[31merror: provide at least one branch to prune\033[0m"
        return 1
    fi

    local current_branch
    local branch
    local -a targets=()

    if ! git rev-parse --git-dir >/dev/null 2>&1; then
        echo -e "\033[31merror: not in a git repository\033[0m"
        return 1
    fi

    current_branch=$(git rev-parse --abbrev-ref HEAD 2>/dev/null)

    for branch in "$@"; do
        if [[ -z "$branch" ]]; then
            echo -e "\033[31merror: branch name cannot be empty\033[0m"
            return 1
        fi

        if [[ "$branch" == "main" || "$branch" == "master" ]]; then
            echo -e "\033[31merror: refusing to prune protected branch '${branch}'\033[0m"
            return 1
        fi

        if [[ "$branch" == "$current_branch" ]]; then
            echo -e "\033[31merror: cannot prune the current branch '${branch}'\033[0m"
            echo -e "\033[33mswitch to another branch first\033[0m"
            return 1
        fi

        if ! git show-ref --verify --quiet "refs/heads/$branch"; then
            echo -e "\033[31merror: local branch '${branch}' not found\033[0m"
            return 1
        fi

        targets+=("$branch")
    done

    if (( ${#targets[@]} == 0 )); then
        echo -e "\033[33mnothing to prune\033[0m"
        return 0
    fi

    # Remove worktrees that use any of the target branches before deleting.
    # Parse porcelain output line-by-line to handle paths with spaces.
    local wt_path="" wt_line
    while IFS= read -r wt_line; do
        if [[ "$wt_line" == worktree\ * ]]; then
            wt_path="${wt_line#worktree }"
        elif [[ "$wt_line" == branch\ refs/heads/* ]]; then
            local wt_branch="${wt_line#branch refs/heads/}"
            for branch in "${targets[@]}"; do
                if [[ "$wt_branch" == "$branch" ]]; then
                    printf '\033[33mremoving worktree using branch %s: %s\033[0m\n' "$branch" "$wt_path"
                    git worktree remove --force "$wt_path" 2>/dev/null
                    break
                fi
            done
            wt_path=""
        fi
    done < <(git worktree list --porcelain 2>/dev/null)

    # Prune stale worktree metadata
    git worktree prune 2>/dev/null

    echo -e "pruning local branches: \033[32m${(j: :)targets}\033[0m"
    if git branch -D "${targets[@]}"; then
        return 0
    fi

    echo -e "\033[31merror: failed to prune one or more branches\033[0m"
    return 1
}

function force_push() {
    echo -e "force pushing with lease: \033[32mgit push --force-with-lease\033[0m"
    git push --force-with-lease
}

function _stage_commit_git_impl() {
    local message=$1
    local skip_detekt=$2
    git add .
    if git diff --cached --quiet; then
        echo "Nothing to commit."
        return 0
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
        echo -e "\033[31merror: please provide a commit message\033[0m"
        return 1
    fi

    local message
    message=$(_build_commit_message "$@")

    local label="staging and committing"
    (( skip_detekt )) && label+=" (skip detekt)"
    (( do_push )) && label="${label/and committing/committing and pushing}"
    echo -e "${label}: \033[32m\"$message\"\033[0m"

    _stage_commit_git_impl "$message" "$skip_detekt"
    local commit_status=$?

    if [[ $commit_status -ne 0 ]]; then
        if ! git diff --quiet || ! git diff --cached --quiet; then
            echo -e "\033[33mHooks modified files, staging changes and retrying commit...\033[0m"
            git add -u
            _stage_commit_git_impl "$message" "$skip_detekt"
            commit_status=$?
        fi
    fi

    if [[ $commit_status -ne 0 ]]; then
        return $commit_status
    fi

    if (( do_push )); then
        if ! git push; then
            echo -e "\033[31m✗ push failed\033[0m"
            echo -e "\033[33moptions:\033[0m"
            echo -e "  1. pull and retry:  \033[32mgit pull --rebase && git push\033[0m"
            echo -e "  2. force push:      \033[32mforce_push\033[0m (use with caution)"
            return 1
        fi
    fi
}

function gsc()       { _gsc_impl 0 0 "$@"; }
function fast_gsc()  { _gsc_impl 1 0 "$@"; }
function gscp()      { _gsc_impl 0 1 "$@"; }
function fast_gscp() { _gsc_impl 1 1 "$@"; }

function merge_from_origin() {
    local remote="origin"
    local main_branch

    main_branch=$(_get_default_branch "$remote")
    if [[ -z "$main_branch" ]]; then
        echo -e "\033[31merror: could not detect main branch (tried main, master)\033[0m"
        return 1
    fi

    echo -e "fetching and merging: \033[32mgit fetch $remote $main_branch && git merge $remote/$main_branch\033[0m"
    git fetch "$remote" "$main_branch" && git merge "$remote/$main_branch"
}

function quit_merge() {
    echo -e "quitting merge: \033[32mgit merge --quit\033[0m"
    git merge --quit
}

function abort_merge() {
    echo -e "aborting merge: \033[32mgit merge --abort\033[0m"
    git merge --abort
}

function hard_reset_head() {
    if ! git rev-parse --git-dir > /dev/null 2>&1; then
        echo -e "\033[31merror: not in a git repository\033[0m"
        return 1
    fi
    echo -e "hard reset to HEAD: \033[32mgit reset --hard\033[0m"
    git reset --hard
}

function soft_reset_origin() {
    if ! git rev-parse --git-dir > /dev/null 2>&1; then
        echo -e "\033[31merror: not in a git repository\033[0m"
        return 1
    fi

    local remote="origin"
    local main_branch
    main_branch=$(_get_default_branch "$remote")
    if [[ -z "$main_branch" ]]; then
        echo -e "\033[31merror: could not detect main branch (tried main, master)\033[0m"
        return 1
    fi

    if ! git show-ref --verify --quiet "refs/heads/$main_branch"; then
        echo -e "\033[31merror: local '${main_branch}' branch not found\033[0m"
        echo -e "\033[33mtry:\033[0m \033[32mgit fetch $remote ${main_branch}:${main_branch}\033[0m"
        return 1
    fi
    echo -e "soft reset onto ${main_branch}: \033[32mgit reset --soft ${main_branch}\033[0m"
    git reset --soft "$main_branch"
}
