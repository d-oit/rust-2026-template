import json
import argparse
from datetime import datetime, timezone
from statistics import mean
import os

def load_json(path):
    if not os.path.exists(path):
        return []
    with open(path, 'r') as f:
        try:
            return json.load(f)
        except json.JSONDecodeError:
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

def deployment_frequency(releases, period_days=30):
    cutoff = datetime.now(timezone.utc).timestamp() - (period_days * 86400)
    recent = []
    for r in releases:
        try:
            published_str = r['published'].replace('Z', '+00:00')
            if datetime.fromisoformat(published_str).timestamp() > cutoff:
                recent.append(r)
        except (ValueError, KeyError):
            continue

    per_day = len(recent) / period_days
    if per_day >= 1: tier = 'Elite'
    elif per_day >= 1/7: tier = 'High'
    elif per_day >= 1/30: tier = 'Medium'
    else: tier = 'Low'
    return {'count': len(recent), 'per_day': round(per_day, 3), 'tier': tier}

def change_lead_time(prs, period_days=30):
    cutoff = datetime.now(timezone.utc).timestamp() - (period_days * 86400)
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
    if not durations: tier = 'N/A'
    elif avg_hours < 1: tier = 'Elite'
    elif avg_hours < 24: tier = 'High'
    elif avg_hours < 168: tier = 'Medium'
    else: tier = 'Low'
    return {'avg_hours': round(avg_hours, 2), 'tier': tier}

def change_failure_rate(dora_metrics):
    hotfixes = sum(1 for m in dora_metrics if m.get('metric') == 'change_failure' or m.get('type') == 'hotfix')
    deployments = sum(1 for m in dora_metrics if m.get('metric') == 'deployment')
    total = hotfixes + deployments

    rate = hotfixes / total if total > 0 else 0
    if total == 0: tier = 'N/A'
    elif rate <= 0.05: tier = 'Elite'
    elif rate <= 0.10: tier = 'High'
    elif rate <= 0.15: tier = 'Medium'
    else: tier = 'Low'
    return {'hotfixes': hotfixes, 'total': total, 'rate': round(rate, 3), 'tier': tier}

def failed_deployment_recovery_time(dora_metrics):
    fdrt_events = [m.get('fdrt_hours') for m in dora_metrics if m.get('metric') == 'fdrt' and 'fdrt_hours' in m]
    avg_hours = mean(fdrt_events) if fdrt_events else 0
    if not fdrt_events: tier = 'N/A'
    elif avg_hours < 1: tier = 'Elite'
    elif avg_hours < 24: tier = 'High'
    elif avg_hours < 168: tier = 'Medium'
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
    parser.add_argument('--period-days', type=int, default=30)
    parser.add_argument('--repo', default='unknown/repo')
    args = parser.parse_args()

    releases = load_json(args.releases)
    prs = load_json(args.prs)
    agent_metrics_data = load_jsonl(args.agent_metrics)
    dora_metrics_data = load_jsonl(args.dora_metrics)

    df = deployment_frequency(releases, args.period_days)
    clt = change_lead_time(prs, args.period_days)
    cfr = change_failure_rate(dora_metrics_data)
    fdrt = failed_deployment_recovery_time(dora_metrics_data)
    am = agentic_metrics(agent_metrics_data)

    overall_tier = get_overall_tier([df['tier'], clt['tier'], cfr['tier'], fdrt['tier']])

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
    report = report.replace('{{ timestamp }}', datetime.now(timezone.utc).strftime('%Y-%m-%dT%H:%M:%SZ'))
    report = report.replace('{{ period_days }}', str(args.period_days))
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

    with open(args.output, 'w') as f:
        f.write(report)

if __name__ == '__main__':
    main()
