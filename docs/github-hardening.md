# GitHub Hardening Proposal

This document provides guidance for securing repositories created from this template. It covers optional protections for harness files, hooks, workflow definitions, and related governance files.

> **Note:** This is advisory documentation, not enforced policy. Repository owners should adapt these recommendations to their team structure, governance model, and security posture. GitHub branch protection rules and several repository security settings do not automatically carry over when a new repository is created from a template.

## Sensitive Paths

The following files and directories typically deserve stronger review and governance in repositories created from this template. Changes to these paths can influence repository behavior, trust boundaries, and CI/CD integrity.

### Agent and Harness Files

| Path | Purpose | Risk |
|------|---------|------|
| `HARNESS.md` | Harness engineering guide — maps sensors and feedforward guides | Modifies agent behavior and self-correction protocols |
| `AGENTS.md` | Canonical instructions for AI coding agents | Root coding conventions, change workflow, quality gates |
| `.agents/**` | AI agent skills, context, and workflow definitions | Executable task knowledge that agents follow |
| `.agents/skills/**` | Specialized skill definitions | Step-by-step procedures agents execute |
| `.agents/context/**` | Cross-repo context for derived repositories | Shared conventions and repository links |

### Git Hooks and Scripts

| Path | Purpose | Risk |
|------|---------|------|
| `.githooks/**` | Local git hook scripts | Runs on developer machines, affects commit quality |
| `hooks/**` | Additional hook scripts | May execute during CI or local workflows |
| `scripts/**` | Automation scripts | Quality gates, releases, and development workflows |

### CI/CD and Workflows

| Path | Purpose | Risk |
|------|---------|------|
| `.github/workflows/**` | GitHub Actions workflow definitions | CI/CD pipeline, automated releases, security scans |
| `.github/dependabot.yml` | Dependabot configuration | Automated dependency updates |
| `.github/CODEOWNERS` | Code review ownership | Review requirements for sensitive paths |

### Governance and Security Configuration

| Path | Purpose | Risk |
|------|---------|------|
| `.pre-commit-config.yaml` | Pre-commit hook configuration | Local quality enforcement |
| `deny.toml` | `cargo-deny` configuration | License and dependency policy |
| `.gitleaks.toml` | Secret scanning configuration | Secret detection rules |
| `.clippy.toml` | Clippy lint configuration | Code quality standards |
| `Cargo.toml` | Workspace manifest and lint configuration | Dependency policy, unsafe code settings |
| `commitlint.config.cjs` | Commit message linting | Conventional commit enforcement |

## Optional CODEOWNERS Usage

The `CODEOWNERS` file can automatically request reviews from the right maintainers when pull requests modify sensitive files. This is especially useful for governance files that affect repository behavior and trust boundaries.

### Why CODEOWNERS Matters for This Template

Repositories created from this template contain files that influence:
- **CI/CD behavior** — workflow definitions can execute arbitrary code
- **Agent behavior** — harness and skill definitions guide AI assistants
- **Security posture** — dependency policies and secret scanning rules
- **Code quality standards** — lint configurations and quality gates

Requiring review from maintainers who understand these areas helps prevent accidental or malicious modifications.

### Example Patterns

The following examples show common patterns. Each repository should adapt these to its own team structure and governance model.

**Pattern 1: Centralized governance review**

```text
# Governance files require review from maintainers
HARNESS.md          @your-org/governance-maintainers
AGENTS.md           @your-org/governance-maintainers
.agents/            @your-org/governance-maintainers
.githooks/          @your-org/governance-maintainers
scripts/            @your-org/governance-maintainers

# CI/CD requires DevOps review
.github/workflows/  @your-org/devops-team
```

**Pattern 2: Fine-grained ownership**

```text
# Harness and agent files
HARNESS.md                    @your-org/harness-maintainers
AGENTS.md                     @your-org/harness-maintainers
.agents/skills/**             @your-org/skill-maintainers
.agents/context/**            @your-org/context-maintainers

# Hooks and scripts
.githooks/**                  @your-org/hooks-maintainers
scripts/quality-gates.sh      @your-org/quality-maintainers
scripts/release*.sh           @your-org/release-maintainers

# CI/CD
.github/workflows/ci.yml     @your-org/ci-maintainers
.github/workflows/release.yml @your-org/release-maintainers
.github/workflows/security*.yml @your-org/security-maintainers

# Security configuration
deny.toml                     @your-org/security-maintainers
.gitleaks.toml                @your-org/security-maintainers
```

