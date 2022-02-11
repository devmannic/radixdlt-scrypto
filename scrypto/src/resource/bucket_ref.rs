use sbor::{describe::Type, *};

use crate::engine::*;
use crate::math::*;
use crate::misc::*;
use crate::resource::*;
use crate::rust::fmt;
use crate::rust::vec::Vec;
use crate::types::*;

/// Represents a reference to a bucket.
#[derive(Debug)]
pub struct BucketRef(u32);

impl Clone for BucketRef {
    fn clone(&self) -> Self {
        let input = CloneBucketRefInput {
            bucket_ref: self.this(),
        };
        let output: CloneBucketRefOutput = call_engine(CLONE_BUCKET_REF, input);

        output.bucket_ref
    }
}

impl BucketRef {
    fn this(&self) -> Self {
        Self(self.0)
    }

    /// Checks if the referenced bucket contains the given resource, and aborts if not so.
    pub fn check(&self, resource_def: ResourceDefRef) {
        if !self.contains(resource_def) {
            panic!("BucketRef check failed");
        }
    }

    pub fn check_non_fungible_key<F: Fn(&NonFungibleKey) -> bool>(
        &self,
        resource_def: ResourceDefRef,
        f: F,
    ) {
        if !self.contains(resource_def) || !self.get_non_fungible_keys().iter().any(f) {
            panic!("BucketRef check failed");
        }
    }

    /// Checks if the referenced bucket contains the given resource.
    pub fn contains(&self, resource_def: ResourceDefRef) -> bool {
        self.amount() > 0.into() && self.resource_def() == resource_def
    }

    /// Returns the resource amount within the bucket.
    pub fn amount(&self) -> Decimal {
        let input = GetBucketRefDecimalInput {
            bucket_ref: self.this(),
        };
        let output: GetBucketRefDecimalOutput = call_engine(GET_BUCKET_REF_AMOUNT, input);

        output.amount
    }

    /// Returns the resource definition of resources within the bucket.
    pub fn resource_def(&self) -> ResourceDefRef {
        let input = GetBucketRefResourceAddressInput {
            bucket_ref: self.this(),
        };
        let output: GetBucketRefResourceAddressOutput =
            call_engine(GET_BUCKET_REF_RESOURCE_DEF, input);

        output.resource_def
    }

    /// Get the non-fungible ids in the referenced bucket.
    pub fn get_non_fungible_keys(&self) -> Vec<NonFungibleKey> {
        let input = GetNonFungibleKeysInBucketRefInput {
            bucket_ref: self.this(),
        };
        let output: GetNonFungibleKeysInBucketRefOutput =
            call_engine(GET_NON_FUNGIBLE_KEYS_IN_BUCKET_REF, input);

        output.keys
    }

    /// Get the non-fungible id and panic if not singleton.
    pub fn get_non_fungible_key(&self) -> NonFungibleKey {
        let keys = self.get_non_fungible_keys();
        assert!(
            keys.len() == 1,
            "Expect 1 non-fungible, but found {}",
            keys.len()
        );
        keys[0].clone()
    }

    /// Destroys this reference.
    pub fn drop(self) {
        let input = DropBucketRefInput { bucket_ref: self };
        let _: DropBucketRefOutput = call_engine(DROP_BUCKET_REF, input);
    }

    /// Checks if the referenced bucket is empty.
    pub fn is_empty(&self) -> bool {
        self.amount() == 0.into()
    }
}

//========
// error
//========

#[derive(Debug, Clone)]
pub enum ParseBucketRefError {
    InvalidLength(usize),
}

#[cfg(not(feature = "alloc"))]
impl std::error::Error for ParseBucketRefError {}

#[cfg(not(feature = "alloc"))]
impl fmt::Display for ParseBucketRefError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

//========
// binary
//========

impl TryFrom<&[u8]> for BucketRef {
    type Error = ParseBucketRefError;

    fn try_from(slice: &[u8]) -> Result<Self, Self::Error> {
        match slice.len() {
            4 => Ok(Self(u32::from_le_bytes(copy_u8_array(slice)))),
            _ => Err(ParseBucketRefError::InvalidLength(slice.len())),
        }
    }
}

impl BucketRef {
    pub fn to_vec(&self) -> Vec<u8> {
        self.0.to_le_bytes().to_vec()
    }
}

custom_type!(BucketRef, CustomType::BucketRef, Vec::new());
