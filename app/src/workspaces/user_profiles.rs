#![allow(dead_code, unused_variables)]
use warpui::{Entity, ModelContext, SingletonEntity};
use crate::auth::UserUid;
#[derive(Debug, Clone, Default)]
pub struct UserProfileWithUID { pub firebase_uid: UserUid, pub display_name: Option<String>, pub email: String, pub photo_url: String }
impl UserProfileWithUID { pub fn displayable_identifier(&self) -> String { self.display_name.clone().unwrap_or_else(|| self.email.clone()) } }
pub fn user_profile_from_persistence<T: 'static>(_data: T) -> UserProfileWithUID { UserProfileWithUID::default() }
#[derive(Default)]
pub struct UserProfiles;
impl UserProfiles {
    pub fn new(_ctx: &mut ModelContext<Self>) -> Self { Self }
    pub fn displayable_identifier_for_uid(&self, _uid: UserUid) -> Option<String> { None }
    pub fn insert_profiles(&mut self, _profiles: Vec<UserProfileWithUID>) {}
    pub fn profile_for_uid(&self, _uid: UserUid) -> Option<&UserProfileWithUID> { None }
    pub fn displayable_identifier_for_email(&self, _email: &str) -> Option<String> { None }
}
impl Entity for UserProfiles { type Event = (); }
impl SingletonEntity for UserProfiles {}


