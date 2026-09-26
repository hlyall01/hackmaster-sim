import unittest
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

if __name__ == '__main__':
    unittest.main()
