#!/usr/bin/env python3
# 搭安装包的内容（施工 DT-6，图纸 app.md「装在哪」）：核心、网页软件编好以后，照 Tauri 打包工具要的样子放进 dist/。
#   dist/binaries/<程序>-<目标三元组>[.exe]  Tauri 的 externalBin：装在托盘程序旁边，名字去掉三元组
#   dist/resources/                         核心仓库的整份 resources/，加网页包的文件夹 packages/webui/
#   dist/resources/licenses/                托盘、核心、网页三份许可证和第三方许可证清单（施工 AP-5，原来外壳的 DT-7）
#   dist/resources/pwsh/                    只 Windows：官方的 PowerShell 7 zip 版解开（施工 AP-5，原来外壳的 DT-9），核心先找它
#   dist/pins.txt                           钉的两个提交号，写进发版说明
# 网页包的文件夹照网页仓库 tools/pack-web.py 装好的样子（网页的会话 2026-10-30 定）：清单没写 version 的补上（照网页的工作区版本）；
# 网页 W-1095（0.1.0）起清单自己写 version 和 targets 三种，照原样用，version 要和工作区版本一样，
# web.json、matugen/ 原样，许可证（仓库根的 LICENSE、webui/ 的第三方许可证清单）放包的根上，pages/ 是 web-demo 里给浏览器的七样，
# 程序放 bin/。等待页 loading.html 是给原来外壳的窗口用的，托盘不用，不带。
#   python3 packaging/stage.py --core <核心检出> --web <网页检出> --triple <rustc 的 host> --out dist
import argparse, hashlib, io, pathlib, re, shutil, sys, tomllib, urllib.request, zipfile

# 核心带的三个程序（核心的主会话 2026-10-30 定）：主程序、沙盒助手、抽字的小程序。
CORE_PROGRAMS = ['miyu', 'miyu-sandbox', 'miyu-extract']
# 网页包的编号，也是网页仓库里那个文件夹的名字（2026-11-12 起包编号不带 miyu-）
WEB_PACKAGE = 'webui'
PAGES = ['index.html', 'distro.json', 'src', 'styles', 'packages', 'resources', 'vendor']


def workspace_version(root):
    text = (root / 'Cargo.toml').read_text(encoding='utf-8')
    found = re.search(r'^\[workspace\.package\]\n(?:.*\n)*?version\s*=\s*"([^"]+)"', text, re.M)
    if not found:
        sys.exit(f'{root}/Cargo.toml 的 [workspace.package] 里没有 version')
    return found.group(1)


def with_version(manifest, version):
    written = tomllib.loads(manifest).get('package', {}).get('version')
    if written is not None:
        # 网页 W-1095 起清单自己写：照原样，和工作区版本对不上的不装
        if written != version:
            sys.exit(f'{WEB_PACKAGE}/package.toml 的 version {written!r} 和网页的工作区版本 {version!r} 不一样：先找网页的会话')
        return manifest
    head = re.search(r'^\[package\]\n', manifest, re.M)
    if not head:
        sys.exit(f'{WEB_PACKAGE}/package.toml 里没有 [package]')
    return manifest[:head.end()] + f'version = "{version}"\n' + manifest[head.end():]


def copy_program(built, name, exe, dest):
    src = built / f'{name}{exe}'
    if not src.is_file():
        sys.exit(f'没编出来：{src}')
    dest.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(src, dest)


def stage_pwsh(pin, triple, dest, fetch=None):
    """Windows：官方的 PowerShell zip 下载下来、核对校验值、解进资源目录的 pwsh/（施工 AP-5，原来外壳的 DT-9）。
    校验值照 pins.toml，对不上的停，不出包。fetch 是测试换掉下载用的。"""
    if dest.exists():
        sys.exit(f'核心的资源目录里已经有 {dest.name}/：先问核心的会话')
    arch = 'arm64' if triple.startswith('aarch64') else 'x64'
    version, want = pin['version'], pin[f'win-{arch}']
    url = f'https://github.com/PowerShell/PowerShell/releases/download/v{version}/PowerShell-{version}-win-{arch}.zip'
    data = (fetch or download)(url)
    got = hashlib.sha256(data).hexdigest()
    if got != want:
        sys.exit(f'PowerShell {version} win-{arch} 的校验值不对：{got}，pins.toml 里是 {want}')
    with zipfile.ZipFile(io.BytesIO(data)) as archive:
        archive.extractall(dest)
    # zip 里自带 MIT 的 LICENSE.txt 和 ThirdPartyNotices.txt，原样留在 pwsh/ 里（licenses/README.txt 指过去）
    for name in ('pwsh.exe', 'LICENSE.txt', 'ThirdPartyNotices.txt'):
        if not (dest / name).is_file():
            sys.exit(f'{url} 解开以后没有 {name}')
    print(f'PowerShell {version} win-{arch} -> {dest}')


def download(url):
    with urllib.request.urlopen(url, timeout=600) as response:
        return response.read()


PWSH_NOTE = """
Windows only: PowerShell 7 is bundled in ../pwsh/ with its own LICENSE.txt (MIT) and ThirdPartyNotices.txt.
"""


