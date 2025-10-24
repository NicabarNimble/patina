# Scryer Prolog Query Examples

Load the knowledge base and run queries:

```bash
cd layer/buckets/patina-dev
scryer-prolog facts.pl rules.pl
```

## Basic Fact Queries

### All sessions
```prolog
?- session(ID, Date, Type, Branch, Commits, Files),
   write(ID), write(' - '), write(Type), nl, fail.
```

### All patterns
```prolog
?- pattern_observed(Session, Pattern, Category),
   write(Pattern), write(' ('), write(Category), write(')'), nl, fail.
```

## Pattern Analysis

### Security patterns
```bash
scryer-prolog facts.pl rules.pl -g "pattern_in_category(P, security), write(P), nl, fail; halt."
```

Output:
```
tmpfs-for-secrets
1password-integration
credential-management
security-review-generated-code
```

### Architecture patterns
```bash
scryer-prolog facts.pl rules.pl -g "pattern_in_category(P, architecture), write(P), nl, fail; halt."
```

Output:
```
neuro-symbolic-persona
domain-buckets
tool-vs-system-distinction
pattern-selection-framework
git-aware-navigation
```

### Workflow patterns
```bash
scryer-prolog facts.pl rules.pl -g "pattern_in_category(P, workflow), write(P), nl, fail; halt."
```

## Session Analysis

### Security-focused sessions
```bash
scryer-prolog facts.pl rules.pl -g "security_session(S), write(S), nl, fail; halt."
```

Output:
```
20251008-061520
20251007-210232
20251007-185647
```

### Architecture-focused sessions
```bash
scryer-prolog facts.pl rules.pl -g "architecture_session(S), write(S), nl, fail; halt."
```

Output:
```
20251010-061739
20250813-055742
20250809-211749
```

### Productive sessions (made commits)
```bash
scryer-prolog facts.pl rules.pl -g "productive_session(S), write(S), nl, fail; halt."
```

### Exploratory sessions (no commits)
```bash
scryer-prolog facts.pl rules.pl -g "exploratory_session(S), write(S), nl, fail; halt."
```

## Technology Analysis

### All technologies used
```bash
scryer-prolog facts.pl rules.pl -g "tech_used(S, Tech, Purpose), write(Tech), write(' - '), write(Purpose), nl, fail; halt."
```

### Technologies for security
```bash
scryer-prolog facts.pl rules.pl -g "tech_for_purpose(T, P), sub_atom(P, _, _, _, security), write(T), nl, fail; halt."
```

## Decision Analysis

### All decisions
```bash
scryer-prolog facts.pl rules.pl -g "decision(S, Choice, Rationale), write(Choice), nl, fail; halt."
```

### Pragmatic decisions
```bash
scryer-prolog facts.pl rules.pl -g "pragmatic_decision(S, Choice), write(S), write(': '), write(Choice), nl, fail; halt."
```

## Challenge/Solution Mapping

### All challenges
```bash
scryer-prolog facts.pl rules.pl -g "challenge(S, Problem, Solution), write(Problem), write(' -> '), write(Solution), nl, fail; halt."
```

## Workflow Chains

### Sessions on same branch
```bash
scryer-prolog facts.pl rules.pl -g "workflow_chain('20251008-061520', S2), write(S2), nl, fail; halt."
```

### Pattern correlations
```bash
scryer-prolog facts.pl rules.pl -g "pattern_correlation(P1, P2), write(P1), write(' + '), write(P2), nl, fail; halt."
```

## Cross-Domain Analysis

### Domain dependencies
```bash
scryer-prolog facts.pl rules.pl -g "depends_on_domain('patina-dev', D), write(D), nl, fail; halt."
```

## Interactive Mode

Start interactive REPL:
```bash
scryer-prolog facts.pl rules.pl
```

Then try queries interactively:
```prolog
?- security_session(S).
S = '20251008-061520' ;
S = '20251007-210232' ;
S = '20251007-185647'.

?- pattern_in_category(P, security).
P = 'tmpfs-for-secrets' ;
P = '1password-integration' ;
P = 'credential-management' ;
P = 'security-review-generated-code'.
```
