import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch
import publish_feature
from publish_feature import allowed

class CandidatePolicyTests(unittest.TestCase):
    def test_only_application_source_is_allowed(self):
        for path in ('src/bin/sim_gui.rs', 'src/game_logic/combat.rs', 'web/requests.js', 'data/weapons.json'):
            self.assertTrue(allowed(path), path)

    def test_paths_cannot_escape_or_change_privileged_execution(self):
        for path in ('../src/main.rs', '/src/main.rs', 'src/../../build.rs', 'src/.git/config.rs',
                     'src/a\\b.rs', 'src/a\nb.rs', 'web/_worker.js', 'web/_routes.json', 'web/_headers',
                     'Cargo.toml', 'build.rs', 'scripts/build_web.py', '.github/workflows/web.yml',
                     'server/requests.mjs', '.codex/AGENTS.md', 'web/nested/index.html'):
            self.assertFalse(allowed(path), path)

class PublisherTests(unittest.TestCase):
    def setUp(self):
        self.original_cwd = Path.cwd()
        scratch = Path(__file__).resolve().parents[1] / 'target' / 'policy-tests'
        scratch.mkdir(parents=True, exist_ok=True)
        self.temporary = tempfile.TemporaryDirectory(dir=scratch)
        os.chdir(self.temporary.name)
        subprocess.run(['git', 'init', '-q'], check=True)
        Path('src').mkdir()
        Path('src/example.rs').write_text('fn main() {}\n')
        Path('.github/workflows').mkdir(parents=True)
        Path('.github/workflows/example.yml').write_text('name: existing\n')
        subprocess.run(['git', 'add', '.'], check=True)
        subprocess.run(['git', '-c', 'user.name=Test', '-c', 'user.email=test@example.invalid',
                        'commit', '-qm', 'Initial'], check=True)
        Path('target/candidate').mkdir(parents=True)
        Path('target/candidate/result.json').write_text(json.dumps({'status': 'implemented', 'summary': 'Test change'}))
        self.environment = patch.dict(os.environ, {'ISSUE_NUMBER': '7', 'GITHUB_OUTPUT': str(Path('target/output').resolve())})
        self.environment.start()

    def tearDown(self):
        self.environment.stop()
        os.chdir(self.original_cwd)
        self.temporary.cleanup()

    def candidate(self):
        data = subprocess.check_output(['git', 'diff', '--binary', 'HEAD'])
        Path('target/candidate/changes.patch').write_bytes(data)
        subprocess.run(['git', 'restore', '--worktree', '.'], check=True)

    def test_valid_patch_creates_pr_without_executing_candidate(self):
        Path('src/example.rs').write_text('fn main() { println!("new"); }\n')
        self.candidate()
        actual_git = publish_feature.git
        def fake_push(*args, **kwargs):
            if args[0] == 'push':
                return b''
            return actual_git(*args, **kwargs)
        with patch.object(publish_feature, 'git', side_effect=fake_push), patch.object(publish_feature, 'api') as api:
            api.side_effect = [{'title': '[Request] Example'}, {'html_url': 'https://github.com/hlyall01/hackmaster-sim/pull/8', 'number': 8}]
            publish_feature.main()
            self.assertEqual(api.call_args.args[:2], ('/pulls', 'POST'))
            self.assertTrue(api.call_args.args[2]['draft'])
        self.assertIn('changed=true', Path('target/output').read_text())
        self.assertIn('println!', Path('src/example.rs').read_text())

    def test_workflow_change_is_rejected_before_network_calls(self):
        Path('.github/workflows/example.yml').write_text('name: unsafe\n')
        self.candidate()
        with patch.object(publish_feature, 'api') as api, self.assertRaises(ValueError):
            publish_feature.main()
        api.assert_not_called()

    def test_executable_source_file_is_rejected(self):
        Path('src/example.rs').chmod(0o755)
        self.candidate()
        with patch.object(publish_feature, 'api') as api, self.assertRaises(ValueError):
            publish_feature.main()
        api.assert_not_called()

    def test_symlink_source_file_is_rejected(self):
        Path('src/example.rs').unlink()
        Path('src/example.rs').symlink_to('../.git/config')
        self.candidate()
        with patch.object(publish_feature, 'api') as api, self.assertRaises(ValueError):
            publish_feature.main()
        api.assert_not_called()

if __name__ == '__main__':
    unittest.main()
