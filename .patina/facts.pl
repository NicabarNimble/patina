% Facts extracted from Patina sessions
% This represents canonical knowledge from layer/sessions/

% Session metadata
% session(id, date, work_type, branch, commits, files_changed)
session('20251010-061739', '2025-10-10', exploration, 'feature/repo-cleanup-action', 0, 5).
session('20251009-064522', '2025-10-09', 'pattern-work', 'feature/yolo-command', 1, 23).
session('20251008-061520', '2025-10-08', experiment, 'feature/yolo-command', 2, 2).
session('20251007-210232', '2025-10-07', exploration, 'feature/yolo-command', 0, 2).
session('20251007-185647', '2025-10-07', 'pattern-work', 'feature/yolo-command', 2, 2).
session('20250813-055742', '2025-08-13', exploration, 'finalize-blackbox-refactor', 0, 0).
session('20250809-211749', '2025-08-09', experiment, main, 0, 0).

% Pattern observations
% pattern_observed(session_id, pattern_name, category)
pattern_observed('20251010-061739', 'neuro-symbolic-persona', architecture).
pattern_observed('20251010-061739', 'domain-buckets', architecture).
pattern_observed('20251009-064522', 'patina-branch-strategy', workflow).
pattern_observed('20251009-064522', 'public-repo-for-automation', infrastructure).
pattern_observed('20251008-061520', 'tmpfs-for-secrets', security).
pattern_observed('20251008-061520', '1password-integration', security).
pattern_observed('20251007-210232', 'credential-management', security).
pattern_observed('20251007-185647', 'security-review-generated-code', security).
pattern_observed('20250813-055742', 'tool-vs-system-distinction', architecture).
pattern_observed('20250813-055742', 'pattern-selection-framework', architecture).
pattern_observed('20250809-211749', 'git-aware-navigation', architecture).

% Technology used
% tech_used(session_id, technology, purpose)
tech_used('20251009-064522', 'github-actions', 'CI/CD automation').
tech_used('20251009-064522', 'github-projects', 'repo cleanup tracking').
tech_used('20251008-061520', '1password-cli', 'secure credential storage').
tech_used('20251008-061520', 'ignore-crate', 'gitignore-aware file scanning').
tech_used('20251007-210232', 'bitwarden', 'credential management research').
tech_used('20250809-211749', 'sqlite', 'semantic search').
tech_used('20250809-211749', 'rayon', 'parallel indexing').

% Key decisions
% decision(session_id, choice, rationale)
decision('20251010-061739', 'markdown-is-generated-output', 'canonical data in facts.db + rules.pl').
decision('20251009-064522', 'public-automation-repo', 'unlimited GitHub Actions vs 3000 min/month').
decision('20251008-061520', '1password-over-bitwarden', 'already installed, less friction').
decision('20251007-210232', 'defer-credential-provider', 'sleep on philosophy vs pragmatism').
decision('20250813-055742', 'pattern-polymorphism', 'different patterns for tools vs systems').

% Challenges faced and solutions
% challenge(session_id, problem, solution)
challenge('20251009-064522', 'yaml-markdown-conflict', 'extract bash to external script').
challenge('20251008-061520', 'kit-scanner-hang', 'replace glob with ignore crate').
challenge('20251007-210232', 'plaintext-credentials', 'researched vault solutions').
challenge('20251007-185647', 'hardcoded-home-path', 'use ${HOME} env var').
challenge('20250813-055742', 'workspace-import-cycles', 'recognized wrong pattern for unstable code').

% Cross-domain references (the unsolved problem!)
% domain_link(source_domain, target_domain, relationship)
domain_link('patina-dev', 'rust-development', 'implements-patterns-from').
domain_link('patina-dev', 'devops', 'uses-tools-from').
domain_link('patina-dev', 'security', 'applies-patterns-from').
