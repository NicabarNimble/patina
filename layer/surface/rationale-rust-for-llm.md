---
id: rationale-rust-for-llm
status: active
created: 2025-11-14
updated: 2025-12-09
oxidizer: nicabar
tags: [rationale, rust, llm-development, design-decisions, type-safety]
references: [dependable-rust]
---

# Why Rust for LLM-Assisted Development

**Core Insight**: In the LLM era, the strictest type system wins. The compiler isn't just a guardrail for humans - it's a teacher for AI.

---

## The Empirical Discovery

**Initial assumption**: "Rust is harder but safer, Go is easier but looser"

**Reality with LLMs**:
- **Rust + LLM = easier** (compiler catches and fixes mistakes)
- **Go + LLM = harder** (runtime surprises, guesswork)

This isn't theoretical - it's based on real experience building Patina with LLM assistance.

---

## The Feedback Loop

### Rust + LLM: Tight Feedback Loop

```
1. LLM generates code
2. Rust compiler: "Error: borrow of moved value at line 42"
3. LLM sees precise error with context
4. LLM fixes it correctly
5. Repeat until ✅ (usually 1-3 iterations)
```

**Example error:**
```rust
error[E0382]: borrow of moved value: `pattern`
  --> src/commands/extract.rs:23:5
   |
21 |     let saved = save_pattern(pattern);
   |                              ------- value moved here
23 |     println!("{}", pattern.name);
   |                    ^^^^^^^ value borrowed here after move
   |
help: consider cloning the value if the performance cost is acceptable
   |
21 |     let saved = save_pattern(pattern.clone());
   |                                     ++++++++
```

**LLM reads this and knows EXACTLY:**
- What went wrong (moved value)
- Where it happened (line 21)
- How to fix it (clone or borrow)

### Go + LLM: Loose Feedback Loop

```
1. LLM generates code
2. Go compiler: "✓ builds fine"
3. Runtime: *crashes* or *subtle bug*
4. LLM has to guess what's wrong
5. Trial and error (5-10+ iterations)
```

**Example error:**
```
panic: runtime error: invalid memory address or nil pointer dereference
[signal SIGSEGV: segmentation violation code=0x1 addr=0x0 pc=0x1234567]
```

**LLM has to guess:**
- Which pointer was nil?
- Where did it become nil?
- What's the root cause?

---

## What the Compiler Catches

### LLM Hallucinations That Rust Prevents

**1. Forgetting to handle errors:**
```rust
// LLM writes this
let pattern = extract_pattern(code);  // ❌ Won't compile

// Rust forces this
let pattern = extract_pattern(code)?;  // ✅ Explicit error handling
```

**2. Type confusion:**
```rust
// LLM writes this
fn process(name: String) { ... }
let borrowed = &some_string;
process(borrowed);  // ❌ Won't compile: expected String, found &String

// Compiler suggests fix
process(borrowed.to_owned());  // ✅
```

**3. Lifetime issues:**
```rust
// LLM writes this
fn get_name(p: &Pattern) -> &str {
    let name = p.name.clone();
    &name  // ❌ Won't compile: returns reference to local variable
}

// Compiler guides to fix
fn get_name(p: &Pattern) -> String {
    p.name.clone()  // ✅
}
```

**4. Concurrency bugs:**
```rust
// LLM writes this
let mut data = vec![1, 2, 3];
thread::spawn(|| {
    data.push(4);  // ❌ Won't compile: data races prevented
});
data.push(5);

// Compiler forces thread safety
```

**In Go:** ALL of these compile fine, fail at runtime.

---

## Compiler as LLM Teacher

### Example: LLM Tries to Add Async

**LLM generates:**
```rust
async fn extract_pattern(code: &str) -> Result<Pattern> {
    let response = reqwest::get("http://api").await?;
    // ...
}
```

**Compiler responds:**
```
error[E0277]: `impl Future<Output = Result<Pattern>>` cannot be sent between threads
note: required by a bound in `tokio::spawn`
help: consider using `#[tokio::main]` or adding Send bounds
```

**LLM realizes:** "This project uses `reqwest::blocking` and no tokio. Let me rewrite synchronously."

**Corrected code:**
```rust
fn extract_pattern(code: &str) -> Result<Pattern> {
    let response = reqwest::blocking::get("http://api")?;
    // ...
}
```

**The compiler TAUGHT the LLM** about project constraints!

### Example: LLM Violates Module Boundaries

**LLM tries:**
```rust
// In src/commands/extract.rs
use crate::indexer::internal::DatabaseConnection;  // ❌

