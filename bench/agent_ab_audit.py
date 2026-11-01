"""Post-run evidence audit; flags require review, never automatic exclusion of failures."""
import argparse
import json
from pathlib import Path
import re
import subprocess


def audit(run):
    result = json.loads((run / 'result.json').read_text())
    records = []
    logs = list(run.rglob('session.v4.jsonl.zstd'))
    if len(logs) != 1:
        raise ValueError(f'{run}: expected one full session log, got {len(logs)}')
    raw = subprocess.run(['zstd', '-dc', str(logs[0])], capture_output=True, text=True, check=True).stdout
    records = [json.loads(line) for line in raw.splitlines() if line.strip()]
    headers, calls, flags, mcp_results = [], [], [], []
    source_models = set()
    call_names = {}
    for event in records:
        data = event.get('data', {})
        if event['type'] == 'request/header':
            headers.append(data['header'].get('config', {}))
        if event['type'] == 'assistant/message':
            source = data.get('message', {}).get('source', {})
            if source.get('kind') == 'model':
                source_models.add((source.get('provider'), source.get('model')))
        if event['type'] == 'tool/call':
            args = json.loads(data.get('arguments') or '{}')
            call = {'id': data['callId'], 'step': data.get('step'), 'name': data['name'], 'arguments': args}
            calls.append(call)
            call_names[data['callId']] = data['name']
            text = json.dumps(args, ensure_ascii=False)
            reasons = []
            if re.search(r'agent-ab|bench-runs|session\.v4|blind-key|controls\.json', text):
                reasons.append('possible benchmark or session access')
            if re.search(r'\b(?:subagent|dsh|kimi|deepseek)\b', data['name']):
                reasons.append('possible other model')
            if '/home/nich/Moirai_N3/nichlink' in text or '../' in text or '/home/' in text:
                reasons.append('possible outside-project access')
            command = args.get('command', '')
            paths = re.findall(r'''(?<![\w:])/(?:tmp|srv|home|root|etc|proc|var)/[^\s"'`;&|<>)]*''', command)
            if any(p != result['work'] and not p.startswith(result['work'] + '/') for p in paths):
                reasons.append('absolute shell path outside project')
            if reasons:
                flags.append({**call, 'reasons': reasons})
        if event['type'] == 'tool/result':
            message = data.get('message', {})
            callid = message.get('toolCallId')
            if call_names.get(callid, '').startswith('mcp__'):
                body = '\n'.join(b.get('text', '') for b in message.get('content', []))
                mcp_results.append({'call': callid, 'name': call_names[callid],
                                    'is_error': message.get('isError', False), 'body': body})
    expected = {'provider': 'deepseek-official', 'model': 'deepseek-flash',
                'reasoningEffort': 'high', 'maxTokens': 16384}
    model_ok = bool(headers) and all(all(h.get(k) == v for k, v in expected.items()) for h in headers)
    return {'run': run.name, 'session_log': str(logs[0]), 'model_config_matches': model_ok,
            'response_models': sorted(source_models), 'full_tool_calls': len(calls),
            'mcp_calls': len(mcp_results), 'calls': calls, 'flags_for_review': flags,
            'mcp_results': mcp_results, 'trajectory_review': 'pending',
            'note': 'Heuristics flag evidence for manual review. No flagged or failed run is dropped from cost totals.'}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    reports = [audit(f.parent) for f in sorted((args.output / 'runs').glob('*/result.json'))]
    (args.output / 'trajectory-audit.json').write_text(json.dumps(reports, indent=2, ensure_ascii=False) + '\n')
    for row in reports:
        print(row['run'], 'model', row['model_config_matches'], 'calls', row['full_tool_calls'],
              'MCP', row['mcp_calls'], 'flags', len(row['flags_for_review']))


if __name__ == '__main__':
    main()
