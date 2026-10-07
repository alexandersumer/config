"""Explicit, local-only policy for remote branch case collisions.

Called inside the supervisor's repository lock and attempt deadline.
"""

from collections import defaultdict
import hashlib
import fnmatch
import os
from pathlib import Path
import tempfile
import subprocess
import sys
import time


def git(*args, optional=False):
    result = subprocess.run(['git', *args], stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    if result.returncode and not (optional and result.returncode == 1):
        raise RuntimeError(result.stderr.decode(errors='replace').strip())
    return result.stdout.decode(errors='surrogateescape').splitlines()


def collision_exclusions(branches, protected):
    groups = defaultdict(list)
    for branch in branches:
        groups[branch.casefold()].append(branch)
    excluded = []
    for group in groups.values():
        if len(group) < 2:
            continue
        required = set(group) & protected
        if len(required) > 1:
            raise ValueError('Case collision involves multiple protected branches: ' + ', '.join(sorted(required)))
        keep = next(iter(required)) if required else min(group, key=lambda name: (name != name.lower(), name))
        excluded.extend(name for name in group if name != keep)
    return sorted(excluded)


def needs_case_recovery():
    if git('rev-parse', '--show-ref-format') == ['reftable']:
        return False
    common = Path(git('rev-parse', '--git-common-dir')[0])
    with tempfile.NamedTemporaryFile(dir=common, prefix='.home-reset-case-probe-') as probe:
        path = Path(probe.name)
        alternate = path.with_name(path.name.upper())
        return alternate.exists() and os.path.samefile(path, alternate)


def prepare(remote, target=None):
    specs = git('config', '--get-all', f'remote.{remote}.fetch', optional=True)
    if os.environ.get('HOME_RESET_RESOLVE_CASE_CONFLICTS') == '1' and needs_case_recovery():
        expected = f'+refs/heads/*:refs/remotes/{remote}/*'
        if [spec for spec in specs if not spec.startswith('^')] != [expected]:
            raise ValueError('Case recovery requires the standard remote branch fetch mapping; custom mappings were left unchanged.')
        advertised = git('ls-remote', '--symref', remote, 'HEAD', 'refs/heads/*')
        defaults = [line.split()[1][len('refs/heads/'):] for line in advertised if line.startswith('ref: refs/heads/') and line.endswith('\tHEAD')]
        if len(defaults) != 1:
            raise ValueError('Cannot protect the default branch: remote HEAD is ambiguous or missing.')
        branches = [line.split()[1][len('refs/heads/'):] for line in advertised if not line.startswith('ref:') and '\trefs/heads/' in line]
        protected = {defaults[0]}
        if target:
            protected.add(target)
        for line in git('for-each-ref', '--format=%(refname:strip=2)\t%(upstream)', 'refs/heads'):
            local, upstream = line.split('\t', 1)
            protected.add(local)
            prefix = f'refs/remotes/{remote}/'
            if upstream.startswith(prefix):
                protected.add(upstream[len(prefix):])
        for spec in specs:
            if spec.startswith('^'):
                blocked = sorted(branch for branch in protected & set(branches) if fnmatch.fnmatchcase('refs/heads/' + branch, spec[1:]))
                if blocked:
                    raise ValueError('Protected branch already excluded by fetch configuration: ' + ', '.join(blocked))
        # Validate every collision before changing any repository metadata.
        excluded = collision_exclusions(branches, protected)
        tips = dict(line.split('\t', 1) for line in git('for-each-ref', '--format=%(refname)\t%(objectname)', f'refs/remotes/{remote}'))
        for branch in excluded:
            ref = f'refs/remotes/{remote}/{branch}'
            tip = tips.get(ref)
            if tip:
                key = hashlib.sha256(ref.encode()).hexdigest()[:16]
                backup = f'refs/home-reset-backups/case-conflicts/{time.time_ns()}/{key}'
                git('update-ref', backup, tip, '0' * len(tip))
                print(f'Saved excluded tracking tip: {backup}', flush=True)
            spec = '^refs/heads/' + branch
            if spec not in specs:
                git('config', '--local', '--add', f'remote.{remote}.fetch', spec)
                specs.append(spec)
            if tip:
                git('update-ref', '-d', ref, tip)
    for spec in specs:
        if spec.startswith('^'):
            print('Fetch exclusion: ' + spec[1:], flush=True)


if __name__ == '__main__':
    try:
        prepare(sys.argv[1], sys.argv[2] if len(sys.argv) > 2 and sys.argv[2] else None)
    except (ValueError, RuntimeError) as error:
        print('error: ' + str(error), file=sys.stderr)
        sys.exit(1)