fn do_something() {
    let conn = DatabaseConnection::new();
}
```

**Compiler:**
```
error[E0603]: module `internal` is private
  --> src/commands/extract.rs:3:24
   |
3  | use crate::indexer::internal::DatabaseConnection;
   |                     ^^^^^^^^ private module
```

**LLM learns:** "Use public API only."

**Corrected:**
```rust
use crate::indexer::add_pattern;  // ✅ Public API

fn do_something() {
    add_pattern(&pattern)?;
}
```

**The compiler ENFORCES** dependable-rust pattern boundaries!

---

## Type System Enables Fearless Refactoring

### Changing Function Signatures

**Change:**
```rust
// Old
pub fn save_pattern(pattern: Pattern) -> Result<()>

// New
pub fn save_pattern(pattern: &Pattern) -> Result<PathBuf>
```

**Rust compiler shows EVERY call site:**
```
error[E0308]: mismatched types
  --> src/commands/extract.rs:42:5
   |
42 |     save_pattern(pattern)?;
   |     ^^^^^^^^^^^^^^^^^^^^ expected `()`, found `PathBuf`

error[E0308]: mismatched types
  --> src/commands/init.rs:78:5
   |
78 |     save_pattern(pattern)?;
   |     ^^^^^^^^^^^^^^^^^^^^ expected `()`, found `PathBuf`
```

**LLM can:**
1. See all places that need updating
2. Fix them systematically
3. Know when it's done (compiles = complete)

**In Go:** Silently compiles, runtime surprises later.

---

## Why Not Go?

### Go's Design Choices Hurt LLM Development

**1. Nil pointers everywhere:**
```go
var pattern *Pattern  // nil by default
pattern.Name          // Runtime panic
```

LLM has to guess which pointers might be nil.

**2. Error handling is optional:**
```go
result, err := doSomething()
// LLM forgets to check err
doSomethingElse(result)  // Might crash
```

**3. Implicit interfaces:**
```go
// LLM implements this
func (p *Pattern) Save() error { ... }

// But forgets this method from the interface
func (p *Pattern) Load() error { ... }

// Compiles fine! Runtime failure when Load is called
```

**4. No sum types:**
```go
// Can't express "Result is either Ok OR Err"
// LLM might check wrong field
```

### What Go Gets Right

**Fair assessment:**
- ✅ Fast compilation (great for iteration)
- ✅ Simple concurrency model (goroutines)
- ✅ Good standard library
- ✅ Easy cross-compilation

**But for LLM-assisted development:**
- ❌ Loose type system = more bugs slip through
- ❌ Runtime failures = harder to debug
- ❌ Less guidance from compiler

---

## Historical Context

### Why Successful Tools Use Go

| Tool | Year | Rust 1.0 | LLM Era | Why Go? |
|------|------|----------|---------|---------|
| Docker | 2013 | 2015 | ❌ | Rust didn't exist |
| Kubernetes | 2014 | 2015 | ❌ | Rust too immature |
| Terraform | 2014 | 2015 | ❌ | Rust too immature |

**They chose Go because:**
1. Rust wasn't stable yet
2. No LLM assistance existed
3. Human developers preferred simpler language

**If built today (2024+) with LLMs:**
- Probably choose Rust (compiler catches LLM mistakes)
- Type safety matters more than simplicity
- Compilation speed less important with AI assistance

---

## Rust + LLM Synergies

### 1. Compiler Errors Are Precise Prompts

**For humans:** "Read error, understand context, fix"
**For LLMs:** Same process, but automated!

LLMs are actually BETTER at parsing compiler errors than humans:
- No fatigue from reading long messages
- Pattern match across thousands of similar errors
- Apply fixes consistently

### 2. Type System Guides Exploration

**LLM exploring a new codebase:**
```rust
// LLM types: pattern.
// IDE/rust-analyzer shows: name, id, tags, content, created_at
// LLM learns structure from types alone
```

**In Go:**
```go
// LLM has to read docs or guess
```

### 3. Refactoring Without Fear

**Prompt:** "Change Pattern to use Uuid instead of String for id"

**Rust:** Compiler shows every place that breaks, LLM fixes systematically.
**Go:** LLM has to search manually, might miss places.

### 4. The Compiler Remembers Constraints

**From `PROJECT_DESIGN.toml`:**
```toml
"No tokio/async - use rayon for parallelism"
```

**In Rust:** LLM tries to use async → compiler rejects → LLM learns
**In Go:** LLM uses goroutines everywhere, no constraint enforcement

---

## Design Philosophy Alignment

### From `CLAUDE.md`:
```md
## Development Guidelines
- Rust for CLI and core logic - let the compiler be your guard rail
```

**This isn't philosophical - it's EMPIRICAL:**
- The compiler is a guard rail **for the LLM**, not just humans
- Type safety **enables** LLM-assisted development
- Strict compiler **accelerates** development (catches mistakes early)

### From `dependable-rust.md`:
```md
Keep your public interface small and stable.
Hide implementation details in internal.rs.
Not a line count rule - a design principle.
```

**Why this works with LLMs:**
- LLMs respect boundaries (compiler enforces them)
- Clear interfaces = clear prompts
- Module system guides decomposition

---

## The Optimal LLM Stack

**Every layer has:**
- ✅ Strong types
- ✅ Explicit error handling
- ✅ Precise error messages
- ✅ Compiler/runtime that catches mistakes

```
┌────────────────────────────────┐
│  Rust (strict types)           │
│  - Borrow checker              │
│  - Lifetime tracking           │
│  - Exhaustive pattern matching │
└───────────┬────────────────────┘
            │ Blocking HTTP
            ▼
