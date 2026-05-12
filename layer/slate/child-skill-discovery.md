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

```text
children/slate-manager/
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

- own Slate-specific skills under `children/slate-manager/skills/`;
- keep command examples accurate with the child WIT operations;
- document version/archive semantics in `slate-version-control`;
- keep `.pi/skills/` files as temporary bridges only.

## Current bridge

Until the Mother surface exists, agents can read:

- `.pi/skills/patina-mother-system/SKILL.md`
- `.pi/skills/patina-slate-code/SKILL.md`
- `children/slate-manager/skills/slate-code/SKILL.md`
- `children/slate-manager/skills/slate-version-control/SKILL.md`
