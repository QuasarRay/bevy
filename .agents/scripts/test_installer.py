from __future__ import annotations
from pathlib import Path
import importlib.util, os, shutil, subprocess, sys, tempfile, tomllib
SOURCE=Path(__file__).resolve().parents[2]
def check(condition,message='check failed'):
    if not condition: raise AssertionError(message)
def run(root,*args,expect=0,uninstall=False):
    script=root/'.agents'/'bootstrap'/('uninstall.py' if uninstall else 'install.py')
    p=subprocess.run([sys.executable,str(script),*args],cwd=root,text=True,capture_output=True,timeout=45)
    if p.returncode!=expect: raise AssertionError(f'expected {expect}, got {p.returncode}\nSTDOUT:{p.stdout}\nSTDERR:{p.stderr}')
    return p
def copy_package(dst): shutil.copytree(SOURCE/'.agents',dst/'.agents')
def main():
    with tempfile.TemporaryDirectory(prefix='seqmax tests with spaces ') as td:
        root=Path(td)/'fresh repo'; root.mkdir(); copy_package(root); run(root); run(root)
        check((root/'AGENTS.md').read_text().count('BEGIN SEQMAX')==1)
        run(root,uninstall=True); check(not (root/'.codex'/'config.toml').exists()); check(not (root/'AGENTS.md').exists())
    if os.name!='nt':
        with tempfile.TemporaryDirectory() as td:
            root=Path(td)/'permissions'; root.mkdir(); copy_package(root); (root/'.codex').mkdir(); cfg=root/'.codex'/'config.toml'; cfg.write_text('file_opener="vscode"\n'); cfg.chmod(0o640)
            run(root); check((cfg.stat().st_mode & 0o777)==0o640); check(((root/'AGENTS.md').stat().st_mode & 0o777)==0o644)
    with tempfile.TemporaryDirectory() as td:
        root=Path(td)/'crlf'; root.mkdir(); copy_package(root); (root/'.codex').mkdir()
        cfg=root/'.codex'/'config.toml'; cfg.write_bytes(b'file_opener = "vscode"\r\n\r\n[[hooks.PreToolUse]]\r\nmatcher="Bash"\r\n')
        run(root); check(b'\r\n' in cfg.read_bytes()); run(root,uninstall=True); check(b'\r\n' in cfg.read_bytes())
    with tempfile.TemporaryDirectory() as td:
        root=Path(td)/'preserve'; root.mkdir(); copy_package(root)
        (root/'AGENTS.md').write_text('# Existing\n\nKeep me.\n',encoding='utf-8')
        (root/'.codex').mkdir(); (root/'.codex'/'config.toml').write_text('file_opener = "vscode"\n\n[agents]\ninterrupt_message = true\n\n[[hooks.PreToolUse]]\nmatcher="Bash"\n',encoding='utf-8')
        (root/'.codex'/'agents').mkdir(); (root/'.codex'/'agents'/'mine.toml').write_text('name="mine"\ndescription="mine"\ndeveloper_instructions="mine"\n',encoding='utf-8')
        run(root); cfg=tomllib.loads((root/'.codex'/'config.toml').read_text()); check(cfg['file_opener']=='vscode'); check(cfg['hooks']['PreToolUse'][0]['matcher']=='Bash'); check((root/'.codex'/'agents'/'mine.toml').exists())
        run(root,uninstall=True); check('Keep me.' in (root/'AGENTS.md').read_text()); cfg=tomllib.loads((root/'.codex'/'config.toml').read_text()); check(cfg['file_opener']=='vscode'); check(cfg['agents']['interrupt_message'] is True)
    with tempfile.TemporaryDirectory() as td:
        root=Path(td)/'user-agents-content'; root.mkdir(); copy_package(root); run(root)
        cfg=root/'.codex'/'config.toml'; text=cfg.read_text(encoding='utf-8')
        text=text.replace('[agents.seqmax_explorer]', 'user_owned_setting = "keep"\n\n[agents.seqmax_explorer]',1); cfg.write_text(text,encoding='utf-8')
        run(root,uninstall=True); parsed=tomllib.loads(cfg.read_text()); check(parsed['agents']['user_owned_setting']=='keep')
    with tempfile.TemporaryDirectory() as td:
        root=Path(td)/'conflict'; root.mkdir(); copy_package(root); (root/'.codex').mkdir(); cfg=root/'.codex'/'config.toml'; cfg.write_text('model="other"\n',encoding='utf-8'); original=cfg.read_bytes()
        run(root,expect=1); check(cfg.read_bytes()==original)
        run(root,'--force-conflicts'); check(tomllib.loads(cfg.read_text())['model']=='gpt-5.6-sol')
        reports=sorted((root/'.agents'/'runtime'/'install-reports').rglob('install-report.json')); report=reports[-1]
        run(root,'--restore-report',str(report),uninstall=True); check(cfg.read_bytes()==original)
    with tempfile.TemporaryDirectory() as td:
        root=Path(td)/'role-conflict'; root.mkdir(); copy_package(root); (root/'.codex').mkdir(); cfg=root/'.codex'/'config.toml'
        cfg.write_text('[agents]\nfoo="bar"\n\n[agents.seqmax_explorer]\ndescription="mine"\nconfig_file="./agents/mine.toml"\n',encoding='utf-8'); before=cfg.read_bytes()
        run(root,expect=1); check(cfg.read_bytes()==before); run(root,'--force-conflicts'); parsed=tomllib.loads(cfg.read_text()); check(parsed['agents']['foo']=='bar'); check(parsed['agents']['seqmax_explorer']['config_file']=='./agents/seqmax-explorer.toml')
    with tempfile.TemporaryDirectory() as td:
        root=Path(td)/'dotted'; root.mkdir(); copy_package(root); (root/'.codex').mkdir(); cfg=root/'.codex'/'config.toml'; cfg.write_text('agents.enabled=true\n',encoding='utf-8'); original=cfg.read_bytes(); run(root,expect=1); check(cfg.read_bytes()==original)
    with tempfile.TemporaryDirectory() as td:
        root=Path(td)/'markers'; root.mkdir(); copy_package(root); (root/'AGENTS.md').write_text('<!-- BEGIN SEQMAX MANAGED BLOCK v3 -->\n',encoding='utf-8'); run(root,expect=1); check((root/'AGENTS.md').read_text().count('BEGIN')==1)
    if hasattr(os,'symlink') and os.name!='nt':
        with tempfile.TemporaryDirectory() as td:
            root=Path(td)/'links'; root.mkdir(); copy_package(root); outside=Path(td)/'outside'; outside.mkdir(); os.symlink(outside,root/'.codex'); run(root,expect=1); check(not any(outside.iterdir()))
    with tempfile.TemporaryDirectory() as td:
        root=Path(td)/'lock'; root.mkdir(); copy_package(root); lock=root/'.agents'/'runtime'/'.install.lock'; lock.write_text('{}',encoding='utf-8'); run(root,expect=1); check(not (root/'AGENTS.md').exists())
    # Transaction rollback: inject one write failure after the first applied destination.
    with tempfile.TemporaryDirectory() as td:
        root=Path(td)/'rollback'; root.mkdir(); copy_package(root)
        modpath=root/'.agents'/'bootstrap'/'install.py'; spec=importlib.util.spec_from_file_location('seqinst',modpath); mod=importlib.util.module_from_spec(spec); sys.modules['seqinst']=mod; spec.loader.exec_module(mod)
        changes,_=mod.plan(False,False); backup=mod.unique_run_dir('backups'); real=mod.atomic_write; count={'n':0,'failed':False}
        def flaky(path,data):
            if str(path).startswith(str(root)) and '.agents/runtime/backups' not in str(path):
                count['n']+=1
                if count['n']==2 and not count['failed']:
                    count['failed']=True; raise OSError('injected write failure')
            return real(path,data)
        mod.atomic_write=flaky
        try: mod.apply_changes(changes,backup); raise AssertionError('failure was not injected')
        except mod.InstallError: pass
        check(not (root/'AGENTS.md').exists()); check(not (root/'.codex'/'config.toml').exists())
    with tempfile.TemporaryDirectory() as td:
        root=Path(td)/'legacy-clean'; root.mkdir(); copy_package(root)
        legacy=SOURCE/'.agents'/'registry'/'legacy-v2'; shutil.copy2(legacy/'AGENTS.md',root/'AGENTS.md')
        (root/'.codex'/'agents').mkdir(parents=True); hashes=tomllib.loads((legacy/'agent-hashes.toml').read_text())['file']
        # One exact legacy file is sufficient to exercise signature deletion.
        original=legacy/'agents'/hashes[0]['name']
        if original.is_file(): shutil.copy2(original,root/'.codex'/'agents'/hashes[0]['name'])
        cleaner=root/'.agents'/'scripts'/'cleanup_legacy_v2.py'
        p=subprocess.run([sys.executable,str(cleaner),'--remove-exact'],cwd=root,text=True,capture_output=True,timeout=45); check(p.returncode==0,p.stderr); check(not (root/'AGENTS.md').exists())
    with tempfile.TemporaryDirectory() as td:
        root=Path(td)/'report-write-failure'; root.mkdir(); copy_package(root)
        modpath=root/'.agents'/'bootstrap'/'install.py'; spec=importlib.util.spec_from_file_location('seqinst_report',modpath); mod=importlib.util.module_from_spec(spec); sys.modules['seqinst_report']=mod; spec.loader.exec_module(mod)
        changes,_=mod.plan(False,False); backup=mod.unique_run_dir('backups'); report_path=mod.unique_run_dir('install-reports')/'install-report.json'; all_changes=changes+[mod.Change(report_path,None,b'{}\n','report')]
        real=mod.atomic_write
        def fail_report(path,data,mode=None):
            if path==report_path: raise OSError('injected report failure')
            return real(path,data,mode)
        mod.atomic_write=fail_report
        try: mod.apply_changes(all_changes,backup); raise AssertionError('report failure was not injected')
        except mod.InstallError: pass
        check(not (root/'AGENTS.md').exists()); check(not (root/'.codex'/'config.toml').exists())
    print('INSTALLER TESTS PASSED')
if __name__=='__main__': main()
