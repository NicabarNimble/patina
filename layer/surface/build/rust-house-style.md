# Rust House Style
Typed, explicit, refactor-friendly Rust ("Gjengset-ish")

This repository prefers Rust that is:
- explicit about failure modes
- strongly typed over stringly typed
- easy to review and refactor
- deterministic in behavior and output
- safe by default

If you contribute code here (human or LLM), follow these rules.

---

## Core principles

### 1) Errors are part of the API
Treat error handling as a first-class design surface.

- Return `Result<T, E>` for fallible work.
- Add context at boundaries (I/O, DB, parse, network).
- Avoid patterns that silently erase errors.

### 2) Prefer types that encode meaning
Use types to represent domain concepts and constraints.

- Newtypes clarify intent (`UserId`, `PathBuf` wrappers, etc.).
- Enums model finite sets (states, variants, event kinds).
- Use `NonZeroUsize`, `NonEmpty` patterns, or validated constructors when useful.

### 3) Separate concerns
Keep code layered:
- **data access / I/O**: fetch, read, parse
- **core logic**: transform, validate, compute
- **presentation**: formatting, printing, UI

Avoid "god functions" that validate + query + parse + format + print.

### 4) Make behavior predictable
Prefer deterministic outputs and stable ordering.
If you truncate or cap output, do it consistently and document it.

---

## Error handling rules

### Don't swallow errors
Avoid converting errors into "missing" or defaults unless that is explicitly the behavior.

**Avoid**
- `.ok()` to erase errors (especially on I/O/DB/parse)
- `unwrap()` / `expect()` in non-test code
- `unwrap_or_default()` when it hides an invariant violation

**Prefer**
- `anyhow::Context` / `with_context` at boundaries
- explicit `match` or `map_err` when you truly want to downgrade an error
- small error enums (`thiserror`) for library-ish modules

### Add context at boundaries
When calling into something fallible, attach a message that explains what you were trying to do.

Example:
- `fs::read_to_string(path).with_context(|| format!("reading config at {path:?}"))?;`

---

## Modeling data and invariants

### Prefer typed parsing over ad-hoc probing
If a structure is known (JSON, CLI args, file formats), parse into a type.

**Prefer**
- `#[derive(Deserialize)] struct ...`
- `#[derive(clap::Parser)] struct ...`
- `FromStr` / `TryFrom` for validated conversions

**Avoid**
- indexing into untyped maps/JSON (string-key probing) when schema is known
- "parse a little, then keep it dynamic" unless it must be dynamic

### Use `Option<T>` intentionally
Use `Option<T>` when "missing" is a valid and meaningful state.
If "missing" is an error, model it as an error.

### Defaulting with `serde`
Use `#[serde(default)]` for fields that legitimately default:
- empty vectors/maps
- false booleans
- optional metadata

Do not default-away required fields silently.

---

## Control flow and indexing

### No unchecked indexing or arithmetic
Prefer safe indexing and checked arithmetic:
- `slice.get(i)` over `slice[i]` when input may be untrusted
- `checked_sub`, `checked_add` when overflow is plausible
- validate early and return descriptive errors

### Prefer early returns for validation
Validate inputs at the top of the function, return early with a clear error.

---

## Functions and modules

### Keep functions small and single-purpose
A good rule of thumb:
- one primary responsibility per function
- helpers for parsing/formatting/querying, not inline blobs

### Prefer returning values over printing
- "business logic" should return data (or a rendered string).
- the CLI/UI layer decides how to display it.

---

## Ownership, borrowing, and allocation

### Don't allocate by accident
- Prefer `&str` / `&Path` inputs where ownership is not required.
- Return `String` when you must own the data.
- Use `Cow<'a, str>` when you can avoid copies but still support ownership.

### Clone is not forbidden, but it should be justified
- cloning a small `Arc` is fine
- cloning a big `Vec` should usually be a design decision with a comment

---

## Concurrency and async

- Prefer sync by default.
- Introduce async only when it buys measurable I/O concurrency.
- Keep async boundaries narrow; do not "async-ify" pure logic.
- Avoid spawning tasks without a clear ownership/lifecycle plan.

---

## Formatting and style

- Use `rustfmt` defaults.
- Prefer clarity over cleverness.
- Prefer `match` when it improves readability over nested `if let`.
- Keep logs/messages consistent; include identifiers and counts in errors.

---

## "Avoid" list (common footguns)

- `.ok()` on fallible operations where the error matters
- `unwrap()` / `expect()` outside tests
- long chains of `map().and_then().unwrap_or()` that obscure control flow
- stringly typed flags/identifiers when enums/newtypes fit
- mixing I/O with logic and formatting in one function

---

## Checklist before finishing a change

- [ ] Errors are not swallowed; failures carry context
- [ ] Types encode invariants (newtypes/enums where helpful)
- [ ] Parsing is typed (serde/clap/FromStr) where schemas exist
- [ ] Indexing and arithmetic are bounds-safe
- [ ] Core logic is separated from I/O and presentation
- [ ] Output and ordering are deterministic
- [ ] No `unwrap()`/`expect()` in production paths
