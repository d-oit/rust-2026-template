import json
import argparse
from datetime import datetime, timezone
from statistics import mean
import os
import hashlib

def file_sha256(path):
    if not path or not os.path.exists(path):
        return hashlib.sha256(b'').hexdigest()
    hasher = hashlib.sha256()
    with open(path, 'rb') as f:
        while chunk := f.read(65536):
            hasher.update(chunk)
    return hasher.hexdigest()

def load_json(path, required=False):
    if not os.path.exists(path):
        if required:
            raise FileNotFoundError(f"Required input JSON file not found: {path}")
        return []
    with open(path, 'r') as f:
        try:
            data = json.load(f)
            if required and not isinstance(data, list):
                raise ValueError(f"Required JSON file {path} must contain a JSON array, got {type(data).__name__}")
            return data
        except json.JSONDecodeError as e:
            if required:
                raise ValueError(f"Failed to parse required JSON file {path}: {e}") from e
            return []

def load_jsonl(path):
    if not os.path.exists(path):
        return []
    data = []
    with open(path, 'r') as f:
        for line in f:
            if line.strip():
                try:
                    data.append(json.loads(line))
                except json.JSONDecodeError:
                    continue
    return data

def load_policy(policy_path):
    default_policy = {
        "version": "1.0",
        "period_days": 30,
        "bot_allowlist": [
            "dependabot[bot]",
            "renovate[bot]",
            "github-actions[bot]"
        ],
        "merge_strategy": "squash_rebase_merge",
        "percentile_method": "mean",
        "revert_predicate": "title_contains_revert",
        "thresholds": {
            "deployment_frequency_per_day": {
                "elite": 1.0,
                "high": 0.142857,
                "medium": 0.033333
            },
            "change_lead_time_hours": {
                "elite": 1.0,
                "high": 24.0,
                "medium": 168.0
            },
            "change_failure_rate": {
                "elite": 0.05,
                "high": 0.10,
                "medium": 0.15
            },
            "fdrt_hours": {
                "elite": 1.0,
                "high": 24.0,
                "medium": 168.0
            }
        }
    }
    resolved_path = policy_path
    if not resolved_path:
        if os.path.exists("plans/dora.json"):
            resolved_path = "plans/dora.json"

    if resolved_path and os.path.exists(resolved_path):
        try:
            with open(resolved_path, 'r') as f:
                data = json.load(f)
                if isinstance(data, dict):
                    for k, v in data.items():
                        if k == "thresholds" and isinstance(v, dict):
                            default_policy["thresholds"].update(v)
                        else:
                            default_policy[k] = v
                return default_policy, resolved_path
        except Exception:
            return default_policy, resolved_path
    return default_policy, resolved_path

def deployment_frequency(releases, period_days=30, now_dt=None, policy=None):
    if now_dt is None:
        now_dt = datetime.now(timezone.utc)
    cutoff = now_dt.timestamp() - (period_days * 86400)
    recent = []
    for r in releases:
        try:
            published_str = r['published'].replace('Z', '+00:00')
            if datetime.fromisoformat(published_str).timestamp() > cutoff:
                recent.append(r)
        except (ValueError, KeyError):
            continue

    per_day = len(recent) / period_days
    thresholds = policy.get('thresholds', {}).get('deployment_frequency_per_day', {}) if policy else {}
    elite_t = thresholds.get('elite', 1.0)
    high_t = thresholds.get('high', 1 / 7)
    medium_t = thresholds.get('medium', 1 / 30)

    if per_day >= elite_t: tier = 'Elite'
    elif per_day >= high_t: tier = 'High'
    elif per_day >= medium_t: tier = 'Medium'
    else: tier = 'Low'
    return {'count': len(recent), 'per_day': round(per_day, 3), 'tier': tier}

