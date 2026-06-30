# Issue 7: Tech Spec - Prune FeatureFlag Enum

## Context

This is a large refactoring pass that MUST happen after Issues 4, 5, and 6 have deleted the code that references cloud-related flags. Otherwise deleting enum variants will cause compile errors at the deleted usage sites.

## Relevant code

- `crates/warp_features/src/lib.rs` — FeatureFlag enum definition (326 variants, ~1000 lines)
- `app/src/features.rs` — enabled_features() function (~500 lines of cfg-gated inserts)
- Every file that calls `FeatureFlag::X.is_enabled()` — scattered across app/src/

## Proposed changes

### 1. Identify dead variants

```powershell
# For each FeatureFlag variant, check if it's referenced anywhere:
# Get all variant names from the enum, then search for each
rg "FeatureFlag::" app/src/ crates/ --type rust | 
    ForEach-Object { ($_ -split "FeatureFlag::")[1] -split "[^A-Za-z0-9_]" | Select-Object -First 1 } |
    Sort-Object -Unique
```

Compare against the full enum definition. Variants with 0 references outside the enum definition itself are DEAD.

### 2. Delete dead variants from enum

In `crates/warp_features/src/lib.rs`, remove the dead variant lines. Run `cargo check` — the compiler will tell you if any were still referenced.

### 3. Clean up enabled_features()

In `app/src/features.rs`, remove all `#[cfg(feature = "xxx")]` blocks that reference deleted/removed features. The function should shrink from ~500 lines to ~50.

### 4. Hardcode "always-on" flags

For variants that are ALWAYS enabled in the BYOP build and have NO reason to be disabled:

Strategy per variant:
1. Search: `rg "FeatureFlag::AgentMode.is_enabled()" app/src/`
2. For each call site, replace the if-check with unconditional execution
3. Once all call sites are removed, delete the variant from the enum

Example transformation:
```rust
// Before:
if FeatureFlag::AgentMode.is_enabled() {
    show_agent_panel();
}

// After:
show_agent_panel();
```

WARNING: Only hardcode flags where you are CERTAIN the behavior should always be on. For flags where there might be a settings toggle (even if not currently exposed), KEEP the variant.

### 5. Resize FLAG_STATES array

The `FLAG_STATES` array in `warp_features/src/lib.rs` is sized by the enum variant count. After pruning variants, this automatically adjusts (it uses `FeatureFlag::COUNT` or similar). Verify the array still works.

### 6. Update USER_PREFERENCE_MAP

If `USER_PREFERENCE_MAP` is indexed by enum discriminant, ensure the remaining variants have correct indices after pruning.

## Testing and validation

### Iterative approach (TDD for deletion)

For each batch of variants to hardcode:
1. RED: Remove the if-check → code now always executes → existing tests should still pass
2. GREEN: Remove the variant from enum → compile check passes
3. Repeat

### Verification

```powershell
# After all pruning:
cargo check --bin warp-oss
cargo test -p warp --lib
cargo test -p warp_features

# Verify enum size
rg "enum FeatureFlag" -A 500 crates/warp_features/src/lib.rs | 
    Select-String "^    [A-Z]" | Measure-Object  # Should be <= 60
```

### Acceptance criteria

- [ ] FeatureFlag enum has <= 60 variants
- [ ] enabled_features() function is < 100 lines
- [ ] No dead variants (every remaining variant has at least 1 runtime reference)
- [ ] "Always-on" features execute unconditionally (no if-check)
- [ ] User-toggleable features still respect settings
- [ ] `cargo check --bin warp-oss` passes
- [ ] `cargo test -p warp --lib` passes
- [ ] `cargo test -p warp_features` passes
- [ ] Multiple commits: one per batch of ~10-20 hardcoded flags

## Risks

- **Risk:** Removing a variant that IS referenced somewhere unexpected
  - **Mitigation:** Compiler catches this immediately. Re-add the variant if needed.

- **Risk:** Hardcoding a flag that users can toggle in settings
  - **Mitigation:** Before hardcoding, search for `set_user_preference` references to that flag. If found, KEEP it.

- **Risk:** FLAG_STATES array index corruption
  - **Mitigation:** The array should be derived from enum size. After pruning, verify with a test that `FeatureFlag::X.is_enabled()` reads the correct bool.