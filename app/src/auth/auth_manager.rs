#![cfg_attr(feature = "local-only", allow(dead_code, unused_imports, unused_variables))]
use std::sync::Arc;

use uuid::Uuid;
use warp_core::channel::ChannelState;
use warp_server_auth::user::persistence::PersistedUser;
use warpui::{Entity, ModelContext, SingletonEntity};

use super::auth_state::{AuthState, PersistAction};
use super::auth_view_modal::{AuthRedirectPayload, AuthViewVariant};
use super::credentials::Credentials;
use super::user::User;
use super::{AuthStateProvider, UserUid};
use crate::server::server_api::auth::UserAuthenticationError;
use crate::server::server_api::{ServerApi, ServerApiProvider};
use crate::server::telemetry::AnonymousUserSignupEntrypoint;
use crate::{send_telemetry_from_ctx, TelemetryEvent};

#[derive(Debug)]
pub enum AuthManagerEvent {
    /// Successfully authenticated a user with no errors.
    AuthComplete,
    /// Failed to authenticate a user, due to a particular `UserAuthenticationError`.
    AuthFailed(UserAuthenticationError),
    /// Failed to create an anonymous user.
    CreateAnonymousUserFailed,
    /// The user chose to skip login entirely (no Firebase user created).
    SkippedLogin,
    /// The user now needs to reauthenticate. If the user needs to reauth, an `AuthFailed`
    /// event might be triggered instead, but there are some code paths where we don't
    /// refresh the entire user, only their token, which is when this event might be emitted.
    NeedsReauth,
    /// The user is anonymous and has attempted to access a login-gated feature or link.
    AttemptedLoginGatedFeature {
        auth_view_variant: AuthViewVariant,
    },
    // The current user is anonymous and the client has received a browser intent to sign in with a different Warp account.
    // Holds an auth payload from the received browser intent.
    LoginOverrideDetected(AuthRedirectPayload),
    /// Received a device authorization code as part of the device auth flow.
    ReceivedDeviceAuthorizationCode {
        #[cfg_attr(target_family = "wasm", allow(unused))]
        verification_url: String,
        #[cfg_attr(target_family = "wasm", allow(unused))]
        verification_url_complete: Option<String>,
        #[cfg_attr(target_family = "wasm", allow(unused))]
        user_code: String,
    },
}

pub type LoginGatedFeature = &'static str;

type URLConstructorCallback = Box<dyn FnOnce(Option<&str>) -> String>;

/// AuthManager is a singleton model which manages the currently logged-in user's state.
/// If you need to access the state, use `AuthStateProvider`.
pub struct AuthManager {
    auth_state: Arc<AuthState>,
    server_api: Arc<ServerApi>,
    /// A generated state token that the web app must provide back to the client.
    pending_auth_state: Option<String>,
}

impl AuthManager {
    /// Creates a new instance of the AuthManager. The auth state must already be initialized through
    /// [`AuthStateProvider`].
    pub fn new(
        server_api: Arc<ServerApi>,
        ctx: &mut ModelContext<Self>,
    ) -> Self {
        let auth_state = AuthStateProvider::as_ref(ctx).get().clone();

        Self {
            auth_state,
            server_api,
            pending_auth_state: None,
        }
    }

    #[cfg(test)]
    pub fn new_for_test(ctx: &mut ModelContext<Self>) -> Self {
        let server_api_provider = ServerApiProvider::as_ref(ctx);
        let server_api = server_api_provider.get();
        let auth_state = AuthStateProvider::as_ref(ctx).get().clone();

        Self {
            auth_state,
            server_api,
            pending_auth_state: None,
        }
    }

    /// Fetches and ultimately sets the user's auth state from an auth payload.
    /// Typically, this function is triggered when a user clicks the intent link from their browser
    /// back to Warp after login (or pastes the URL in the app).
    pub fn initialize_user_from_auth_payload(
        &mut self,
        auth_payload: AuthRedirectPayload,
        enforce_state_validation: bool,
        ctx: &mut ModelContext<Self>,
    ) {
        let AuthRedirectPayload {
            refresh_token: _,
            user_uid,
            deleted_anonymous_user,
            state,
        } = auth_payload.clone();

        if let Some(received_state) = &state {
            if !self.consume_auth_state(received_state) {
                if self.should_silently_ignore_stale_redirect(&user_uid) {
                    log::info!(
                        "Dropping auth redirect with stale state for already-logged-in user"
                    );
                    return;
                }
                ctx.emit(AuthManagerEvent::AuthFailed(
                    UserAuthenticationError::InvalidStateParameter,
                ));
                return;
            }
        } else if enforce_state_validation {
            if self.should_silently_ignore_stale_redirect(&user_uid) {
                log::info!("Dropping auth redirect without state for already-logged-in user");
                return;
            }
            ctx.emit(AuthManagerEvent::AuthFailed(
                UserAuthenticationError::MissingStateParameter,
            ));
            return;
        }

        if self.auth_state.is_user_anonymous().unwrap_or_default() {
            let incoming_user_matches_current_user = match user_uid {
                None => false,
                Some(incoming_user_uid) => self
                    .auth_state
                    .user_id()
                    .map(|current_user_uid| current_user_uid == incoming_user_uid)
                    .unwrap_or_default(),
            };
            if !incoming_user_matches_current_user && !deleted_anonymous_user.unwrap_or_default() {
                ctx.emit(AuthManagerEvent::LoginOverrideDetected(auth_payload));
                return;
            }
            send_telemetry_from_ctx!(TelemetryEvent::AnonymousUserLinkedFromBrowser, ctx);
        }
    }

