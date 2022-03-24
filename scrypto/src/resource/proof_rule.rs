use crate::resource::*;
use crate::rust::vec;
use crate::rust::vec::Vec;
use sbor::*;
use scrypto::math::Decimal;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Describe, TypeId, Encode, Decode)]
pub struct SborPath(Vec<usize>);

impl SborPath {
    pub fn rel_path(&self) -> SborRelPath {
        SborRelPath(&self.0)
    }
}

pub struct SborRelPath<'a>(&'a [usize]);

impl<'a> SborRelPath<'a> {
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn pop(&self) -> (usize, Self) {
        let (index_slice, extended_path) = self.0.split_at(1);
        let index = index_slice[0];
        (index, SborRelPath(extended_path))
    }
}

impl From<&str> for SborPath {
    fn from(str: &str) -> Self {
        let path: Vec<usize> = str
            .split('/')
            .map(|s| s.parse::<usize>().unwrap())
            .collect();
        SborPath(path)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Describe, TypeId, Encode, Decode)]
pub enum ProofRuleResource {
    NonFungible(NonFungibleAddress),
    Resource(ResourceDefId),
    FromComponent(SborPath),
}

impl From<NonFungibleAddress> for ProofRuleResource {
    fn from(non_fungible_address: NonFungibleAddress) -> Self {
        ProofRuleResource::NonFungible(non_fungible_address)
    }
}

impl From<ResourceDefId> for ProofRuleResource {
    fn from(resource_def_id: ResourceDefId) -> Self {
        ProofRuleResource::Resource(resource_def_id)
    }
}

impl From<SborPath> for ProofRuleResource {
    fn from(path: SborPath) -> Self {
        ProofRuleResource::FromComponent(path)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Describe, TypeId, Encode, Decode)]
pub enum ProofRuleResourceList {
    StaticList(Vec<ProofRuleResource>),
    FromComponent(SborPath),
}

/// Authorization Rule
#[derive(Debug, Clone, PartialEq, Eq, Hash, Describe, TypeId, Encode, Decode)]
pub enum ProofRule {
    This(ProofRuleResource),
    AmountOf(Decimal, ProofRuleResource),
    CountOf(u8, ProofRuleResourceList),
    AllOf(ProofRuleResourceList),
    AnyOf(ProofRuleResourceList),
}

impl From<NonFungibleAddress> for ProofRule {
    fn from(non_fungible_address: NonFungibleAddress) -> Self {
        ProofRule::This(non_fungible_address.into())
    }
}

impl From<ResourceDefId> for ProofRule {
    fn from(resource_def_id: ResourceDefId) -> Self {
        ProofRule::This(resource_def_id.into())
    }
}

#[macro_export]
macro_rules! resource_list {
  ($($resource: expr),*) => ({
      let mut list: Vec<::scrypto::resource::ProofRuleResource> = Vec::new();
      $(
        list.push($resource.into());
      )*
      ::scrypto::resource::ProofRuleResourceList::StaticList(list)
  });
}

#[macro_export]
macro_rules! this {
    ($resource:expr) => {{
        ::scrypto::resource::ProofRule::This($resource.into())
    }};
}

#[macro_export]
macro_rules! any_of {
    ($($resource:expr),*) => ({
        ::scrypto::resource::ProofRule::AnyOf(resource_list!($($resource),+))
    });
}

#[macro_export]
macro_rules! all_of {
    ($($resource:expr),*) => ({
        ::scrypto::resource::ProofRule::AllOf(resource_list!($($resource),+))
    });
}

#[macro_export]
macro_rules! min_n_of {
    ($count:expr, $($resource:expr),+) => ({
        ::scrypto::resource::ProofRule::CountOf($count, resource_list!($($resource),+))
    })
}

#[macro_export]
macro_rules! min_amount_of {
    ($amount:expr, $resource:expr) => {
        ProofRule::AmountOf($amount, $resource.into())
    };
}