def change_lead_time(prs, period_days=30, now_dt=None, policy=None):
    if now_dt is None:
        now_dt = datetime.now(timezone.utc)
    cutoff = now_dt.timestamp() - (period_days * 86400)
    durations = []
    for pr in prs:
        try:
            merged_str = pr['merged'].replace('Z', '+00:00')
            merged_dt = datetime.fromisoformat(merged_str)
            if merged_dt.timestamp() < cutoff:
                continue

            created_str = pr['created'].replace('Z', '+00:00')
            created_dt = datetime.fromisoformat(created_str)
            durations.append((merged_dt - created_dt).total_seconds() / 3600)
        except (ValueError, KeyError, TypeError):
            continue

    avg_hours = mean(durations) if durations else 0
    thresholds = policy.get('thresholds', {}).get('change_lead_time_hours', {}) if policy else {}
    elite_t = thresholds.get('elite', 1.0)
    high_t = thresholds.get('high', 24.0)
    medium_t = thresholds.get('medium', 168.0)

    if not durations: tier = 'N/A'
    elif avg_hours < elite_t: tier = 'Elite'
    elif avg_hours < high_t: tier = 'High'
    elif avg_hours < medium_t: tier = 'Medium'
    else: tier = 'Low'
    return {'avg_hours': round(avg_hours, 2), 'tier': tier}

def change_failure_rate(dora_metrics, policy=None):
    hotfixes = sum(1 for m in dora_metrics if m.get('metric') == 'change_failure' or m.get('type') == 'hotfix')
    deployments = sum(1 for m in dora_metrics if m.get('metric') == 'deployment')
    total = hotfixes + deployments

    rate = hotfixes / total if total > 0 else 0
    thresholds = policy.get('thresholds', {}).get('change_failure_rate', {}) if policy else {}
    elite_t = thresholds.get('elite', 0.05)
    high_t = thresholds.get('high', 0.10)
    medium_t = thresholds.get('medium', 0.15)

    if total == 0: tier = 'N/A'
    elif rate <= elite_t: tier = 'Elite'
    elif rate <= high_t: tier = 'High'
    elif rate <= medium_t: tier = 'Medium'
    else: tier = 'Low'
    return {'hotfixes': hotfixes, 'total': total, 'rate': round(rate, 3), 'tier': tier}

def failed_deployment_recovery_time(dora_metrics, policy=None):
    fdrt_events = [m.get('fdrt_hours') for m in dora_metrics if m.get('metric') == 'fdrt' and 'fdrt_hours' in m]
    avg_hours = mean(fdrt_events) if fdrt_events else 0
    thresholds = policy.get('thresholds', {}).get('fdrt_hours', {}) if policy else {}
    elite_t = thresholds.get('elite', 1.0)
    high_t = thresholds.get('high', 24.0)
    medium_t = thresholds.get('medium', 168.0)

    if not fdrt_events: tier = 'N/A'
    elif avg_hours < elite_t: tier = 'Elite'
    elif avg_hours < high_t: tier = 'High'
    elif avg_hours < medium_t: tier = 'Medium'
    else: tier = 'Low'
    return {'avg_hours': round(avg_hours, 2), 'tier': tier}

def agentic_metrics(agent_metrics):
    total = len(agent_metrics)
    successes = sum(1 for m in agent_metrics if m.get('success'))
    interventions = sum(m.get('human_interventions', 0) for m in agent_metrics)
    tokens = sum(m.get('tokens_used', 0) for m in agent_metrics)

    return {
        'total_tasks': total,
        'success_rate': round(successes / total, 3) if total > 0 else 0,
        'human_intervention_rate': round(interventions / total, 3) if total > 0 else 0,
        'avg_tokens': round(tokens / total) if total > 0 else 0
    }