LICENSES_README = """Licenses of the software in this Miyu package.

miyu-applet/   the tray app (miyu-applet): LICENSE (AGPL-3.0-or-later) and THIRD-PARTY-LICENSES.txt
miyu/       the core (miyu, miyu-sandbox, miyu-extract and these resources): LICENSE (AGPL-3.0-or-later),
            LICENSES/ (Apache-2.0 for the parts listed in the core's README) and THIRD-PARTY-LICENSES.txt
miyu-web/   the web app (packages/webui): LICENSE (AGPL-3.0-or-later) and THIRD-PARTY-LICENSES.txt
"""


def stage_licenses(app, core, web, dest, windows):
    """三份许可证和第三方许可证清单放进资源目录的 licenses/（施工 AP-5，原来外壳的 DT-7）：缺一份就停，不出没有清单的包。
    核心、网页的清单是它们各自仓库生成、门禁守着的那一份，照钉的提交原样拷。"""
    if dest.exists():
        sys.exit(f'核心的资源目录里已经有 {dest.name}/：先问核心的会话')
    parts = {
        'miyu-applet': (app, ['LICENSE', 'THIRD-PARTY-LICENSES.txt']),
        'miyu': (core, ['LICENSE', 'LICENSES', 'THIRD-PARTY-LICENSES.txt']),
        'miyu-web': (web, ['LICENSE', f'{WEB_PACKAGE}/THIRD-PARTY-LICENSES.txt']),
    }
    for part, (root, names) in parts.items():
        for name in names:
            src = root / name
            target = dest / part / pathlib.Path(name).name
            if src.is_dir():
                shutil.copytree(src, target)
            elif src.is_file():
                target.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(src, target)
            else:
                sys.exit(f'少了 {src}：安装包要带它（施工 AP-5）')
    (dest / 'README.txt').write_text(LICENSES_README + (PWSH_NOTE if windows else ''), encoding='utf-8')


def main():
    ap = argparse.ArgumentParser(description='搭 Miyu 安装包的内容')
    ap.add_argument('--core', required=True, type=pathlib.Path, help='核心的检出，已经编过 release')
    ap.add_argument('--web', required=True, type=pathlib.Path, help='网页的检出，已经编过 release')
    ap.add_argument('--triple', required=True, help='目标三元组，比如 x86_64-unknown-linux-gnu')
    ap.add_argument('--out', required=True, type=pathlib.Path)
    ap.add_argument('--core-built', type=pathlib.Path, help='核心编好的程序在哪，不写照 <核心检出>/target/release')
    ap.add_argument('--web-built', type=pathlib.Path, help='网页软件编好的程序在哪，不写照 <网页检出>/target/release')
    args = ap.parse_args()

    exe = '.exe' if 'windows' in args.triple else ''
    out = args.out.resolve()
    if out.exists():
        shutil.rmtree(out)

    for name in CORE_PROGRAMS:
        copy_program(args.core_built or args.core / 'target' / 'release', name, exe, out / 'binaries' / f'{name}-{args.triple}{exe}')

    resources = out / 'resources'
    shutil.copytree(args.core / 'resources', resources)
    package = resources / 'packages' / WEB_PACKAGE
    if package.exists():
        sys.exit(f'核心的资源目录里已经有 packages/{WEB_PACKAGE}/：先问核心的会话')
    source = args.web / WEB_PACKAGE
    package.mkdir(parents=True)
    manifest = (source / 'package.toml').read_text(encoding='utf-8')
    manifest = with_version(manifest, workspace_version(args.web))
    tomllib.loads(manifest)
    (package / 'package.toml').write_text(manifest, encoding='utf-8')
    shutil.copy2(source / 'web.json', package / 'web.json')
    shutil.copytree(source / 'matugen', package / 'matugen')
    # 许可证照 pack-web.py 放包的根上（网页的 0.1.0 起清单在 webui/ 里）：缺了就停，不出没有清单的包
    for src, name in ((source / 'THIRD-PARTY-LICENSES.txt', 'THIRD-PARTY-LICENSES.txt'), (args.web / 'LICENSE', 'LICENSE')):
        if not src.is_file():
            sys.exit(f'网页少了 {src}：安装包要带它（施工 AP-5）')
        shutil.copy2(src, package / name)
    for name in PAGES:
        page = args.web / 'web-demo' / name
        if page.is_dir():
            shutil.copytree(page, package / 'pages' / name)
        elif page.is_file():
            (package / 'pages').mkdir(exist_ok=True)
            shutil.copy2(page, package / 'pages' / name)
        else:
            sys.exit(f'网页少了 {page}')
    copy_program(args.web_built or args.web / 'target' / 'release', 'miyu-web', exe, package / 'bin' / f'miyu-web{exe}')

    app = pathlib.Path(__file__).resolve().parent.parent
    windows = 'windows' in args.triple
    stage_licenses(app, args.core, args.web, resources / 'licenses', windows)
    pins = tomllib.loads((app / 'pins.toml').read_text(encoding='utf-8'))
    if windows:
        stage_pwsh(pins['pwsh'], args.triple, resources / 'pwsh')
    lines = [f'{part}: {pins[part]["repo"]}@{pins[part]["rev"]}\n' for part in ('core', 'web')]
    lines.append(f'pwsh (Windows): PowerShell {pins["pwsh"]["version"]}\n')
    (out / 'pins.txt').write_text(''.join(lines), encoding='utf-8')
    print(f'staged into {out}')


if __name__ == '__main__':
    main()