    pub fn resume_interrupted_auth_payload(
        &mut self,
        _auth_payload: AuthRedirectPayload,
        _ctx: &mut ModelContext<Self>,
    ) {
    }

    #[cfg(target_family = "wasm")]
    pub fn initialize_user_from_session_cookie(&self, _ctx: &mut ModelContext<Self>) {
    }

    /// Refreshes the user's auth state using their existing credentials.
    pub fn refresh_user(&self, _ctx: &mut ModelContext<Self>) {
    }

    /// Authenticate asynchronously using the OAuth2 device authorization flow.
    ///
    /// This is only used by the Warp CLI if running on a device that does not have the Warp app installed.
    #[cfg_attr(target_family = "wasm", allow(dead_code))]
    pub fn authorize_device(&self, _ctx: &mut ModelContext<Self>) {
    }

    /// Sets the user and credentials in auth state and persists to secure storage.
    /// Persistence depends on the credential type - currently, we only persist
    /// state if authenticated via a Firebase token.
    fn set_and_persist(
        &self,
        user: Option<User>,
        credentials: Option<Credentials>,
        ctx: &mut ModelContext<Self>,
    ) {
        self.auth_state.set_user(user);
        self.auth_state.set_credentials(credentials);
        self.persist(ctx);
    }

    /// Persists (or removes) the current user and credentials to/from secure storage,
    /// based on the current auth state.
    fn persist(&self, ctx: &mut ModelContext<Self>) {
        match self.auth_state.persist_action() {
            PersistAction::Persist(persisted_user) => {
                if persisted_user.auth_tokens.refresh_token.is_empty() {
                    log::warn!("Skipping user persistence due to empty refresh token");
                    return;
                }
                let _ = persisted_user.write_to_secure_storage(ctx).map_err(|err| {
                    log::warn!("Unable to persist user to secure storage: {err:?}");
                });
            }
            PersistAction::Remove => {
                let _ = PersistedUser::remove_from_secure_storage(ctx).map_err(|err| {
                    log::warn!("Unable to clear user from secure storage: {err:?}");
                });
            }
            PersistAction::DoNothing => {}
        }
    }

    /// Helper function for logging out the user.
    /// NOTE: You probably want to call auth::log_out instead; this only manages the auth state,
    /// it doesn't shut down any other user-dependent parts of the app.
    /// TODO(jeff): Can we move those pieces in here?
    pub(super) fn log_out(&mut self, ctx: &mut ModelContext<Self>) {
        // Clear any dangling CSRF token from an auth flow that was started but never
        // completed before this logout, so it can't be replayed against the next session
        // in the same process.
        self.pending_auth_state = None;
        self.set_and_persist(None, None, ctx);
    }

    /// Sets whether or not this user's Firebase credentials are invalid and thus needs to reauth.
    pub fn set_needs_reauth(&self, needs_reauth: bool, ctx: &mut ModelContext<Self>) {
        let became_true = self.auth_state.set_needs_reauth(needs_reauth);

        if became_true {
            send_telemetry_from_ctx!(TelemetryEvent::NeedsReauth, ctx);
            ctx.emit(AuthManagerEvent::NeedsReauth);
        }
    }

    pub fn attempt_login_gated_feature(
        &self,
        feature: LoginGatedFeature,
        auth_view_variant: AuthViewVariant,
        ctx: &mut ModelContext<Self>,
    ) {
        if self.auth_state.is_anonymous_or_logged_out() {
            send_telemetry_from_ctx!(
                TelemetryEvent::AnonymousUserAttemptLoginGatedFeature { feature },
                ctx
            );
            ctx.emit(AuthManagerEvent::AttemptedLoginGatedFeature { auth_view_variant });
        };
    }