**Pattern 3: Minimal CODEOWNERS**

```text
# Require review for all governance and CI/CD changes
HARNESS.md       @your-org/maintainers
AGENTS.md        @your-org/maintainers
.agents/**       @your-org/maintainers
.github/**       @your-org/maintainers
scripts/**       @your-org/maintainers
```

### Implementation Notes

1. **No hardcoded teams in template** — The template intentionally does not include a `CODEOWNERS` file. Each repository must create its own based on team structure.

2. **CODEOWNERS requires branch protection** — For CODEOWNERS to enforce reviews, enable "Require review from Code Owners" in branch protection settings.

3. **File pattern syntax** — GitHub supports glob patterns (`*`, `**`, `?`) for matching multiple files. See [GitHub CODEOWNERS documentation](https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/about-code-owners) for syntax details.

4. **Multiple owners** — A file can have multiple owners. All listed owners will be requested for review.

## Optional Branch Protection and Required Checks

Branch protection rules help maintain code quality and prevent accidental or unauthorized changes to critical branches. The following recommendations provide a baseline for repositories created from this template.

> **Important:** Branch protection settings are configured in GitHub repository settings and are not inherited from template repositories. Each repository must configure these settings after creation.

### Recommended Baseline

For the default branch (typically `main`), consider enabling:

| Setting | Recommendation | Rationale |
|---------|----------------|-----------|
| **Require pull request reviews** | Enable | Prevents direct commits to main |
| **Required approving reviews** | 1-2 reviewers | Ensures code review before merge |
| **Dismiss stale reviews** | Enable | Prevents approval of outdated changes |
| **Require review from Code Owners** | Optional | Enforce review for sensitive paths |
| **Require status checks** | Enable | Ensure CI passes before merge |
| **Require branches to be up to date** | Enable | Prevent merge conflicts |
| **Require conversation resolution** | Enable | Ensure all comments are addressed |
| **Require linear history** | Optional | Enforce rebase or squash merges |
| **Include administrators** | Consider | Apply rules even to repo admins |

### Required Status Checks

The template's CI pipeline (`.github/workflows/ci.yml`) runs multiple checks. Consider requiring these before merge:

**Essential checks:**
- `fmt` — Code formatting
- `clippy` — Lint warnings
- `build` — Compilation
- `test` — Unit and integration tests
- `doc-test` — Documentation tests
- `security-audit` — Known vulnerabilities (`cargo audit`)
- `deny` — License and supply chain policy (`cargo deny`)

**Optional checks:**
- `unused-deps` — Unused dependency detection
- `privacy-scan` — Personal data detection
- `secret-scan` — Secret detection (`gitleaks`)

### Branch Protection Configuration

Configure branch protection via:

1. **GitHub UI**: Repository Settings → Branches → Add branch protection rule
2. **GitHub CLI**: `gh api repos/{owner}/{repo}/branches/{branch}/protection`
3. **Terraform**: `github_branch_protection` resource
4. **Organization-level**: Organization Settings → Repository → Branch protection policies

### Example: GitHub CLI Configuration

```bash
# Protect main branch with required reviews and status checks
gh api repos/{owner}/{repo}/branches/main/protection \
  --method PUT \
  --field required_status_checks='{"strict":true,"contexts":["fmt","clippy","build","test"]}' \
  --field enforce_admins=true \
  --field required_pull_request_reviews='{"required_approving_review_count":1,"dismiss_stale_reviews":true,"require_code_owner_reviews":true}' \
  --field restrictions=null
```

### Example: Terraform Configuration

```hcl
resource "github_branch_protection" "main" {
  repository_id = github_repository.repo.node_id
  pattern       = "main"

  enforce_admins = true

  required_pull_request_reviews {
    required_approving_review_count = 1
    dismiss_stale_reviews          = true
    require_code_owner_reviews     = true
  }

  required_status_checks {
    strict   = true
    contexts = ["fmt", "clippy", "build", "test", "security-audit"]
  }

  allows_force_pushes  = false
  allows_deletions     = false
}
```

