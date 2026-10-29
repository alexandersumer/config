#!/usr/bin/env python3
"""External auth CLI simulator; workctl and its process supervision are real.

Matches the installed bb/TWG/Atlas command/output contracts. CI never gets real
credentials. Live provider validation is a separate explicit lane.
"""
import atexit
import fcntl
import base64
import json
import os
import pathlib
import subprocess
import sys
import time

root = pathlib.Path(os.environ['AUTH_FIXTURE'])
state_path = root / 'state.json'
state = json.loads(state_path.read_text())
name = pathlib.Path(sys.argv[0]).name
args = sys.argv[1:]
# Each provider owns its state, just as the installed CLIs do. Base test
# configuration is immutable while providers run; no shared read/modify/write.
provider_state = root / (name + '-state.json')
if provider_state.exists():
    state = json.loads(provider_state.read_text())
log = os.open(root / 'calls.jsonl', os.O_WRONLY | os.O_CREAT | os.O_APPEND, 0o600)
os.write(log, (json.dumps([name, args]) + '\n').encode())
os.close(log)
if name in ['klist', 'kinit']:
    key = name + '-' + ('test' if args else 'list')
elif name == 'atlas':
    audience = args[args.index('--aud') + 1]
    key = 'atlas-' + ('cache-' if '--cached' in args else 'force-') + audience
else:
    key = '-'.join([name] + (args[:1] if args[0] == 'doctor' else args[:2]))

