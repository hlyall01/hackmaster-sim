#!/usr/bin/env python3
"""Validate an untrusted agent patch, then create a branch and draft PR without executing it."""
import json
import os
from pathlib import Path, PurePosixPath
import re
import subprocess
import urllib.request

REPO = 'hlyall01/hackmaster-sim'

def allowed(path):
    p = PurePosixPath(path)
    if not re.fullmatch(r'[A-Za-z0-9_./-]+', path) or '..' in p.parts or path.startswith('/'):
        return False
    if any(part.startswith('.') for part in p.parts):
        return False
    return ((path.startswith('src/') and p.suffix == '.rs') or
            (path.startswith('data/') and p.suffix == '.json') or
            (len(p.parts) == 2 and p.parts[0] == 'web' and not p.name.startswith('_') and p.suffix in ('.js', '.css', '.html')))

def git(*args, **kwargs):
    return subprocess.check_output(['git', '-c', 'core.hooksPath=/dev/null', *args], **kwargs)

def api(path, method='GET', body=None):
    request = urllib.request.Request('https://api.github.com/repos/' + REPO + path,
        data=json.dumps(body).encode() if body is not None else None, method=method,
        headers={'Authorization': 'Bearer ' + os.environ['GH_TOKEN'], 'Accept': 'application/vnd.github+json',
                 'Content-Type': 'application/json', 'X-GitHub-Api-Version': '2022-11-28'})
    with urllib.request.urlopen(request, timeout=30) as response:
        return json.load(response)

def output(name, value):
    if '\n' in str(value):
        raise ValueError('Invalid output')
    with open(os.environ['GITHUB_OUTPUT'], 'a') as stream:
        stream.write(f'{name}={value}\n')

def main():
    number = os.environ['ISSUE_NUMBER']
    if not re.fullmatch('[1-9][0-9]{0,8}', number):
        raise ValueError('Invalid issue number')
    patch = Path('target/candidate/changes.patch')
    result_path = Path('target/candidate/result.json')
    if patch.is_symlink() or result_path.is_symlink() or patch.stat().st_size > 500_000 or result_path.stat().st_size > 20_000:
        raise ValueError('Candidate output exceeds allowed size or type')
    result = json.loads(result_path.read_text())
    if result.get('status') != 'implemented' or patch.stat().st_size == 0:
        output('changed', 'false')
        summary = str(result.get('summary', 'No implementation produced.'))[:5000]
        api(f'/issues/{number}/comments', 'POST', {'body': 'Agent follow-up (untrusted generated text):\n\n' + summary})
        return
    # Validate names before applying and inspect file modes afterwards; reject symlinks and submodules.
    stats = git('apply', '--numstat', '-z', str(patch)).split(b'\0')
    paths = []
    for row in filter(None, stats):
        fields = row.decode('utf-8').split('\t')
        if len(fields) != 3 or fields[0] == '-' or not allowed(fields[2]):
            raise ValueError('Patch modifies a protected path or binary file')
        paths.append(fields[2])
    if not paths or len(paths) > 40:
        raise ValueError('Patch must change between 1 and 40 source files')
    git('apply', '--check', '--index', str(patch))
    git('apply', '--index', str(patch))
    changed = git('diff', '--cached', '--name-only', '-z').decode().split('\0')
    if any(not allowed(path) for path in changed if path):
        raise ValueError('Applied patch contains a protected path')
    for row in filter(None, git('ls-files', '--stage', '-z', '--', *paths).split(b'\0')):
        if not row.startswith(b'100644 '):
            raise ValueError('Only regular, non-executable source files are allowed')
    for path in paths:
        p = Path(path)
        if p.exists() and (p.is_symlink() or p.stat().st_size > 1_000_000):
            raise ValueError('Invalid candidate file')
    branch = f'codex/request-{number}'
    git('config', 'user.name', 'github-actions[bot]')
    git('config', 'user.email', '41898282+github-actions[bot]@users.noreply.github.com')
    git('checkout', '-b', branch)
    git('commit', '-m', f'Implement feature request #{number}')
    # The token is supplied by a one-shot askpass script, never embedded in a remote URL or config.
    git('push', 'origin', f'HEAD:refs/heads/{branch}')
    sha = git('rev-parse', 'HEAD').decode().strip()
    issue = api(f'/issues/{number}')
    summary = str(result.get('summary', 'Implementation proposed by the coding agent.'))[:6000]
    pr = api('/pulls', 'POST', {'head': branch, 'base': 'main', 'draft': True,
        'title': issue['title'].removeprefix('[Request] ')[:200],
        'body': f'Fixes #{number}\n\n{summary}\n\nPreview and build status: https://feature.sim-gui.com/{number}\n\n'
                'Generated from a public feature request. Review the code and passing checks before merging. '
                'Preview builds run automatically; production updates only after a merge into main.'})
    output('changed', 'true')
    output('sha', sha)
    output('pr', pr['html_url'])
    output('pr_number', pr['number'])

if __name__ == '__main__':
    main()
