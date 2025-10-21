use crate::generic_types::ObjectID;

#[derive(PartialEq, Eq, Hash, Clone)]
pub struct KeyServerInfoCacheKey {
    id: ObjectID,
}

impl KeyServerInfoCacheKey {
    pub fn new(id: ObjectID) -> Self {
        Self { id }
    }
}

#[derive(PartialEq, Eq, Hash, Clone)]
pub struct DerivedKeyCacheKey {
    request: Vec<u8>,
    server_id: ObjectID,
    threshold: u8,
}

impl DerivedKeyCacheKey {
    pub fn new(request: Vec<u8>, server_id: ObjectID, threshold: u8) -> Self {
        Self {
            request,
            server_id,
            threshold,
        }
    }
}