# Fail on forbidden overlap rather than relying on elapsed-time guesses.
if state.get('assert_serial'):
    provider_lock = (root / (name + '-active.lock')).open('a')
    try:
        fcntl.flock(provider_lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
    except BlockingIOError:
        (root / 'provider-overlap').touch()
        sys.exit(97)
interactive = (name == 'bb' and args[:2] == ['auth', 'login']) or (name == 'atlas' and '--force' in args)
if interactive and state.get('assert_interaction'):
    interaction_lock = (root / 'interaction-active.lock').open('a')
    try:
        fcntl.flock(interaction_lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
    except BlockingIOError:
        (root / 'interaction-overlap').touch()
        sys.exit(98)
if key in state.get('hang_keys', []):
    descendant = subprocess.Popen(['/bin/sleep', '30'])
    (root / ('descendant-' + key)).write_text(str(descendant.pid))
(root / ('started-' + key)).touch()
atexit.register(lambda: (root / ('finished-' + key)).touch())
if key in state.get('hold_keys', []) or key in state.get('hang_keys', []):
    while not (root / ('release-' + key)).exists():
        time.sleep(.01)
secret = 'AUTH_SECRET_MUST_NEVER_APPEAR'
mode = state.get('mode')
if name == 'twg' and state.get('twg_delay'):
    time.sleep(state['twg_delay'])
if mode == 'hang':
    child = subprocess.Popen(['/bin/sleep', '30'])
    (root / 'descendant').write_text(str(child.pid))
    (root / 'started').touch()
    time.sleep(30)
if mode == 'leader_exit':
    child = subprocess.Popen(['/bin/sleep', '30'])
    (root / 'descendant').write_text(str(child.pid))
if mode == 'flood':
    print(secret * 20000)
    sys.exit(0)
if mode == 'outage':
    print('service unavailable ' + secret, file=sys.stderr)
    sys.exit(1)
if mode == 'permission':
    print('credential store: permission denied ' + secret, file=sys.stderr)
    sys.exit(1)
if mode == 'malformed':
    print(secret + '\x1b[31munsupported response')
    sys.exit(0)

def save():
    provider_state.write_text(json.dumps(state))

def jwt():
    audience = args[args.index('--aud') + 1]
    claims = {'exp': int(time.time()) + state.get('ttl', 3600),
              'aud': state.get('aud', audience)}
    if 'nbf' in state:
        claims['nbf'] = state['nbf']
    def part(obj):
        return base64.urlsafe_b64encode(json.dumps(obj).encode()).decode().rstrip('=')
    return part({'alg': 'RS256'}) + '.' + part(claims) + '.fixture-signature'

if name == 'bb':
    if args[:2] == ['workspace', 'list']:
        assert '--json' in args
        time.sleep(state.get('bb_api_delay', 0))
        if state.get('bb_revoked'):
            print('Authentication failed (401) ' + secret, file=sys.stderr)
            sys.exit(1)
        if state.get('bb_api_outage'):
            print('Service unavailable ' + secret, file=sys.stderr)
            sys.exit(1)
        print('[]')
    elif args[:2] == ['auth', 'status']:
        if not state.get('bb_valid', True):
            print('Not logged in ' + secret, file=sys.stderr)
            sys.exit(1)
        account = args[args.index('--account') + 1] if '--account' in args else 'office'
        marker = '' if state.get('bb_inactive') else ' (active)'
        lifetime = str(state['bb_seconds']) + 's' if 'bb_seconds' in state else str(state.get('bb_minutes', 60)) + ' min'
        print('✓ ' + account + marker + '\ttest@example.com\tOAuth\texpires in ' + lifetime)
        if state.get('bb_other_active'):
            print('✓ other (active)\ttest@example.com\tOAuth\texpires in 60 min')
    elif args[:2] == ['auth', 'token']:
        if state.get('bb_silent', False):
            state['bb_valid'] = True
            state['bb_minutes'] = 60
            save()
        if not state.get('bb_valid', True):
            print('Authentication failed ' + secret, file=sys.stderr)
            sys.exit(1)
        print(secret)
    elif args[:2] == ['auth', 'login']:
        if state.get('refresh_fails'):
            print(secret, file=sys.stderr)
            sys.exit(1)
        state['bb_valid'] = True
        state['bb_minutes'] = 120
        state['bb_revoked'] = False
        save()
        print('Logged in ' + secret)
    else:
        raise RuntimeError('Unexpected bb command')
elif name == 'twg':
    assert '-o' in args and args[args.index('-o') + 1] == 'json'
    assert '--output-summary' in args and args[args.index('--output-summary') + 1] == 'none'
    assert os.environ['TWG_AGENT_DEFAULTS'] == '0'
    if args[0] == 'doctor':
        print(json.dumps({'data': {'connectivity': {'ok': state.get('twg_valid', True)},
                                 'auth': {'resolved': {'tokenPresent': state.get('twg_valid', True)}},
                                 'bitbucket': state.get('twg_bb_response', {'attempted': True, 'ok': state.get('twg_bb_valid', True),
                                               'status': state.get('twg_bb_status', 'valid' if state.get('twg_bb_valid', True) else 'invalid-or-expired'),
                                               'username': 'fixture-user'})}}))
        sys.exit(0 if state.get('twg_bb_valid', True) else 1)
    elif args[:2] == ['auth', 'refresh']:
        assert '--force' in args
        if state.get('refresh_fails'):
            print('Authentication failed ' + secret, file=sys.stderr)
            sys.exit(1)
        if not state.get('refresh_stale'):
            state['twg_valid'] = True
            save()
        print(json.dumps({'data': {'status': state.get('twg_refresh_status', 'refreshed'), 'secret': secret}}))
    else:
        raise RuntimeError('Unexpected TWG command')
elif name == 'atlas':
    assert args[:2] == ['slauth', 'token']
    assert '--output' in args and args[args.index('--output') + 1] == 'jwt'
    audience = args[args.index('--aud') + 1]
    valid = state.get('atlas_valid_by_audience', {}).get(audience, state.get('atlas_valid', True))
    if '--cached' in args:
        if not valid:
            print('ERROR: No existing token entry in cache ' + secret, file=sys.stderr)
            sys.exit(2)
        print(jwt())
    else:
        assert '--force' in args and '--cached' not in args
        if state.get('refresh_outage'):
            print('Service unavailable ' + secret, file=sys.stderr)
            sys.exit(1)
        if state.get('refresh_denied'):
            print('Forbidden: group access denied ' + secret, file=sys.stderr)
            sys.exit(1)
        if state.get('prompt'):
            print(secret, file=sys.stderr, flush=True)
            print('https://atlassian.okta.com/activate?user_code=ABCD1234', flush=True)
            (root / 'started').touch()
            while not (root / 'release').exists():
                time.sleep(.01)
        if state.get('refresh_fails'):
            print('MFA required ' + secret, file=sys.stderr)
            sys.exit(1)
        if not state.get('refresh_stale'):
            state.setdefault('atlas_valid_by_audience', {})[audience] = True
            state['ttl'] = state.get('refreshed_ttl', 3600)
            save()
        if state.get('refresh_cache_then_fail'):
            print('MFA required ' + secret, file=sys.stderr)
            sys.exit(1)
        if state.get('refresh_hang'):
            (root / 'started').touch()
            time.sleep(30)
        print(jwt())
    if mode == 'leader_exit':
        sys.exit(0)
elif name == 'klist':
    if args:
        assert args in [['-s'], ['-t']]
        sys.exit(0 if state.get('kerberos_valid', True) else 1)
    if state.get('kerberos_cache_missing'):
        print('No credentials cache found ' + secret, file=sys.stderr)
        sys.exit(1)
    print('Principal: ' + state.get('kerberos_principal', 'fixture@EXAMPLE.COM'))
elif name == 'kinit':
    assert args == ['-R']
    if state.get('refresh_fails'):
        print('Ticket expired ' + secret, file=sys.stderr)
        sys.exit(1)
    state['kerberos_valid'] = True
    # klist and kinit share a credential cache.
    (root / 'klist-state.json').write_text(json.dumps(state))
else:
    raise RuntimeError('Unknown simulator')
