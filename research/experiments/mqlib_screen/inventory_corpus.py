"""Metadata-only census. Never opens instance coefficients or outcome files."""
import json, argparse
from pathlib import Path
from collections import Counter
ROOT=Path(__file__).resolve().parents[3]
D=ROOT/'benchmark_suite/data/biqmac'
known={
 'be100.1.sparse':['research/PREREG_RC014.md:99'],
 'gka8a.sparse':['research/PREREG_RC014_AMENDMENT_2.md:101'],
 'gka1a.sparse':['research/PREREG_RC014.md:286'],
 'gka5c.sparse':['benchmark_suite/scripts/ab_engine_compare.py:32'],
 'g05_100.0':['benchmark_suite/scripts/compare_openjij.py:58'],
 'pm1s_100.3':['benchmark_suite/scripts/compare_openjij.py:59'],
}
rows=[]
for p in sorted(D.iterdir()):
 if not p.is_file() or p.name in ['README.md','metadata.json']:continue
 ns='be_sparse' if p.name.startswith('be') and p.suffix=='.sparse' else 'gka_sparse' if p.name.startswith('gka') and p.suffix=='.sparse' else 'ising' if p.name.startswith(('ising','t2g','t3g')) else 'rudy_other'
 rows.append({'path':str(p.relative_to(ROOT)), 'name':p.name,'bytes':p.stat().st_size,'filename_namespace':ns,'eligible_as_unseen':'NOT_ESTABLISHED','exposure_status':'NAMED_IN_PRIOR_PROTOCOL_OR_INSTRUMENT' if p.name in known else 'UNKNOWN_OUTCOME_EXPOSURE','refs':known.get(p.name,[]),'prior_header_inspection':p.suffix=='.sparse'})
m=json.loads((D/'metadata.json').read_text())
obj={'schema':'metadata_only_corpus_inventory_v1','date':'2026-09-27','scope':'filenames, file stat, existing metadata and protocol/instrument declarations only; no coefficient/outcome reads or new downloads','reproduction':'python3 research/experiments/mqlib_screen/inventory_corpus.py --output NEW.json','notes':['Filename namespaces are not independently verified scientific families.','Local registry description mislabels be sparse as Beasley; official-source identification is handled separately by parent.','Existing metadata hashes were not recomputed against coefficient files in this pass.','No file is cleared as an untouched evaluation instance by this inventory.','Prior use filenames are conservative exposure-risk markers; instrument inclusion alone is not proof of completed execution.','125 sparse headers historically inspected per PREREG_RC014_AMENDMENT_2.md:90; metadata inspection is not outcome exposure.'], 'parent_named_path_exists':(ROOT/'benchmark_suite/biqmac').exists(),'actual_path':'benchmark_suite/data/biqmac','data_file_count':len(rows),'data_bytes':sum(r['bytes'] for r in rows),'namespace_counts':dict(Counter(r['filename_namespace'] for r in rows)),'existing_provenance':{k:m[k] for k in ['dataset','official_url','downloaded_at','status']},'metadata_record_count':len(m['files']),'provenance_refs':['benchmark_suite/data/biqmac/metadata.json','benchmark_suite/data/biqmac/README.md','benchmark_suite/configs/benchmark_registry.yaml','benchmark_suite/scripts/download_all_benchmarks.py','benchmark_suite/ising_bench/download.py','memory/FILE_MAP.tsv:82','research/EXTERNAL_COMPARISON_PROTOCOL.md:79'],'archive_names':['rudy_all.tar.gz','ising_all.tar.gz','gka_sparse_all.tar.gz','be_sparse_all.tar.gz'],'seed_constraints':{'refs':['research/PREREG_RC021_HOST_INSTRUMENT.md:69','research/PREREG_RC021_HOST_INSTRUMENT.md:97','research/PREREG_RC021_HOST_INSTRUMENT.md:133','research/experiments/mqlib_screen/protocol.md','research/mqlib_qualification/README.md'],'burned_ranges':[[5001,5008],[7001,7008],[10001,10008]],'reserved_unopened_ranges':[[6001,6008],[8001,8008],[9001,9008],[11001,11008],[12001,12008]],'rc021_reserved_range':[31001,31099],'earlier_science_ranges':[[1001,1008],[2001,2008],[3001,3008],[4001,4008]],'mq_screen_generator_range':[41001,41008],'mq_screen_search_range':[51001,51010],'mq_screen_bootstrap_seed':52001,'mq_smoke_seeds':[501,502],'mq_qualification_seeds':[101,102,103],'warning':'Minimum exclusion inventory only; derived/control/permutation seeds additionally occupied. No new seed authorized.'},'files':rows}
parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('--output',type=Path,required=True)
args=parser.parse_args()
with args.output.open('x') as f:
 f.write(json.dumps(obj,indent=2)+'\n')
print(json.dumps({'path':str(args.output),'count':len(rows),'bytes':obj['data_bytes'],'namespaces':obj['namespace_counts']}))
