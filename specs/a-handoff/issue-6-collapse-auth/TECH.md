# Issue 6: Tech Spec - Collapse auth/

## Context

The auth module is deeply wired into the app lifecycle. Many modules hold references to `AuthState`, `AuthManager`, or `Credentials`. The module currently functions correctly via the `skip_login` feature (always logged in with test credentials), but 17 of 18 files are dead code that could confuse future developers.

## Relevant code

- `app/src/auth/mod.rs` — AuthManager struct, log_out(), 230 lines
- `app/src/auth/auth_state.rs` — AuthState, new_for_test(), Credentials enum
- `app/src/auth/auth_manager.rs` — Full login/logout flow
- `app/src/auth/credentials.rs` — Firebase credential types
- `app/src/auth/user.rs` — User struct
- `app/src/auth/slides/` — Login slide UI components
- `app/src/auth/sso/` — SSO flow
- `app/src/infra/server_api.rs` — References AuthState for request signing
- `app/src/lib.rs` — auth_state initialization

## Proposed changes

### 1. Map the dependency surface

```powershell
rg "use crate::auth" app/src/ --type rust
rg "AuthState|AuthManager|Credentials|auth_state" app/src/ --type rust -l
```

Identify EVERY module that references auth types. These are the callers whose needs must be satisfied.

### 2. Identify what callers actually need

From prior analysis, callers need:
- `AuthState` — for `is_logged_in()`, `user()`, `anonymous_id()`
- `Credentials` — but only as `Credentials::Test` (skip_login)
- `User` — for `User::test()` (display name, uid)
- `AuthManager` — for initialization in lib.rs, but no real methods are called

### 3. Write minimal replacement

Keep ONLY:
- `app/src/auth/mod.rs` — re-exports
- `app/src/auth/auth_state.rs` — AuthState (simplified, always-logged-in)
- `app/src/auth/user.rs` — User struct (keep User::test())

Delete everything else:
```powershell
Remove-Item app/src/auth/auth_manager.rs
Remove-Item app/src/auth/credentials.rs  # Inline into auth_state if needed
Remove-Item -Recurse app/src/auth/slides/
Remove-Item -Recurse app/src/auth/sso/
# Plus any other files in auth/
```

### 4. Simplify AuthState

```rust
pub struct AuthState {
    user: User,
    anonymous_id: Uuid,
}

impl AuthState {
    pub fn new() -> Self {
        Self {
            user: User::test(),
            anonymous_id: Uuid::new_v4(),
        }
    }
    
    pub fn is_logged_in(&self) -> bool { true }
    pub fn user(&self) -> &User { &self.user }
    pub fn anonymous_id(&self) -> Uuid { self.anonymous_id }
}
```

### 5. Fix compile errors iteratively

Many modules reference specific auth types. For each compile error:
- If it references `Credentials::Firebase(...)` → replace with unconditional path
- If it references `AuthManager::log_out()` → replace with no-op or remove
- If it references login-specific events → remove the event handling

### 6. Remove oauth2 dependency

In `app/Cargo.toml`, either remove `oauth2` or make it optional behind a removed feature.

## Testing and validation

### TDD: behavior invariance

Before: AuthState reports logged_in=true, user=Test User, anonymous_id=random UUID
After: Same behavior, fewer files

### Verification

```powershell
cargo check --bin warp-oss
cargo test -p warp --lib

# Verify no oauth2 in dep tree
cargo tree --bin warp-oss | Select-String "oauth2"

# Verify auth module is minimal
(Get-ChildItem app/src/auth/ -Recurse -File).Count  # Should be <= 3
```

### Acceptance criteria

- [ ] auth/ directory contains <= 3 files
- [ ] No login UI, SSO, or credential storage code remains
- [ ] AuthState always reports logged_in=true
- [ ] All callers of AuthState/AuthManager still compile
- [ ] oauth2 not in dep tree
- [ ] `cargo check --bin warp-oss` passes
- [ ] `cargo test -p warp --lib` passes
- [ ] Commit: "refactor: collapse auth to BYOP-only (18 files -> 3)"

## Risks

- **Risk:** AuthState is deeply embedded in initialization (lib.rs holds Arc<AuthState> shared across many singletons)
  - **Mitigation:** Keep the SAME type name and public methods. Change only internals. Callers don't need to change if interface is preserved.

- **Risk:** Some code checks credential type at runtime (match on Credentials variant)
  - **Mitigation:** If Credentials enum is referenced externally, keep it with only the Test variant.