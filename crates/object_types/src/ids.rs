use serde::{Deserialize, Serialize};

use crate::cloud_object::ObjectIdType;

pub use warp_types::{
    ClientId, HashableId, HashedSqliteId, ObjectUid, ParseServerIdError, ServerId, SyncId,
    ToServerId, SERVER_ID_LENGTH,
};

pub type ApiKeyUid = String;

pub trait ServerIdExt {
    fn sqlite_type_and_uid_hash(&self, object_id_type: ObjectIdType) -> HashedSqliteId;
}

impl ServerIdExt for ServerId {
    fn sqlite_type_and_uid_hash(&self, object_id_type: ObjectIdType) -> HashedSqliteId {
        format!("{}-{}", object_id_type.sqlite_prefix(), self)
    }
}

pub trait SyncIdExt {
    fn sqlite_uid_hash(&self, object_id_type: ObjectIdType) -> String;
}

impl SyncIdExt for SyncId {
    fn sqlite_uid_hash(&self, object_id_type: ObjectIdType) -> String {
        match self {
            SyncId::ClientId(id) => id.sqlite_hash(),
            SyncId::ServerId(id) => id.sqlite_type_and_uid_hash(object_id_type),
        }
    }
}

/// Removes the prefix from sqlite IDs to extract the UIDs.
#[allow(clippy::result_unit_err)]
pub fn parse_sqlite_id_to_uid(hashed_sqlite_id: HashedSqliteId) -> Result<ObjectUid, ()> {
    let Some(uid) = hashed_sqlite_id.split("-").last() else {
        return Err(());
    };
    Ok(uid.to_owned())
}

#[derive(Clone, Debug, PartialEq)]
pub struct ServerIdAndType {
    pub id: ServerId,
    pub id_type: ObjectIdType,
}

impl ServerIdAndType {
    pub fn sqlite_type_and_uid_hash(&self) -> HashedSqliteId {
        self.id.sqlite_type_and_uid_hash(self.id_type)
    }

    pub fn sqlite_uid_hash(&self) -> String {
        SyncId::ServerId(self.id).sqlite_uid_hash(self.id_type)
    }
}

#[derive(Clone, Debug, Copy, PartialEq, Eq, Hash, Default)]
pub struct FolderId(ServerId);
warp_types::server_id_traits! { FolderId, "Folder" }

impl From<FolderId> for SyncId {
    fn from(id: FolderId) -> Self {
        Self::ServerId(id.into())
    }
}

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct GenericStringObjectId(ServerId);
warp_types::server_id_traits! { GenericStringObjectId, "GenericStringObject" }

impl From<GenericStringObjectId> for SyncId {
    fn from(id: GenericStringObjectId) -> Self {
        Self::ServerId(id.into())
    }
}

impl GenericStringObjectId {
    pub fn uid(&self) -> ObjectUid {
        self.0.uid()
    }
}
