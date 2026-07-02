#![allow(dead_code, unused_variables, unused_imports)]
use std::sync::Arc;
use warpui::{Entity, SingletonEntity};
use crate::objects::InitiatedBy;
use crate::ids::{ClientId, SyncId};
pub use object_types::cloud_object::SerializedModel;

#[derive(Default)]
pub struct SyncQueue;
impl Entity for SyncQueue { type Event = SyncQueueEvent; }
impl SingletonEntity for SyncQueue {}
impl SyncQueue {
    pub fn new<A: 'static, B: 'static>(_a: A, _b: B) -> Self { Self }
}

#[derive(Debug, Clone)]
pub enum SyncQueueEvent {}

#[derive(Debug, Clone)]
pub struct QueueItem;
impl QueueItem {
    pub fn is_synced(&self) -> bool { true }
}

#[derive(Debug, Clone)]
pub struct QueueItemId;

#[derive(Debug, Clone)]
pub enum CreationFailureReason { Unknown }

#[derive(Debug, Clone)]
pub struct GenericStringObjectToCreate;