def get_overall_tier(tiers):
    tier_priority = {'Elite': 4, 'High': 3, 'Medium': 2, 'Low': 1, 'N/A': 0}
    valid_tiers = [t for t in tiers if t != 'N/A']
    if not valid_tiers:
        return 'Unknown'

    min_priority = min(tier_priority[t] for t in valid_tiers)
    for name, priority in tier_priority.items():
        if priority == min_priority:
            return name
    return 'Unknown'

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--releases', required=True)
    parser.add_argument('--prs', required=True)
    parser.add_argument('--agent-metrics', required=True)
    parser.add_argument('--dora-metrics', required=True)
    parser.add_argument('--output', required=True)
    parser.add_argument('--template', required=True)
    parser.add_argument('--policy', help='Path to DORA policy JSON file (e.g. plans/dora.json)')
    parser.add_argument('--manifest-output', help='Path to output derivation manifest JSON file')
    parser.add_argument('--period-days', type=int, help='Evaluation window in days (overrides policy if set)')
    parser.add_argument('--repo', default='unknown/repo')
    parser.add_argument('--now', help='Reference ISO 8601 timestamp for evaluation period (e.g. 2026-09-17T00:00:00Z)')
    args = parser.parse_args()

    policy, resolved_policy_path = load_policy(args.policy)
    period_days = args.period_days if args.period_days is not None else policy.get('period_days', 30)

    now_dt = None
    if args.now:
        now_str = args.now.replace('Z', '+00:00')
        now_dt = datetime.fromisoformat(now_str)
        if now_dt.tzinfo is None:
            now_dt = now_dt.replace(tzinfo=timezone.utc)
    else:
        now_dt = datetime.now(timezone.utc)

    start_dt = datetime.fromtimestamp(now_dt.timestamp() - (period_days * 86400), tz=timezone.utc)
    now_iso = now_dt.strftime('%Y-%m-%dT%H:%M:%SZ')
    start_iso = start_dt.strftime('%Y-%m-%dT%H:%M:%SZ')

    releases = load_json(args.releases, required=True)
    prs = load_json(args.prs, required=True)
    agent_metrics_data = load_jsonl(args.agent_metrics)
    dora_metrics_data = load_jsonl(args.dora_metrics)

    df = deployment_frequency(releases, period_days, now_dt=now_dt, policy=policy)
    clt = change_lead_time(prs, period_days, now_dt=now_dt, policy=policy)
    cfr = change_failure_rate(dora_metrics_data, policy=policy)
    fdrt = failed_deployment_recovery_time(dora_metrics_data, policy=policy)
    am = agentic_metrics(agent_metrics_data)

    overall_tier = get_overall_tier([df['tier'], clt['tier'], cfr['tier'], fdrt['tier']])

    input_hashes = {
        'releases': file_sha256(args.releases),
        'prs': file_sha256(args.prs),
        'agent_metrics': file_sha256(args.agent_metrics),
        'dora_metrics': file_sha256(args.dora_metrics),
        'policy': file_sha256(resolved_policy_path) if resolved_policy_path else file_sha256(None)
    }

    manifest = {
        "schema_version": "1.0",
        "evaluation_window": {
            "now_iso": now_iso,
            "start_iso": start_iso,
            "period_days": period_days
        },
        "scanned_counts": {
            "releases": len(releases),
            "prs": len(prs),
            "agent_metrics": len(agent_metrics_data),
            "dora_metrics": len(dora_metrics_data)
        },
        "policy": {
            "policy_file": resolved_policy_path,
            "bot_allowlist": policy.get("bot_allowlist", []),
            "merge_strategy": policy.get("merge_strategy", "squash_rebase_merge"),
            "percentile_method": policy.get("percentile_method", "mean"),
            "revert_predicate": policy.get("revert_predicate", "title_contains_revert")
        },
        "input_hashes": input_hashes,
        "metrics": {
            "deployment_frequency": df,
            "change_lead_time": clt,
            "change_failure_rate": cfr,
            "failed_deployment_recovery_time": fdrt,
            "agentic_metrics": am,
            "overall_tier": overall_tier
        }
    }

    manifest_output_path = args.manifest_output
    if not manifest_output_path:
        out_dir = os.path.dirname(args.output)
        if out_dir:
            manifest_output_path = os.path.join(out_dir, "dora-manifest.json")
        else:
            manifest_output_path = "reports/dora-manifest.json"

    if manifest_output_path:
        manifest_dir = os.path.dirname(manifest_output_path)
        if manifest_dir:
            os.makedirs(manifest_dir, exist_ok=True)
        with open(manifest_output_path, 'w') as f:
            json.dump(manifest, f, indent=2, sort_keys=True)

    badges = {
        'Elite': '![Elite](https://img.shields.io/badge/DORA-Elite-brightgreen)',
        'High': '![High](https://img.shields.io/badge/DORA-High-green)',
        'Medium': '![Medium](https://img.shields.io/badge/DORA-Medium-yellow)',
        'Low': '![Low](https://img.shields.io/badge/DORA-Low-orange)',
        'Unknown': '![Unknown](https://img.shields.io/badge/DORA-Unknown-lightgrey)'
    }
    overall_tier_badge = badges.get(overall_tier, badges['Unknown'])

    if os.path.exists(args.template):
        with open(args.template, 'r') as f:
            template_content = f.read()
    else:
        template_content = "# DORA Report Placeholder"

    report = template_content
    report = report.replace('{{ timestamp }}', now_iso)
    report = report.replace('{{ start_iso }}', start_iso)
    report = report.replace('{{ period_days }}', str(period_days))
    report = report.replace('{{ repo }}', args.repo)

    report = report.replace('{{ df.per_day }}', str(df['per_day']))
    report = report.replace('{{ df.count }}', str(df['count']))
    report = report.replace('{{ df.tier }}', df['tier'])

    report = report.replace('{{ clt.avg_hours }}', str(clt['avg_hours']))
    report = report.replace('{{ clt.tier }}', clt['tier'])

    report = report.replace('{{ cfr.rate * 100 }}', str(round(cfr['rate'] * 100, 1)))
    report = report.replace('{{ cfr.hotfixes }}', str(cfr['hotfixes']))
    report = report.replace('{{ cfr.total }}', str(cfr['total']))
    report = report.replace('{{ cfr.tier }}', cfr['tier'])

    report = report.replace('{{ fdrt.avg_hours }}', str(fdrt['avg_hours']))
    report = report.replace('{{ fdrt.tier }}', fdrt['tier'])

    report = report.replace('{{ am.total_tasks }}', str(am['total_tasks']))
    report = report.replace('{{ am.success_rate * 100 }}', str(round(am['success_rate'] * 100, 1)))
    report = report.replace('{{ am.human_intervention_rate * 100 }}', str(round(am['human_intervention_rate'] * 100, 1)))
    report = report.replace('{{ am.avg_tokens }}', str(am['avg_tokens']))

    report = report.replace('{{ overall_tier_badge }}', overall_tier_badge)

    bot_allowlist_str = ", ".join(manifest["policy"]["bot_allowlist"]) if manifest["policy"]["bot_allowlist"] else "none"
    report = report.replace('{{ policy.percentile_method }}', manifest["policy"]["percentile_method"])
    report = report.replace('{{ policy.revert_predicate }}', manifest["policy"]["revert_predicate"])
    report = report.replace('{{ policy.bot_allowlist_str }}', bot_allowlist_str)
    report = report.replace('{{ input_hashes.releases }}', input_hashes['releases'])
    report = report.replace('{{ input_hashes.prs }}', input_hashes['prs'])
    report = report.replace('{{ input_hashes.policy }}', input_hashes['policy'])

    out_dir = os.path.dirname(args.output)
    if out_dir:
        os.makedirs(out_dir, exist_ok=True)

    with open(args.output, 'w') as f:
        f.write(report)

if __name__ == '__main__':
    main()