    pub fn anonymous_user_hit_drive_object_limit(&self, ctx: &mut ModelContext<Self>) {
        if self.auth_state.is_anonymous_or_logged_out() {
            send_telemetry_from_ctx!(TelemetryEvent::AnonymousUserHitCloudObjectLimit, ctx);
            ctx.emit(AuthManagerEvent::AttemptedLoginGatedFeature {
                auth_view_variant: AuthViewVariant::HitDriveObjectLimitCloseable,
            });
        };
    }

    pub fn initiate_anonymous_user_linking(
        &self,
        _entrypoint: AnonymousUserSignupEntrypoint,
        _ctx: &mut ModelContext<Self>,
    ) {
    }

    pub fn open_url_maybe_with_anonymous_token(
        &self,
        ctx: &mut ModelContext<Self>,
        construct_url: URLConstructorCallback,
    ) {
        let url: String = construct_url(None);
        ctx.open_url(&url);
    }

    pub fn copy_anonymous_user_linking_url_to_clipboard(&self, _ctx: &mut ModelContext<Self>) {
    }

    /// Generates a unique state parameter for the authentication flow.
    fn generate_auth_state(&mut self) -> String {
        let state = Uuid::new_v4().to_string();
        self.pending_auth_state = Some(state.clone());
        state
    }

    pub fn sign_up_url(&mut self) -> String {
        let state = self.generate_auth_state();
        format!(
            // TODO: we should probably be able to remove the public_beta flag
            "{}/signup/remote?scheme={}&state={}&public_beta=true",
            ChannelState::server_root_url(),
            ChannelState::url_scheme(),
            state,
        )
    }

    pub fn sign_in_url(&mut self) -> String {
        let state = self.generate_auth_state();
        format!(
            "{}/login/remote?scheme={}&state={}",
            ChannelState::server_root_url(),
            ChannelState::url_scheme(),
            state,
        )
    }

    /// The upgrade confirmation page will kick the user back to the app with a refresh token
    /// if we send a `state` query param to /upgrade
    pub fn upgrade_url(&mut self) -> String {
        let state = self.generate_auth_state();
        format!(
            "{}/upgrade?scheme={}&state={}",
            ChannelState::server_root_url(),
            ChannelState::url_scheme(),
            state,
        )
    }

    pub fn login_options_url(&mut self, custom_token: &str) -> String {
        let state = self.generate_auth_state();
        format!(
            "{}/login_options/{}?state={}",
            ChannelState::server_root_url(),
            custom_token,
            state,
        )
    }

    pub fn link_sso_url(&mut self, email: &str) -> String {
        let state = self.generate_auth_state();
        format!(
            "{}/link_sso?email={}&state={}",
            ChannelState::server_root_url(),
            email,
            state,
        )
    }

    /// Validates and consumes the pending auth state token. Returns `true` if the
    /// provided state matches; in that case the pending state is cleared so the
    /// CSRF token is single-use. A subsequent call with the same value will fail.
    fn consume_auth_state(&mut self, received_state: &str) -> bool {
        if self.pending_auth_state.as_deref() == Some(received_state) {
            self.pending_auth_state = None;
            true
        } else {
            false
        }
    }

    /// Returns whether an auth redirect that failed state validation should be
    /// silently dropped rather than surfaced as an error. This covers the
    /// "user clicks the browser's 'Take me to Warp' button twice" case: once
    /// they're fully logged in, a second redirect targeting the same user is
    /// redundant and should not produce a user-visible error.
    fn should_silently_ignore_stale_redirect(&self, incoming_user_uid: &Option<UserUid>) -> bool {
        if self.auth_state.is_anonymous_or_logged_out() {
            return false;
        }
        match (self.auth_state.user_id(), incoming_user_uid) {
            (Some(current_uid), Some(incoming_uid)) => current_uid == *incoming_uid,
            _ => false,
        }
    }

    /// Sets the user as onboarded locally.
    pub fn set_user_onboarded(&self, ctx: &mut ModelContext<Self>) {
        self.auth_state.set_is_onboarded(true);
        self.persist(ctx);
    }

    pub fn create_anonymous_user(
        &mut self,
        _entrypoint: Option<AnonymousUserSignupEntrypoint>,
        _ctx: &mut ModelContext<Self>,
    ) {
        todo!("GraphQL backend removed")
    }
}

#[derive(Clone, Debug)]
pub struct PersistedCurrentUserInformation {
    pub email: String,
}

impl Entity for AuthManager {
    type Event = AuthManagerEvent;
}

impl SingletonEntity for AuthManager {}

#[cfg(test)]
#[path = "auth_manager_tests.rs"]
mod auth_manager_test;
