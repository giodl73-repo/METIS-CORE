"""Check the explicit reviewed foundation transition and reject further drift."""
import json
import pathlib
import subprocess

R = pathlib.Path(__file__).resolve().parents[2]
transition = json.loads((R / 'browser/explorer/foundation-transition.json').read_bytes())
assert transition['schema'] == 'metis.foundation-transition.v1'
paths = transition['protected_paths']
reviewed = transition['reviewed_commit']


def tree(ref):
    result = {}
    for row in subprocess.check_output(['git', 'ls-tree', '-r', '-z', ref, '--', *paths], cwd=R).split(b'\0'):
        if not row:
            continue
        header, name = row.split(b'\t', 1)
        mode, kind, oid = header.decode().split()
        result[name.decode()] = dict(mode=mode, type=kind, oid=oid)
    return result


current = tree(reviewed)
for anchor in transition['anchors']:
    before = tree(anchor['commit'])
    changes = [dict(path=name, before=before.get(name), after=current.get(name))
               for name in sorted(before.keys() | current.keys())
               if before.get(name) != current.get(name)]
    assert changes == anchor['changes'], 'Reviewed transition differs from historical anchor'
subprocess.run(['git', 'diff', '--exit-code', reviewed, '--', *paths], cwd=R, check=True)
assert not subprocess.check_output(['git', 'ls-files', '--others', '--exclude-standard', '--', *paths], cwd=R), 'Untracked protected source'
print('Protected foundation matches explicit reviewed transition; historical changed blobs/modes retained')