## Optional GitHub Security Features

GitHub provides several security features that repositories created from this template may want to enable, depending on plan and organizational needs.

### Dependabot

**Dependency Graph:**
- Automatically detects dependencies from `Cargo.toml`
- Visualizes dependency tree and transitive dependencies
- Enable in: Repository Settings → Code security and analysis → Dependency graph

**Dependabot Alerts:**
- Notifies when dependencies have known vulnerabilities
- Automatically configured in `.github/dependabot.yml`
- Enable in: Repository Settings → Code security and analysis → Dependabot alerts

**Dependabot Security Updates:**
- Automatically creates PRs to update vulnerable dependencies
- Requires Dependabot alerts to be enabled
- Enable in: Repository Settings → Code security and analysis → Dependabot security updates

### Secret Scanning

**Push Protection:**
- Blocks commits containing detected secrets
- Works with pre-commit hooks and CI secret scanning
- Enable in: Repository Settings → Code security and analysis → Secret scanning → Push protection

**Secret Scanning Alerts:**
- Scans repository for exposed secrets
- Integrates with `.gitleaks.toml` configuration
- Enable in: Repository Settings → Code security and analysis → Secret scanning

**Partner Patterns:**
- GitHub partners (AWS, Azure, GCP, etc.) detect their specific token formats
- Custom patterns can be added via repository or organization settings

### Code Scanning

**CodeQL Analysis:**
- Semantic code analysis for security vulnerabilities
- Can detect Rust-specific issues (though limited compared to other languages)
- Enable via: Repository Settings → Code security and analysis → Code scanning

**Third-Party Tools:**
- Codacy, SonarQube, and other tools can integrate via GitHub Apps
- See `.codacy/` configuration in this template for examples

### Security Advisories

- Private vulnerability reporting (already configured in `SECURITY.md`)
- Security advisory creation and CVE assignment
- Enable in: Repository Settings → Code security and analysis → Private vulnerability reporting

### Security Recommendations by Plan

| Feature | Free | Team | Enterprise |
|---------|------|------|------------|
| Dependency graph | ✅ | ✅ | ✅ |
| Dependabot alerts | ✅ | ✅ | ✅ |
| Dependabot security updates | ✅ | ✅ | ✅ |
| Secret scanning | ✅ | ✅ | ✅ |
| Push protection | ✅ | ✅ | ✅ |
| Code scanning (CodeQL) | ✅ | ✅ | ✅ |
| Security advisories | ✅ | ✅ | ✅ |
| Organization-level policies | ❌ | ✅ | ✅ |
| Advanced security features | ❌ | ❌ | ✅ |

## Hooks and Harness Philosophy

This template implements a layered approach to code quality and security enforcement. Understanding this philosophy helps repository owners decide which protections to enable and how to configure them.

### Three Layers of Enforcement

```
┌─────────────────────────────────────────────────────────┐
│                    CI Pipeline                           │
│         (.github/workflows/*.yml)                       │
│    Authoritative enforcement — blocks merge              │
└─────────────────────────────────────────────────────────┘
                          ▲
                          │
┌─────────────────────────────────────────────────────────┐
│                  Local Hooks                            │
│         (.githooks/**, .pre-commit-config.yaml)         │
│    Developer ergonomics — catches problems early         │
└─────────────────────────────────────────────────────────┘
                          ▲
                          │
┌─────────────────────────────────────────────────────────┐
│               Feedforward Guides                        │
│         (AGENTS.md, HARNESS.md, .agents/**)             │
│    Context and conventions — shapes behavior             │
└─────────────────────────────────────────────────────────┘
```

### Local Hooks: Developer Ergonomics

**Purpose:** Catch common issues before commit, provide fast feedback during development.

**Characteristics:**
- Run on developer machines
- Fast execution (seconds)
- Focus on formatting, linting, basic validation
- Can be bypassed (though discouraged)

**Examples from this template:**
- `cargo fmt --check` — Format validation
- `cargo clippy` — Lint warnings
- `cargo deny check` — License and dependency policy
- `shellcheck` — Shell script validation
- `commitlint` — Conventional commit format

**Trust level:** Advisory. Developers can bypass if needed, but should not.

