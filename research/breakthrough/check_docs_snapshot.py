from pathlib import Path
import subprocess, tempfile, shutil, hashlib
root=Path.cwd(); snap=Path(tempfile.mkdtemp(prefix='breakthrough-memory-'))
archive=snap/'base.tar'
with archive.open('wb') as f: subprocess.run(['git','archive','HEAD'],stdout=f,check=True)
subprocess.run(['tar','-xf',str(archive),'-C',str(snap)],check=True); archive.unlink()
for cmd in [['git','init','-q'],['git','add','.'],['git','-c','user.name=Validation Snapshot','-c','user.email=validation@invalid','commit','-qm','Synthetic source snapshot']]: subprocess.run(cmd,cwd=snap,check=True)
paths=['memory/NOW.md','memory/CATALOG.md','memory/BINDING_SHA256','memory/TIMELINE.md']+[str(f) for f in Path('research/breakthrough').rglob('*') if f.is_file()]
manifest=[]
for rel in paths:
    dest=snap/rel; dest.parent.mkdir(parents=True,exist_ok=True); shutil.copyfile(root/rel,dest)
    manifest.append(hashlib.sha256((root/rel).read_bytes()).hexdigest()+'  '+rel)
result=subprocess.run(['bash','scripts/check_memory_docs.sh','HEAD'],cwd=snap,text=True,capture_output=True)
print('Snapshot:',snap); print(result.stdout,result.stderr)
(root/'research/breakthrough/memory_validation_manifest.txt').write_text('source='+subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip()+'\n'+'\n'.join(manifest)+'\n')
raise SystemExit(result.returncode)
