# Autocomplete Rearchitecture: Tree-Sitter-Based Approach

## Overview

This document proposes a comprehensive rearchitecture of jiq's autocomplete system, replacing character-by-character lexical scanning with AST-based parsing using tree-sitter-jq. This will dramatically reduce complexity while improving semantic understanding of jq queries.

## Current System: Problems and Complexity

### Architecture Overview

The current autocomplete implementation spans **2,470 lines** across 7 modules:

- `context.rs` (389 lines) - Character-by-character context detection
- `insertion.rs` (621 lines) - 13 different insertion formulas
- `result_analyzer.rs` (609 lines) - JSON field extraction from results
- `jq_functions.rs` (473 lines) - Function metadata
- `autocomplete_state.rs` (203 lines) - State management
- `autocomplete_render.rs` (175 lines) - UI rendering

### Core Problems

#### 1. Three-Level Context Analysis

The system performs a complex 3-stage analysis to determine what to suggest:

```
SuggestionContext (Function vs Field)
    ↓
CharType (13 variants: PipeOperator, CloseBracket, Dot, etc.)
    ↓
needs_leading_dot (boolean decision)
```

Each level adds complexity and edge cases.

#### 2. Thirteen Insertion Formulas

The `insertion.rs` module contains **13 different formulas** for inserting suggestions, one for each `CharType`:

- `NoOp`: `base + middle + ("." if needed) + suggestion`
- `CloseBracket`: `base + middle + "." + suggestion`
- `PipeOperator`: `base + middle.trim() + " " + suggestion`
- `Semicolon`, `Comma`, `Colon`: Similar variations
- `OpenParen`, `OpenBracket`, `OpenBrace`: Direct insertion
- `QuestionMark`, `Dot`, `CloseParen`, `CloseBrace`: More variations

Each formula handles spacing, dot placement, and "middle query" preservation differently.

#### 3. Complex "Middle Query" Extraction

Lines 294-350 of `insertion.rs` extract the "middle query" - everything between the last successful query and the current cursor position:

```rust
// Query: ".services | if has(...) then .ca"
// Base: ".services"
// Middle: " | if has(...) then "  // Preserves complex expressions!
```

This logic is intricate and fragile, requiring special handling for nested arrays, root replacement, and whitespace.

#### 4. Special Case Accumulation

The codebase shows evidence of technical debt through special cases:

- **Root replacement** (lines 138-146): Prevent `"." + ".services"` from becoming `"..services"`
- **Nested array detection** (lines 108-134): Move `[]` from middle to base, strip from suggestion
- **Whitespace awareness** (lines 43-48): Detect whitespace before dots (e.g., `then .field`)

#### 5. Limited Semantic Understanding

The lexical approach cannot distinguish between semantically different contexts:

- `if .x then .y` (conditional) vs `.x | .y` (pipe) - both see "then" as just an identifier
- `map(.x)` (function argument) vs `(.x)` (grouping) - both see opening paren
- `.[]? | .field` (optional) vs `.[] | .field` (required) - doesn't understand `?` operator

### Evolution History

Git history reveals the system's growing complexity:

1. **Original**: Static JSON analysis (json_analyzer.rs, ~1400 lines)
2. **Nov 27 refactor (#20)**: Replaced with result-based analysis (deleted ~1500 lines, fixed fundamental bugs)
3. **Nov 28 enhancement**: Added smart parenthesis insertion (added 13 formula variations)
4. **Current state**: 320+ tests suggesting many edge cases discovered and patched

## Proposed Solution: Tree-Sitter-Based AST Parsing

### Key Benefits

1. **Reduced Complexity**: 2,470 lines → ~1,800 lines (-27%)
2. **Semantic Understanding**: Knows conditionals, variables, try-catch, recursive descent
3. **Fewer Formulas**: 13 insertion formulas → 4 insertion strategies (-69%)
4. **Simplified Context**: 3-level analysis → 1 unified AST context (-67%)
5. **Better Maintainability**: Fewer special cases, clearer architecture
6. **Future-Proof**: Enables LSP features, type checking, refactorings

### New Architecture

```
Keystroke → Debouncer (50ms) → Execute jq → Parse JSON → AST Parse → Context → Suggestion
                                      ↓                      ↓
                                Result Caching          Tree-sitter (cached)
```

**Module Structure:**

```
src/autocomplete/
├── parser.rs                   # NEW: Tree-sitter integration & caching
├── context_ast.rs              # NEW: AST-based context detection
├── insertion_ast.rs            # NEW: Simplified insertion (4 strategies)
├── result_analyzer.rs          # KEEP: Result-based field suggestions
├── jq_functions.rs             # KEEP: Function metadata
├── autocomplete_state.rs       # KEEP: UI state
└── autocomplete_render.rs      # KEEP: Rendering
```

## Technical Design

### 1. Parser Module (`parser.rs`)

**Purpose**: Manage tree-sitter parser, caching, and AST traversal utilities

**Core Structure:**

```rust
pub struct JqParser {
    parser: Parser,
    cache: ParserCache,  // LRU cache, 100 entries, ~20-50KB
}

impl JqParser {
    pub fn new() -> Result<Self, ParseError>;
    pub fn parse(&mut self, query: &str) -> Result<&Tree, ParseError>;
    pub fn node_at_cursor(tree: &Tree, cursor_pos: usize) -> Option<Node>;
    pub fn parent_chain(node: Node) -> Vec<NodeInfo>;
    pub fn find_base_expression(tree: &Tree, cursor_pos: usize) -> Option<Node>;
}
```

**Key Features:**

- **LRU Caching**: HashMap with O(1) lookup, 70-80% expected hit rate
- **Fast Parsing**: <2ms for typical queries, <1ms on cache hit
- **ERROR Node Handling**: Incomplete queries (like `.ser`) produce ERROR nodes that trigger autocomplete
- **AST Traversal**: Utilities for finding cursor position, parent chains, base expressions

### 2. Context AST Module (`context_ast.rs`)

**Purpose**: Replace lexical scanning with semantic AST analysis

**Core Structures:**

```rust
pub struct ASTContext {
    pub suggestion_mode: SuggestionMode,
    pub insertion_strategy: InsertionStrategy,
    pub base_node: Option<NodeInfo>,
    pub parent_chain: Vec<NodeInfo>,
}

pub enum SuggestionMode {
    FieldAccess,     // Show JSON field names
    FunctionCall,    // Show jq functions/keywords
    Mixed,           // Show both (after pipe, at start)
    None,            // No suggestions (inside strings)
}

pub enum InsertionStrategy {
    Direct,              // "field" - continuing path
    WithDot,             // ".field" - after pipe/operator
    ArrayChain,          // "[].field" - after array index
    FunctionWithParen,   // "map(" - function needs args
}
```

**Context Detection Examples:**

| Query | Cursor | Node Kind | Parent Chain | Mode | Strategy |
|-------|--------|-----------|--------------|------|----------|
| `.services\|` | 11 | ERROR | [ERROR, pipe, root] | Mixed | WithDot |
| `.services[].` | 12 | ERROR | [ERROR, index, field] | FieldAccess | Direct |
| `map(.` | 5 | ERROR | [ERROR, function_call] | FieldAccess | Direct |
| `if .x then .` | 12 | ERROR | [ERROR, if_expression] | Mixed | Direct |

**How It Works:**

1. **Parse query** to get AST
2. **Find node at cursor** using byte position
3. **Get parent chain** to understand context
4. **Determine suggestion mode** based on node types:
   - `field_access`, `identity` → FieldAccess
   - `pipe`, `pipe_expression` → Mixed
   - `function_call` → Check if in name or argument
   - `if_expression`, `try_expression` → Mixed
   - `string`, `string_content` → None
5. **Determine insertion strategy** based on previous character and parents:
   - After `.` → Direct (dot already there)
   - After `|`, `;`, `,` → WithDot
   - After `[]` → ArrayChain
   - In function call → FunctionWithParen

### 3. Insertion AST Module (`insertion_ast.rs`)

**Purpose**: Simplify insertion to 4 strategies instead of 13 formulas

**Core Logic:**

```rust
pub fn insert_suggestion_ast(
    textarea: &mut TextArea<'_>,
    query_state: &mut QueryState,
    parser: &mut JqParser,
    suggestion: &Suggestion,
    context: &ASTContext,
);

fn format_suggestion_text(
    suggestion: &Suggestion,
    strategy: &InsertionStrategy,
) -> String;

fn calculate_insertion_point(
    query: &str,
    cursor_pos: usize,
    context: &ASTContext,
) -> (usize, usize);
```

**Mapping 13 Formulas to 4 Strategies:**

| Old CharType | Old Formula Complexity | New Strategy |
|--------------|----------------------|--------------|
| NoOp | Conditional dot logic | Direct |
| Dot | Direct append | Direct |
| CloseBracket | Add dot | WithDot |
| PipeOperator | Trim + space + add | WithDot |
| Semicolon, Comma, Colon | Similar to pipe | WithDot |
| OpenBracket | Special array handling | ArrayChain |
| OpenParen (in function) | Function context | FunctionWithParen |
| QuestionMark | Add dot | WithDot |
| CloseParen, CloseBrace | Add dot | WithDot |
| OpenBrace | Direct | Direct |

**Key Simplifications:**

- **No "Middle Query" Extraction**: AST already knows structure, just replace partial at cursor
- **No Special Cases**: AST context handles root replacement, nested arrays naturally
- **Consistent Logic**: Each strategy has clear, consistent behavior

### 4. Advanced Feature Support

#### Conditionals

```jq
if .count > 10 then .high_value else .low_value end
```

**AST Understanding:**
- Recognizes `if_expression` node type
- Knows which clause cursor is in (condition/then/else)
- Suggests fields/functions appropriately in each context

**Current System**: Treats "then" and "else" as regular identifiers, gets confused

#### Variables

```jq
.items[] as $item | $item.name
```

**AST Understanding:**
- Recognizes `variable_binding` (definition: `as $item`)
- Recognizes `variable_reference` (usage: `$item`)
- Can track variable scope (future enhancement)

**Current System**: No variable support

#### Try-Catch

```jq
try .field.nested catch "default"
```

**AST Understanding:**
- Recognizes `try_expression` node
- Suggests fields after "try"
- Understands "catch" block (string context)

**Current System**: Limited support

#### Recursive Descent

```jq
.. | select(.type == "user")
```

**AST Understanding:**
- Recognizes `recursive_descent` operator (`..`)
- Understands it produces multiple values
- Suggests fields after pipe

**Current System**: Basic support through lexical patterns

## Migration Strategy

### Phase 1: Infrastructure (1-2 days)

**Goals:**
- Add tree-sitter dependencies
- Implement parser module
- Add to App struct

**Tasks:**
- [ ] Add to `Cargo.toml`: `tree-sitter = "0.20"`, `tree-sitter-jq = "0.1"`
- [ ] Create `src/autocomplete/parser.rs`
- [ ] Implement `JqParser` struct with caching
- [ ] Add `parser: JqParser` field to `App` struct in `app_state.rs`
- [ ] Initialize in `App::new()`
- [ ] Write 20+ unit tests (parsing, caching, node lookup)

**Success Criteria:**
- Parser can parse all valid jq queries
- Cache hit rate >70% in tests
- Parse time <2ms for typical queries

### Phase 2: Context Detection (2-3 days)

**Goals:**
- Implement AST-based context detection
- Add feature flag for safe rollout
- Preserve existing behavior

**Tasks:**
- [ ] Create `src/autocomplete/context_ast.rs`
- [ ] Implement `ASTContext`, `SuggestionMode`, `InsertionStrategy`
- [ ] Implement `analyze_context_ast()` function
- [ ] Add feature flag `ast-autocomplete` to `Cargo.toml`
- [ ] Add feature-gated entry point in `mod.rs`
- [ ] Run all existing tests with old path (should pass)
- [ ] Write 30+ new tests for AST context detection
- [ ] Test advanced features (conditionals, variables)

**Success Criteria:**
- All existing tests pass with legacy path
- New tests cover all AST node types
- Context detection matches or improves on legacy

### Phase 3: Insertion Logic (2-3 days)

**Goals:**
- Implement simplified insertion
- Wire up AST path
- Verify no regressions

**Tasks:**
- [ ] Create `src/autocomplete/insertion_ast.rs`
- [ ] Implement `insert_suggestion_ast()` and helpers
- [ ] Move `context.rs` and `insertion.rs` to `legacy/` folder
- [ ] Update feature-gated entry in `mod.rs`
- [ ] Run full test suite with `--features ast-autocomplete`
- [ ] Fix any failing tests
- [ ] Write 25+ tests for insertion strategies

**Success Criteria:**
- All tests pass with AST flag enabled
- Insertion behavior matches or improves on legacy
- No performance regressions

### Phase 4: Integration (1-2 days)

**Goals:**
- Integrate into App lifecycle
- Performance validation
- End-to-end testing

**Tasks:**
- [ ] Update `src/autocomplete/mod.rs` entry points
- [ ] Update `src/app/app_state.rs::update_autocomplete()`
- [ ] Pass parser to autocomplete functions
- [ ] Run performance benchmarks
- [ ] Integration testing with real JSON datasets
- [ ] Test debouncer still works (50ms)
- [ ] Test with large queries (>100 chars)

**Success Criteria:**
- Total autocomplete time <3ms
- Debouncer works correctly
- No UI lag or glitches
- Works with all test datasets

### Phase 5: Cleanup (1 day)

**Goals:**
- Remove legacy code
- Update documentation
- Final validation

**Tasks:**
- [ ] Remove feature flags (make AST default)
- [ ] Delete `legacy/` folder
- [ ] Update `development/subsystems/AUTOCOMPLETE.md`
- [ ] Update `development/ARCHITECTURE.md`
- [ ] Add parser section to architecture docs
- [ ] Run final test suite
- [ ] Manual testing of all features
- [ ] Performance benchmarks

**Success Criteria:**
- Clean codebase with no legacy code
- All documentation updated
- All tests passing
- Performance within targets

### Feature Flag Pattern

Safe migration using Cargo features:

```rust
// In Cargo.toml
[features]
default = []
ast-autocomplete = ["tree-sitter", "tree-sitter-jq"]

// In src/autocomplete/mod.rs
#[cfg(feature = "ast-autocomplete")]
pub use context_ast::analyze_context_ast;
#[cfg(not(feature = "ast-autocomplete"))]
pub use context::analyze_context;
```

This allows:
- Testing both paths in parallel
- Easy revert if issues found
- Gradual rollout to users
- A/B testing if desired

## Performance Analysis

### Parse Caching Strategy

**Design:**
- LRU cache with 100 entries
- HashMap for O(1) lookup
- Stores both tree and source string

**Performance:**
- **Hit Rate**: 70-80% expected during typing
- **Memory**: ~20-50KB total (negligible vs JSON input)
- **Lookup Time**: <0.1ms on cache hit
- **Parse Time**: <2ms on cache miss

### Timing Comparison

**Current System:**
```
Keystroke → 50ms debounce → 5-20ms jq exec → 1-5ms JSON parse → 0.5ms lexical scan
Total autocomplete: 0.5ms
```

**New System:**
```
Keystroke → 50ms debounce → 5-20ms jq exec → 1-5ms JSON parse → 2ms AST parse + 0.5ms analysis
Total autocomplete: 2.5ms
```

**Impact:** +2ms added latency (still well within 50ms debounce window, imperceptible to users)

### Benchmark Targets

| Operation | Target | Typical |
|-----------|--------|---------|
| Simple query parse | <1ms | 0.5ms |
| Complex query parse | <3ms | 1.5ms |
| Cache hit lookup | <0.1ms | 0.05ms |
| Total autocomplete | <3ms | 2.5ms |
| Memory overhead | <100KB | ~50KB |
| Cache hit rate | >70% | ~75% |

### Memory Considerations

**New Overhead:**
- Parser instance: ~1KB
- Parse tree (per query): 100-500 bytes
- Cache (100 entries): 20-50KB total
- **Total: <100KB**

**Context:**
- jiq typically loads JSON files (KB to MB)
- Parser overhead is <1% of typical usage
- Acceptable tradeoff for improved functionality

## Testing Strategy

### Unit Tests

**Parser Module (20+ tests):**
- Parse valid queries
- Handle invalid/incomplete queries
- Node lookup at cursor position
- Parent chain traversal
- Base expression finding
- Cache hit/miss behavior
- ERROR node detection

**Context AST Module (30+ tests):**
- Each SuggestionMode scenario
- Each InsertionStrategy scenario
- All jq node types (field_access, pipe, function_call, etc.)
- Advanced features (if/then/else, try/catch, variables)
- Edge cases (empty query, cursor at start/end)

**Insertion AST Module (25+ tests):**
- Each InsertionStrategy formatting
- Partial text extraction
- Insertion point calculation
- Cursor positioning after insert

### Integration Tests

**End-to-End Flow:**
- Keystroke → parse → suggest → insert
- Real-world queries from git history
- Complex nested expressions
- All TUI interaction paths

**Edge Cases:**
- Empty queries
- Very long queries (>200 chars)
- Malformed queries
- Rapid typing (stress test debouncer)

### Regression Tests

**Critical Requirement**: All existing tests must pass

- Run existing test suite with legacy path (baseline)
- Run existing test suite with AST path (should match)
- No behavior changes for end users
- All special cases still handled correctly

### Performance Tests

**Benchmarks:**
```rust
#[bench]
fn bench_parse_simple_query(b: &mut Bencher) {
    let mut parser = JqParser::new().unwrap();
    b.iter(|| parser.parse(".services.name"));
}

#[bench]
fn bench_parse_complex_query(b: &mut Bencher) {
    let mut parser = JqParser::new().unwrap();
    let query = ".services[] | select(.active == true) | {name, cpu}";
    b.iter(|| parser.parse(query));
}

#[bench]
fn bench_context_detection(b: &mut Bencher) {
    let mut parser = JqParser::new().unwrap();
    b.iter(|| analyze_context_ast(&mut parser, ".services | .", 13));
}
```

**Targets:**
- Simple parse: <1ms
- Complex parse: <3ms
- Context detection: <3ms total
- Memory: <100KB overhead

## Risk Analysis

### Risk 1: tree-sitter-jq Grammar Incompleteness

**Likelihood**: Medium
**Impact**: High

**Description**: tree-sitter-jq may not cover all jq syntax or have bugs

**Mitigation:**
- Test with comprehensive jq queries early
- Graceful degradation on parse errors
- Keep legacy code available during migration

**Fallback**: Feature flag allows easy revert to legacy system

### Risk 2: Performance Regression

**Likelihood**: Low
**Impact**: Medium

**Description**: AST parsing could be slower than lexical scanning

**Mitigation:**
- Aggressive caching (LRU, 100 entries)
- Early benchmarking in Phase 1
- Optimize hot paths if needed

**Fallback**: Add incremental parsing if needed, or revert to legacy for slow cases

### Risk 3: Breaking Existing Functionality

**Likelihood**: Medium
**Impact**: High

**Description**: AST approach might change suggestion behavior in unexpected ways

**Mitigation:**
- Feature flag for parallel testing
- Preserve all existing tests
- Extensive integration testing
- Gradual rollout

**Fallback**: Feature flag allows instant revert, no user impact

### Risk 4: Complex Edge Cases

**Likelihood**: Medium
**Impact**: Low

**Description**: AST might not handle some malformed queries well

**Mitigation:**
- ERROR nodes are designed for incomplete queries
- Graceful degradation on parse failures
- Extensive edge case testing

**Fallback**: Fall back to lexical approach for error-heavy queries

## Success Metrics

### Quantitative

| Metric | Before | After | Target |
|--------|--------|-------|--------|
| Total lines | 2,470 | TBD | <1,900 |
| Insertion formulas | 13 | 4 | 4 |
| Context analysis levels | 3 | 1 | 1 |
| Special cases | 3 | 0 | 0 |
| Autocomplete latency | 0.5ms | TBD | <3ms |
| Test coverage | ~85% | TBD | >85% |
| Binary size increase | 0 | TBD | <1MB |

### Qualitative

- [ ] Code is easier to understand and maintain
- [ ] New features are easier to add
- [ ] Fewer bugs related to edge cases
- [ ] Better support for advanced jq features
- [ ] No user-visible regressions
- [ ] Documentation is clear and complete

## Dependencies

### New Crates

```toml
[dependencies]
tree-sitter = "0.20"          # Core parser (MIT)
tree-sitter-jq = "0.1"        # jq grammar (MIT)
```

**Size Impact:**
- tree-sitter: ~400KB
- tree-sitter-jq: ~100KB
- **Total: ~500KB binary size increase**

**License**: Both MIT licensed, compatible with jiq's license

**Maintenance**: tree-sitter is actively maintained by GitHub, tree-sitter-jq is community-maintained

## Future Enhancements

Once AST foundation is in place, these become possible:

### 1. Type Propagation Through Pipes

Track types through pipe chains:

```jq
.users[]  →  Object
| .id     →  Number
| tostring →  String
```

Enables better suggestions based on expected types.

### 2. Variable Tracking

Remember variable definitions and suggest them:

```jq
.items[] as $item | /* suggest $item here */
```

### 3. Function Signature Validation

Check argument types match function expectations:

```jq
select(.active)  # ✓ boolean expression
select(.name)    # ⚠ might not be boolean
```

### 4. LSP Server

AST enables Language Server Protocol features:
- Go to definition
- Find references
- Rename refactoring
- Diagnostics

### 5. Query Formatter

Pretty-print jq queries with consistent style:

```jq
.a|.b|.c  →  .a | .b | .c
```

### 6. Error Recovery

Better error messages using AST:

```jq
.services[.name  # Missing closing bracket at position 14
```

## Conclusion

The tree-sitter-based rearchitecture addresses fundamental limitations of the lexical approach while significantly reducing complexity. The migration path is low-risk with feature flags enabling safe rollout and easy reversion if needed.

**Key Benefits:**
- **27% less code** with clearer architecture
- **69% fewer insertion formulas** (13 → 4)
- **Better semantic understanding** of jq syntax
- **Future-proof** for advanced features

**Cost:**
- +2ms latency (imperceptible)
- +500KB binary size
- 1-2 weeks implementation time

**Recommendation**: Proceed with implementation using the phased migration strategy outlined above.