### CI Pipeline: Authoritative Enforcement

**Purpose:** Final quality gate before merge, security scanning, comprehensive testing.

**Characteristics:**
- Runs on CI servers
- Longer execution (minutes)
- Full test suite, security audits, integration tests
- Cannot be bypassed (with branch protection enabled)

**Examples from this template:**
- Full test suite (`cargo nextest run`)
- Security audit (`cargo audit`)
- Supply chain check (`cargo deny check`)
- Secret scanning (`gitleaks`)
- Mutation testing (periodic)

**Trust level:** Authoritative. Must pass before merge.

### Feedforward Guides: Context and Conventions

**Purpose:** Provide context, constraints, and conventions that shape developer and agent behavior.

**Characteristics:**
- Documentation and configuration
- Read by developers and AI agents
- Define coding standards, architecture decisions, workflow rules
- Updated through governance process

**Examples from this template:**
- `AGENTS.md` — Root coding conventions and change workflow
- `HARNESS.md` — Maps sensors and feedforward guides
- `.agents/skills/` — Executable task knowledge
- `deny.toml` — Dependency policy
- `.clippy.toml` — Lint configuration

**Trust level:** Inferential. Guides behavior but doesn't enforce.

### Trust Boundaries

Understanding trust boundaries helps repository owners configure appropriate protections:

| Boundary | Trust Level | Protection |
|----------|-------------|------------|
| Developer machine | Moderate | Local hooks, IDE integration |
| Pull request | High | Code review, CI checks |
| Merge to main | Very high | Branch protection, required reviews |
| Release | Critical | Signed commits, automated releases |
| Supply chain | Variable | Dependency auditing, pinning |

### Security Considerations for Governance Files

Governance files (HARNESS.md, AGENTS.md, .agents/**, .github/workflows/**) deserve extra scrutiny because they can:

1. **Modify CI behavior** — Workflow definitions can execute arbitrary code
2. **Guide agent behavior** — Skill definitions tell AI assistants what to do
3. **Enforce or bypass checks** — Hook scripts can skip quality gates
4. **Access secrets** — Workflows may have access to repository secrets

**Recommendation:** Require review from trusted maintainers for changes to governance files. Consider using CODEOWNERS to enforce this.

### Self-Correction Protocol

When a sensor fires (CI or local hook), the harness includes fix hints to guide correction:

1. Read the full error message
2. Identify the category (fmt / lint / test / arch / security)
3. Apply the minimal fix
4. Re-run the specific sensor
5. Only commit when the sensor is green

This protocol works across all three layers: local hooks provide immediate feedback, CI provides authoritative verification, and feedforward guides provide context for fixing issues.

## Implementation Checklist

Use this checklist when creating a repository from this template:

### Immediate (Day 1)

- [ ] Update `SECURITY.md` with your project's security contact
- [ ] Review and customize `deny.toml` for your license requirements
- [ ] Enable Dependabot alerts in repository settings
- [ ] Enable secret scanning in repository settings

### Short-term (First Week)

- [ ] Configure branch protection on `main`
- [ ] Create `CODEOWNERS` file for governance files
- [ ] Enable push protection for secret scanning
- [ ] Review and customize `.gitleaks.toml` if needed

### Medium-term (First Month)

- [ ] Enable code scanning (CodeQL or third-party)
- [ ] Configure organization-level policies (if applicable)
- [ ] Set up security advisory workflow
- [ ] Review Dependabot security update configuration

### Ongoing

- [ ] Review and update CODEOWNERS as team changes
- [ ] Monitor Dependabot alerts and security updates
- [ ] Review governance file changes carefully
- [ ] Keep security documentation current

## Further Reading

- [GitHub Repository Security](https://docs.github.com/en/code-security/getting-started/quickstart-for-securing-your-repository)
- [About Code Owners](https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/about-code-owners)
- [Managing Protected Branches](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-protected-branches/managing-a-branch-protection-rule)
- [Dependabot Configuration](https://docs.github.com/en/code-security/dependabot/dependabot-version-updates/configuration-options-for-the-dependabot.yml-file)
- [Secret Scanning](https://docs.github.com/en/code-security/secret-scanning)
- [Code Scanning](https://docs.github.com/en/code-security/code-scanning)