┌────────────────────────────────┐
│  Effect/TS (typed errors)      │
│  - Effect<A, E, R>             │
│  - Compile-time error tracking │
│  - Type-safe composition       │
└───────────┬────────────────────┘
            │ Queries
            ▼
┌────────────────────────────────┐
│  Scryer Prolog (unification)   │
│  - Logic errors at query time  │
│  - Precise failure messages    │
│  - Declarative constraints     │
└────────────────────────────────┘
```

**Pattern:** Each layer catches different classes of bugs, guides LLM to correct solutions.

---

## Common Objections

### "But Rust compile times are slow!"

**Pre-LLM:** Major issue (waiting for compiler)
**With LLM:** Less important (LLM works while compiling)

Plus:
- LLM makes fewer mistakes → fewer recompiles
- Incremental compilation is fast
- `cargo check` is faster than full build

### "But Go is simpler!"

**For humans writing code:** Maybe
**For LLMs writing code:** No - more runtime bugs

Simplicity that allows bugs isn't actually simple.

### "But everyone uses Go for CLI tools!"

**They did in 2014 - before:**
- Rust 1.0 (2015)
- Stable Rust ecosystem (2018+)
- LLM-assisted development (2023+)

The landscape has changed.

---

## Real-World Evidence

### Patina Development Experience

**Observed patterns:**
1. **Fewer iterations** - Rust catches issues immediately
2. **Better refactoring** - Change signature, compiler shows all impacts
3. **Clearer prompts** - Type errors are precise instructions
4. **Less debugging** - Bugs caught at compile time
5. **LLM learns faster** - Compiler teaches project patterns

**Quote from developer:**
> "LLM + Go honestly sucks. I find LLM + Rust much better experience. Type safety is important to me. Compile-time bug catching is chef's kiss vs finding shit later. Plus the compiler errors help LLM write better code and loop until working."

---

## Conclusion

**Pre-LLM era thinking:**
- Rust is harder but safer
- Go is easier but looser
- Choose based on team skill level

**LLM era reality:**
- **Rust is EASIER with LLMs** (compiler catches and fixes)
- **Go is HARDER with LLMs** (runtime surprises)
- Choose based on what catches errors fastest

**For Patina:**
- Rust for CLI and core logic ✅
- Effect/TS for async orchestration ✅
- Scryer Prolog for logical reasoning ✅

Every layer chosen for **type safety + precise errors = better LLM experience**.

The compiler isn't a burden - it's a co-pilot for your AI co-pilot.

---

## Key Principles

1. **Compiler errors are prompts** - More precise = better LLM fixes
2. **Type system guides exploration** - LLM learns structure from types
3. **Refactoring is fearless** - Compiler shows all impacts
4. **Constraints are enforced** - Not documented, CHECKED
5. **Bugs caught early** - Compile time >> runtime

**Remember:** In the LLM era, the strictest type system wins.

---

## References
- [Dependable Rust Pattern](../core/dependable-rust.md)
- [Unix Philosophy](../core/unix-philosophy.md)
- [Adapter Pattern](../core/adapter-pattern.md)
- [TypeScript Effect Integration](./ts-effect-patina-exploration.md)
- Rust Book: Error Handling - https://doc.rust-lang.org/book/ch09-00-error-handling.html

---

*Last Updated: 2025-11-14*
*Status: Active*
*Based on: Empirical experience building Patina with LLM assistance*
