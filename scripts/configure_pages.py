#!/usr/bin/env python3
"""Provision Pages projects and synchronize production bindings without printing secrets."""
import json
import os
import re
import urllib.error
import urllib.request

account = os.environ['CLOUDFLARE_ACCOUNT_ID']
project = os.environ['CLOUDFLARE_PAGES_PROJECT']
if not re.fullmatch('[a-f0-9]{32}', account) or not re.fullmatch('[a-z0-9-]{2,58}', project):
    raise SystemExit('Invalid Cloudflare account or project')
base = f'https://api.cloudflare.com/client/v4/accounts/{account}/pages/projects'
headers = {'Authorization': 'Bearer ' + os.environ['CLOUDFLARE_API_TOKEN'], 'Content-Type': 'application/json'}

def api(path, method='GET', body=None):
    request = urllib.request.Request(base + path, headers=headers, method=method,
                                     data=json.dumps(body).encode() if body is not None else None)
    try:
        with urllib.request.urlopen(request, timeout=30) as response:
            result = json.load(response)
    except urllib.error.HTTPError as error:
        if error.code == 404 and method == 'GET':
            return None
        raise SystemExit(f'Cloudflare operation failed: HTTP {error.code}') from None
    if not result.get('success'):
        raise SystemExit('Cloudflare operation failed')
    return result['result']

existing = api('/' + project)
if existing is None:
    existing = api('', 'POST', {'name': project, 'production_branch': 'main'})
if existing['production_branch'] != 'main':
    raise SystemExit('Pages production branch must be main')
if project == 'hackmaster-sim':
    # Preserve the old token's bytes solely as an HMAC key for existing tickets.
    # GitHub authentication now uses the app; token expiry does not affect HMAC.
    signing_secret = os.environ.get('ISSUES_TOKEN')
    private_key = os.environ.get('ISSUES_APP_PRIVATE_KEY', '')
    app_id = os.environ.get('ISSUES_APP_ID', '')
    installation_id = os.environ.get('ISSUES_APP_INSTALLATION_ID', '')
    if not signing_secret or not private_key.startswith('-----BEGIN PRIVATE KEY-----'):
        raise SystemExit('Missing request signing secret or PKCS#8 app private key')
    if not re.fullmatch('[1-9][0-9]*', app_id) or not re.fullmatch('[1-9][0-9]*', installation_id):
        raise SystemExit('Invalid GitHub App or installation ID')
    api('/' + project, 'PATCH', {'deployment_configs': {'production': {
        'compatibility_date': '2026-09-26',
        'env_vars': {
            'GITHUB_ISSUES_TOKEN': None,
            'REQUEST_SIGNING_SECRET': {'type': 'secret_text', 'value': signing_secret},
            'GITHUB_APP_PRIVATE_KEY': {'type': 'secret_text', 'value': private_key},
            'GITHUB_APP_ID': {'type': 'plain_text', 'value': app_id},
            'GITHUB_APP_INSTALLATION_ID': {'type': 'plain_text', 'value': installation_id},
            'FEATURE_REQUESTS_ENABLED': {'type': 'plain_text', 'value': os.environ.get('FEATURE_REQUESTS_ENABLED', 'false')},
        },
    }}})
print('Configured Pages project:', project)
