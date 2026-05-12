# Slate Child Skill Discovery

[[mother-brokers-child-skills]] sets the direction: Mother brokers child skills, while `AGENTS.md` only routes agents to the overarching Patina/Mother skill system.

## Desired Mother surface

```bash
patina mother skills list
patina mother skills show <child>
patina mother skills help <child> [skill]
```

For Slate:

```bash
patina mother skills show slate-manager
patina mother skills help slate-manager slate-code
patina mother skills help slate-manager slate-version-control
```

## Child package layout

Slate manager now lives outside the Patina monorepo:

```text
/Users/nicabar/Projects/Patina/patina-child-slate/
  child.toml
  skills/
    README.md
    slate-code/SKILL.md
    slate-version-control/SKILL.md
```

A child skill package is documentation plus command examples and lifecycle gates. It should behave like `--help`: Mother reports what an active child can do, which skills apply, and what command surface to use.

## Mother responsibilities

- discover active children;
- discover each child's declared skill packages;
- expose skill names, descriptions, and package paths;
- render full help for a requested child skill;
- avoid embedding child-specific policy in `AGENTS.md`.

## Slate responsibilities

- own Slate-specific skills under `/Users/nicabar/Projects/Patina/patina-child-slate/skills/`;
- keep command examples accurate with the child WIT operations;
- document version/archive semantics in `slate-version-control`;
- install skill packages with the child so Mother can broker them.

## Current surface

Mother can render installed child skills:

- `patina mother skills show slate-manager`
- `patina mother skills help slate-manager slate-code`
- `patina mother skills help slate-manager slate-version-control`
