#!/usr/bin/env python3
"""External auth CLI simulator; workctl and its process supervision are real.

Matches the installed bb/TWG/Atlas command/output contracts. CI never gets real
credentials. Live provider validation is a separate explicit lane.
"""
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
with (root / 'calls.jsonl').open('a') as log:
    log.write(json.dumps([name, args]) + '\n')
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
    state_path.write_text(json.dumps(state))

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
                                 'bitbucket': {'status': 'invalid-or-expired'}}}))
        sys.exit(1)  # unrelated Bitbucket credential failure must not invalidate OAuth
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
else:
    raise RuntimeError('Unknown simulator')
