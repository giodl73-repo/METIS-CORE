"""Exercise the real guard in a new Git history without original anchor objects."""
import copy
import io
import json
from pathlib import Path, PurePosixPath
import subprocess
import sys
import tempfile
import zipfile

ROOT = Path(__file__).resolve().parents[2]


def main():
    commands = []
    with tempfile.TemporaryDirectory(prefix='metis-foundation-control-') as temporary:
        repo = Path(temporary).resolve()
        assert repo.name.startswith('metis-foundation-control-')
        modes = {}
        for row in subprocess.check_output(['git', 'ls-tree', '-rz', 'HEAD'], cwd=ROOT).split(b'\0'):
            if row:
                header, name = row.split(b'\t', 1)
                mode, kind, oid = header.split()
                assert kind == b'blob' and mode in (b'100644', b'100755')
                modes[name.decode()] = int(mode, 8)
        raw = subprocess.check_output(['git', '-c', 'core.autocrlf=false', 'archive', '--format=zip', 'HEAD'], cwd=ROOT)
        with zipfile.ZipFile(io.BytesIO(raw)) as archive:
            assert {e.filename for e in archive.infolist() if not e.is_dir()} == set(modes)
            for entry in archive.infolist():
                path = PurePosixPath(entry.filename)
                assert not path.is_absolute() and '..' not in path.parts and '\\' not in entry.filename
                if entry.is_dir():
                    continue
                mode = modes[entry.filename]
                target = repo/entry.filename
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_bytes(archive.read(entry))
                target.chmod(mode & 0o777)
        for name in ['verify-foundation.py', 'foundation-transition.json']:
            (repo/'browser/explorer'/name).write_bytes((ROOT/'browser/explorer'/name).read_bytes())
        def git(*args):
            return subprocess.run(['git', *args], cwd=repo, capture_output=True, check=True)
        git('init')
        git('config', 'core.autocrlf', 'false')
        git('add', '.')
        git('-c', 'user.name=Guard control', '-c', 'user.email=guard@example.invalid',
            'commit', '-m', 'Fresh unrelated history')
        manifest_path = repo/'browser/explorer/foundation-transition.json'
        original = manifest_path.read_bytes()
        manifest = json.loads(original)
        for commit in manifest['snapshots']:
            assert subprocess.run(['git', 'cat-file', '-e', commit+'^{commit}'],
                                  cwd=repo, capture_output=True).returncode != 0
        def run(name, expected):
            result = subprocess.run([sys.executable, '-B', 'browser/explorer/verify-foundation.py'],
                                    cwd=repo, capture_output=True)
            commands.append(dict(name=name, exit_code=result.returncode, expected_success=expected))
            assert (result.returncode == 0) == expected, (name, result.stdout, result.stderr)
        run('missing-original-objects-positive', True)
        for name in ['src/lib.rs', 'LICENSE']:
            target = repo/name
            before = target.read_bytes()
            try:
                target.write_bytes(before+b'\nchanged\n')
                run('reject-'+name, False)
            finally:
                target.write_bytes(before)
        target = repo/'src/untracked_guard_probe.rs'
        try:
            target.write_bytes(b'// unexpected protected source\n')
            run('reject-untracked', False)
        finally:
            target.unlink()
        controls = [
            ('raw-commit', lambda s:s['snapshots'][s['reviewed_commit']].update(commit_bytes_hex='00')),
            ('full-tree', lambda s:s['snapshots'][s['reviewed_commit']]['entries']['src/lib.rs'].update(oid='0'*40)),
            ('scope', lambda s:s['protected_paths'].remove('tests')),
            ('anchor', lambda s:s['anchors'].pop()),
            ('historical-delta', lambda s:s['anchors'][0]['changes'].pop()),
        ]
        for name, change in controls:
            altered = copy.deepcopy(manifest)
            change(altered)
            try:
                manifest_path.write_text(json.dumps(altered)+'\n', encoding='utf-8')
                run('reject-'+name, False)
            finally:
                manifest_path.write_bytes(original)
        run('restored-positive', True)
    print(json.dumps(dict(original_commit_objects_absent=True, controls=commands)))


if __name__ == '__main__':
    main()
