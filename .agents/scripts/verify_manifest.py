from pathlib import Path
import hashlib
AG=Path(__file__).resolve().parents[1]
manifest=AG/'MANIFEST.sha256'; errors=[]; listed={}; order=[]
for line in manifest.read_text(encoding='utf-8').splitlines():
    if not line.strip(): continue
    try: digest,rel=line.split('  ',1)
    except ValueError: errors.append(f'malformed manifest line: {line!r}'); continue
    if rel in listed: errors.append(f'duplicate manifest path {rel}'); continue
    listed[rel]=digest; order.append(rel); p=AG/rel
    if not p.is_file(): errors.append(f'missing {rel}'); continue
    actual=hashlib.sha256(p.read_bytes()).hexdigest()
    if actual!=digest: errors.append(f'hash mismatch {rel}')
if order!=sorted(order): errors.append('manifest paths are not sorted')
actual=set()
for p in AG.rglob('*'):
    if not p.is_file() or p==manifest: continue
    rel=p.relative_to(AG).as_posix()
    if rel.startswith('runtime/') and rel!='runtime/.gitignore': continue
    if '__pycache__/' in rel or rel.endswith('.pyc'): continue
    actual.add(rel)
for rel in sorted(actual-set(listed)): errors.append(f'unexpected unmanifested file {rel}')
for rel in sorted(set(listed)-actual):
    if (AG/rel).is_file(): errors.append(f'manifest policy mismatch for {rel}')
if errors:
    print('MANIFEST INVALID'); print('\n'.join('- '+e for e in errors)); raise SystemExit(1)
print('MANIFEST VALID')
