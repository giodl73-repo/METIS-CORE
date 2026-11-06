"""Check the explicit reviewed foundation transition and reject further drift."""
import json
import hashlib
import pathlib
import subprocess

R = pathlib.Path(__file__).resolve().parents[2]
transition = json.loads((R / 'browser/explorer/foundation-transition.json').read_bytes())
assert transition['schema'] == 'metis.foundation-transition.v2'
paths = transition['protected_paths']
reviewed = transition['reviewed_commit']
assert paths == ['src', 'Cargo.toml', 'Cargo.lock', 'tests', 'verify', 'benches',
                 '.pitfall', 'docs', '.roles', '.github/ci.yml', 'LICENSE', 'NOTICE']
assert reviewed == '0ad9f83d1ed41c1499d00233fefa0d4de6743aa8'
assert [a['commit'] for a in transition['anchors']] == [
    'df08dec373b3daecd73313750766e397aa047e9f', 'b4dc9663243c26771112da3329f171fc5f28c098']


def tree(ref):
    result = {}
    for row in subprocess.check_output(['git', 'ls-tree', '-r', '-z', ref, '--', *paths], cwd=R).split(b'\0'):
        if not row:
            continue
        header, name = row.split(b'\t', 1)
        mode, kind, oid = header.decode().split()
        result[name.decode()] = dict(mode=mode, type=kind, oid=oid)
    return result


def object_id(kind, data):
    return hashlib.sha1(kind.encode()+b' '+str(len(data)).encode()+b'\0'+data).hexdigest()


def verify_snapshot(commit):
    snapshot = transition['snapshots'][commit]
    nodes = {}
    for name, entry in snapshot['entries'].items():
        parts = name.split('/')
        assert all(p and p not in ('.', '..') and '\0' not in p for p in parts)
        assert entry['type'] == 'blob' and entry['mode'] in ('100644', '100755', '120000')
        assert len(bytes.fromhex(entry['oid'])) == 20
        node = nodes
        for part in parts[:-1]:
            node = node.setdefault(part, {})
            assert isinstance(node, dict)
        assert parts[-1] not in node
        node[parts[-1]] = (entry['mode'], entry['oid'])

    def tree_id(node):
        data = b''
        for name, value in sorted(node.items(), key=lambda p: (p[0]+('/' if isinstance(p[1], dict) else '')).encode()):
            mode, oid = ('40000', tree_id(value)) if isinstance(value, dict) else value
            data += mode.encode()+b' '+name.encode()+b'\0'+bytes.fromhex(oid)
        return object_id('tree', data)

    raw_commit = bytes.fromhex(snapshot['commit_bytes_hex'])
    assert object_id('commit', raw_commit) == commit, 'Original commit receipt differs'
    assert raw_commit.split(b'\n', 1)[0] == b'tree '+tree_id(nodes).encode(), 'Original complete tree receipt differs'
    return {name:entry for name, entry in snapshot['entries'].items()
            if any(name == path or name.startswith(path+'/') for path in paths)}


# Original reviewed/anchor commits need not remain reachable after a squash.
# Complete tree + raw commit receipts preserve their exact Git identity offline.
assert set(transition['snapshots']) == {reviewed, *(a['commit'] for a in transition['anchors'])}
current = verify_snapshot(reviewed)
for anchor in transition['anchors']:
    before = verify_snapshot(anchor['commit'])
    changes = [dict(path=name, before=before.get(name), after=current.get(name))
               for name in sorted(before.keys() | current.keys())
               if before.get(name) != current.get(name)]
    assert changes == anchor['changes'], 'Reviewed transition differs from historical anchor'
assert tree('HEAD') == current, 'Protected committed foundation differs from reviewed source'
subprocess.run(['git', 'diff', '--exit-code', 'HEAD', '--', *paths], cwd=R, check=True)
assert not subprocess.check_output(['git', 'ls-files', '--others', '--exclude-standard', '--', *paths], cwd=R), 'Untracked protected source'
print('Protected foundation matches explicit reviewed transition; historical changed blobs/modes retained')
