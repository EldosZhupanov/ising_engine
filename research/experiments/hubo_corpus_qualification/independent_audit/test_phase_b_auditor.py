"""Three adversarial checks of the independent auditor; never edit raw evidence."""
import copy
from datetime import datetime, timezone
import importlib.util
import json
from pathlib import Path
import shutil
import tempfile

HERE = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location('independent_phase_b', HERE / 'audit_phase_b.py')
AUDIT = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(AUDIT)


def rejects(call, expected):
    try:
        call()
    except ValueError as error:
        AUDIT.check(expected in str(error), 'mutation rejected for wrong reason: ' + str(error))
        return dict(status='PASS', rejection=str(error))
    raise AssertionError('corrupted evidence was accepted')


def main():
    output = HERE / 'audit_mutations.json'
    AUDIT.check(not output.exists(), 'refuse to overwrite mutation evidence')
    outcomes = {}
    cases = {case['id']: case for case in AUDIT.cases(False)}
    found = False
    for path in sorted((AUDIT.BASE / 'phase_b_run/cells').glob('*.json')):
        original = AUDIT.read(path)
        for index, record in enumerate(original['events']):
            if record['event']['kind'] != 'inc' or record['received_s'] <= 2:
                continue
            altered = copy.deepcopy(original)
            altered['events'][index]['event']['energy'] += 2
            # Keep complete raw-output correspondence valid, so energy validation
            # itself must reject this witness even though it is after the deadline.
            lines = altered['stdout'].split('\n')
            lines[index + 1] = json.dumps(altered['events'][index]['event'])
            altered['stdout'] = '\n'.join(lines)
            case = cases[original['identity']['case']]
            outcomes['wrong_late_energy'] = rejects(
                lambda: AUDIT.audit_row(case, altered), 'reported objective vs independent energy')
            outcomes['wrong_late_energy'].update(cell=path.name, receipt_s=record['received_s'])
            found = True
            break
        if found:
            break
    if not found:
        # This campaign has no complete late incumbents. Inject a copied real
        # witness after 2 s, within its archived process termination interval.
        altered = copy.deepcopy(original)
        event = copy.deepcopy(next(x['event'] for x in original['events'] if x['event']['kind'] == 'inc'))
        event['energy'] += 2
        receipt = (2 + original['termination_s']) / 2
        AUDIT.check(2 < receipt <= original['termination_s'], 'late synthetic receipt interval')
        altered['events'].append(dict(event=event, received_s=receipt))
        complete_prefix = altered['stdout'].rsplit('\n', 1)[0] + '\n'
        altered['stdout'] = complete_prefix + json.dumps(event) + '\n'
        case = cases[original['identity']['case']]
        outcomes['wrong_late_energy'] = rejects(
            lambda: AUDIT.audit_row(case, altered), 'reported objective vs independent energy')
        outcomes['wrong_late_energy'].update(cell=path.name, receipt_s=receipt,
            synthetic_late_event=True, reason='No complete late incumbents in recorded campaign')
    with tempfile.TemporaryDirectory(prefix='q002-independent-mutations-') as temporary:
        directory = Path(temporary) / 'smoke'
        shutil.copytree(AUDIT.BASE / 'phase_b_smoke', directory)
        manifest_path = directory / 'raw_sha256.json'
        manifest = AUDIT.read(manifest_path)
        manifest['cells/0000.json'] = '0' * 64
        manifest_path.write_text(json.dumps(manifest))
        outcomes['corrupted_raw_sha'] = rejects(lambda: AUDIT.audit(directory, True), 'raw SHA: cells/0000.json')
    with tempfile.TemporaryDirectory(prefix='q002-independent-mutations-') as temporary:
        directory = Path(temporary) / 'smoke'
        shutil.copytree(AUDIT.BASE / 'phase_b_smoke', directory)
        summary_path = directory / 'summary.json'
        summary = AUDIT.read(summary_path)
        summary['incumbents'] += 1
        summary_path.write_text(json.dumps(summary))
        outcomes['altered_summary'] = rejects(lambda: AUDIT.audit(directory, True), 'summary.incumbents')
    report = dict(status='PASS', mutations_rejected=3, outcomes=outcomes,
                  audited_utc=datetime.now(timezone.utc).isoformat(),
                  auditor_sha256=AUDIT.sha((HERE / 'audit_phase_b.py').read_bytes()),
                  test_sha256=AUDIT.sha(Path(__file__).read_bytes()),
                  scope='Only in-memory rows and temporary copies mutated; no solver calls or original raw writes')
    with output.open('x') as stream:
        stream.write(json.dumps(report, sort_keys=True, indent=2) + '\n')
    print(json.dumps(dict(status='PASS', mutations_rejected=3, output=str(output))))


if __name__ == '__main__':
    main()
