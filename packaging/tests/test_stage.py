# 搭安装包的内容（packaging/stage.py）：拿替身的核心、网页检出整趟跑一遍，看页面文件带全没有。
#   python3 -m unittest discover -s packaging/tests
import pathlib, subprocess, sys, tempfile, unittest

ROOT = pathlib.Path(__file__).resolve().parent.parent.parent
TRIPLE = 'x86_64-unknown-linux-gnu'
# 网页仓库 tools/pack-web.py 里列的页面（照网页 origin/main 的那一份）
PAGES = ['index.html', 'manifest.webmanifest', 'distro.json', 'src', 'styles', 'packages', 'resources', 'vendor']


def write(path, text=''):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding='utf-8')


def core(root):
    write(root / 'resources' / 'README.md')
    for name in ('miyu', 'miyu-sandbox', 'miyu-extract', 'miyu-watchdog'):
        write(root / 'target' / 'release' / name)
    write(root / 'LICENSE')
    write(root / 'LICENSES' / 'Apache-2.0.txt')
    write(root / 'THIRD-PARTY-LICENSES.txt')


def web(root, pages):
    write(root / 'Cargo.toml', '[workspace.package]\nversion = "0.1.0"\n')
    write(root / 'webui' / 'package.toml', '[package]\nid = "webui"\nversion = "0.1.0"\n')
    write(root / 'webui' / 'web.json', '{}')
    write(root / 'webui' / 'matugen' / 'config.toml')
    write(root / 'webui' / 'THIRD-PARTY-LICENSES.txt')
    write(root / 'LICENSE')
    write(root / 'target' / 'release' / 'miyu-web')
    write(root / 'tools' / 'pack-web.py', f'import sys\nPAGES = {pages!r}\n')
    for name in pages:
        if '.' in name:
            write(root / 'web-demo' / name)
        else:
            write(root / 'web-demo' / name / 'a.txt')


class Stage(unittest.TestCase):
    def stage(self, pages):
        with tempfile.TemporaryDirectory() as tmp:
            tmp = pathlib.Path(tmp)
            core(tmp / 'core')
            web(tmp / 'web', pages)
            done = subprocess.run(
                [sys.executable, str(ROOT / 'packaging' / 'stage.py'), '--core', str(tmp / 'core'), '--web', str(tmp / 'web'),
                 '--triple', TRIPLE, '--out', str(tmp / 'dist')],
                capture_output=True, text=True)
            self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
            got = tmp / 'dist' / 'resources' / 'packages' / 'webui' / 'pages'
            return sorted(p.name for p in got.iterdir())

    def test_the_watchdog_is_staged_on_unix_only(self):
        # 核心 LW-28：看门的小程序只在 Unix 上带，Windows 上核心进作业对象
        sys.path.insert(0, str(ROOT / 'packaging'))
        try:
            import stage
        finally:
            sys.path.pop(0)
        self.assertIn('miyu-watchdog', stage.core_programs('x86_64-unknown-linux-gnu'))
        self.assertIn('miyu-watchdog', stage.core_programs('aarch64-apple-darwin'))
        self.assertNotIn('miyu-watchdog', stage.core_programs('x86_64-pc-windows-msvc'))
        self.assertEqual(stage.core_programs('x86_64-pc-windows-msvc'), ['miyu', 'miyu-sandbox', 'miyu-extract'])

    def test_every_page_the_web_packs_is_staged(self):
        # 审计 #082：原来照自己写死的七样拷，漏了网页后来加的 manifest.webmanifest，装好的包里「安装为应用」读它 404
        self.assertEqual(self.stage(PAGES), sorted(PAGES))

    def test_a_page_the_web_adds_later_is_staged_too(self):
        self.assertEqual(self.stage(PAGES + ['icons']), sorted(PAGES + ['icons']))


if __name__ == '__main__':
    unittest.main()
