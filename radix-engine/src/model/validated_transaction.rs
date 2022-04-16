use scrypto::crypto::*;
use scrypto::engine::types::*;
use scrypto::rust::collections::{BTreeSet, HashMap};
use scrypto::rust::string::String;
use scrypto::rust::vec;
use scrypto::rust::vec::Vec;
use scrypto::values::*;
use crate::engine::{IdAllocator, IdSpace, Process};
use crate::errors::RuntimeError;
use crate::errors::RuntimeError::ProofNotFound;
use crate::ledger::SubstateStore;

/// Represents a validated transaction
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedTransaction {
    pub raw_hash: Hash,
    pub instructions: Vec<ValidatedInstruction>,
    pub signers: Vec<EcdsaPublicKey>,
}

impl ValidatedTransaction {
    fn replace_ids(
        mut values: Vec<ScryptoValue>,
        proof_id_mapping: &mut HashMap<ProofId, ProofId>,
        bucket_id_mapping: &mut HashMap<BucketId, BucketId>
    ) -> Result<Vec<ScryptoValue>, RuntimeError> {
        for value in values.iter_mut() {
            value.replace_ids(proof_id_mapping, bucket_id_mapping)
                .map_err(|e| match e {
                    ScryptoValueReplaceError::BucketIdNotFound(bucket_id) => RuntimeError::BucketNotFound(bucket_id),
                    ScryptoValueReplaceError::ProofIdNotFound(proof_id) => RuntimeError::ProofNotFound(proof_id),
                })?;
        }
        Ok(values)
    }

    pub fn main<L: SubstateStore>(&self, proc: &mut Process<L>) -> (Vec<ScryptoValue>, Option<RuntimeError>) {
        let mut proof_id_mapping: HashMap<ProofId, ProofId> = HashMap::new();
        let mut bucket_id_mapping: HashMap<BucketId, BucketId> = HashMap::new();
        let mut id_allocator = IdAllocator::new(IdSpace::Transaction);
        let mut error: Option<RuntimeError> = None;
        let mut outputs = vec![];

        for inst in &self.instructions {
            let result = match inst {
                ValidatedInstruction::TakeFromWorktop { resource_address } => {
                    id_allocator.new_bucket_id()
                        .map_err(RuntimeError::IdAllocatorError)
                        .and_then(|new_id| {
                            proc
                                .take_all_from_worktop(*resource_address)
                                .map(|bucket_id| {
                                    bucket_id_mapping.insert(new_id, bucket_id);
                                    ScryptoValue::from_value(&scrypto::resource::Bucket(new_id))
                                })
                        })
                },
                ValidatedInstruction::TakeFromWorktopByAmount {
                    amount,
                    resource_address,
                } =>
                    id_allocator
                        .new_bucket_id()
                        .map_err(RuntimeError::IdAllocatorError)
                        .and_then(|new_id| {
                            proc
                                .take_from_worktop(*amount, *resource_address)
                                .map(|bucket_id| {
                                    bucket_id_mapping.insert(new_id, bucket_id);
                                    ScryptoValue::from_value(&scrypto::resource::Bucket(new_id))
                                })
                        }),
                ValidatedInstruction::TakeFromWorktopByIds {
                    ids,
                    resource_address,
                } =>
                    id_allocator
                        .new_bucket_id()
                        .map_err(RuntimeError::IdAllocatorError)
                        .and_then(|new_id| {
                            proc
                                .take_non_fungibles_from_worktop(ids, *resource_address)
                                .map(|bucket_id| {
                                    bucket_id_mapping.insert(new_id, bucket_id);
                                    ScryptoValue::from_value(&scrypto::resource::Bucket(new_id))
                                })
                        }),
                ValidatedInstruction::ReturnToWorktop { bucket_id } => {
                    bucket_id_mapping.remove(bucket_id)
                        .map(|real_id| proc.return_to_worktop(real_id))
                        .unwrap_or(Err(RuntimeError::BucketNotFound(*bucket_id)))
                }
                ValidatedInstruction::AssertWorktopContains { resource_address } => {
                    proc.assert_worktop_contains(*resource_address)
                }
                ValidatedInstruction::AssertWorktopContainsByAmount {
                    amount,
                    resource_address,
                } => proc.assert_worktop_contains_by_amount(*amount, *resource_address),
                ValidatedInstruction::AssertWorktopContainsByIds {
                    ids,
                    resource_address,
                } => proc.assert_worktop_contains_by_ids(&ids, *resource_address),
                ValidatedInstruction::PopFromAuthZone {} => {
                    id_allocator.new_proof_id()
                        .map_err(RuntimeError::IdAllocatorError)
                        .and_then(|new_id| {
                            proc
                                .pop_from_auth_zone()
                                .map(|proof_id| {
                                    proof_id_mapping.insert(new_id, proof_id);
                                    ScryptoValue::from_value(&scrypto::resource::Proof(new_id))
                                })
                        })
                },
                ValidatedInstruction::ClearAuthZone => {
                    proof_id_mapping.clear();
                    proc
                        .drop_all_auth_zone_proofs()
                        .map(|_| ScryptoValue::from_value(&()))
                },
                ValidatedInstruction::PushToAuthZone { proof_id } => {
                    match proof_id_mapping.remove(proof_id) {
                        Some(mapped_id) => {
                            proc
                                .push_to_auth_zone(mapped_id)
                                .map(|_| ScryptoValue::from_value(&()))
                        },
                        None => {
                            Err(RuntimeError::ProofNotFound(*proof_id))
                        }
                    }
                },
                ValidatedInstruction::CreateProofFromAuthZone { resource_address } =>
                    id_allocator.new_proof_id()
                        .map_err(RuntimeError::IdAllocatorError)
                        .and_then(|new_id| {
                            proc
                                .create_auth_zone_proof(*resource_address)
                                .map(|proof_id| {
                                    proof_id_mapping.insert(new_id, proof_id);
                                    ScryptoValue::from_value(&scrypto::resource::Proof(new_id))
                                })
                        }),
                ValidatedInstruction::CreateProofFromAuthZoneByAmount {
                    amount,
                    resource_address,
                } =>
                    id_allocator.new_proof_id()
                        .map_err(RuntimeError::IdAllocatorError)
                        .and_then(|new_id| {
                            proc
                                .create_auth_zone_proof_by_amount(*amount, *resource_address)
                                .map(|proof_id| {
                                    proof_id_mapping.insert(new_id, proof_id);
                                    ScryptoValue::from_value(&scrypto::resource::Proof(new_id))
                                })
                        }),
                ValidatedInstruction::CreateProofFromAuthZoneByIds {
                    ids,
                    resource_address,
                } =>
                    id_allocator.new_proof_id()
                        .map_err(RuntimeError::IdAllocatorError)
                        .and_then(|new_id| {
                            proc
                                .create_auth_zone_proof_by_ids(ids, *resource_address)
                                .map(|proof_id| {
                                    proof_id_mapping.insert(new_id, proof_id);
                                    ScryptoValue::from_value(&scrypto::resource::Proof(new_id))
                                })
                        }),
                ValidatedInstruction::CreateProofFromBucket { bucket_id } => {
                    id_allocator.new_proof_id()
                        .map_err(RuntimeError::IdAllocatorError)
                        .and_then(|new_id| {
                            proc
                                .txn_create_bucket_proof(*bucket_id)
                                .map(|proof_id| {
                                    proof_id_mapping.insert(new_id, proof_id);
                                    ScryptoValue::from_value(&scrypto::resource::Proof(new_id))
                                })
                        })
                },
                ValidatedInstruction::CloneProof { proof_id } =>
                    id_allocator
                        .new_proof_id()
                        .map_err(RuntimeError::IdAllocatorError)
                        .and_then(|new_id| {
                            proof_id_mapping
                                .get(proof_id)
                                .cloned()
                                .map(|real_id| {
                                    proc
                                        .clone_proof(real_id)
                                        .map(|proof_id| {
                                            proof_id_mapping.insert(new_id, proof_id);
                                            ScryptoValue::from_value(&scrypto::resource::Proof(new_id))
                                        })
                                })
                                .unwrap_or(Err(RuntimeError::ProofNotFound(*proof_id)))
                        }),
                ValidatedInstruction::DropProof { proof_id } => {
                    proof_id_mapping.remove(proof_id)
                        .map(|real_id| {
                            proc.drop_proof(real_id)
                                .map(|_| ScryptoValue::from_value(&()))
                        })
                        .unwrap_or(Err(ProofNotFound(*proof_id)))
                },
                ValidatedInstruction::CallFunction {
                    package_address,
                    blueprint_name,
                    function,
                    args,
                } => {
                    Self::replace_ids(args.clone(), &mut proof_id_mapping, &mut bucket_id_mapping)
                        .and_then(|args| {
                            proc.txn_call_function(
                                *package_address,
                                &blueprint_name,
                                &function,
                                args
                            )
                        })
                },
                ValidatedInstruction::CallMethod {
                    component_address,
                    method,
                    args,
                } => {
                    Self::replace_ids(args.clone(), &mut proof_id_mapping, &mut bucket_id_mapping)
                        .and_then(|args| {
                            proc.txn_call_method(*component_address, &method, args)
                        })
                },
                ValidatedInstruction::CallMethodWithAllResources {
                    component_address,
                    method,
                } => proc.call_method_with_all_resources(*component_address, &method),
                ValidatedInstruction::PublishPackage { code } => proc
                    .publish_package(code.clone())
                    .map(|package_address| ScryptoValue::from_value(&package_address)),
            };
            match result {
                Ok(data) => {
                    outputs.push(data);
                }
                Err(e) => {
                    error = Some(e);
                    break;
                }
            }
        }

        // drop all dangling proofs
        error = error.or_else(|| match proc.drop_all_proofs() {
            Ok(_) => None,
            Err(e) => Some(e),
        });

        // check resource
        error = error.or_else(|| match proc.check_resource() {
            Ok(_) => None,
            Err(e) => Some(e),
        });

        (outputs, error)
    }
}


#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidatedInstruction {
    TakeFromWorktop {
        resource_address: ResourceAddress,
    },
    TakeFromWorktopByAmount {
        amount: Decimal,
        resource_address: ResourceAddress,
    },
    TakeFromWorktopByIds {
        ids: BTreeSet<NonFungibleId>,
        resource_address: ResourceAddress,
    },
    ReturnToWorktop {
        bucket_id: BucketId,
    },
    AssertWorktopContains {
        resource_address: ResourceAddress,
    },
    AssertWorktopContainsByAmount {
        amount: Decimal,
        resource_address: ResourceAddress,
    },
    AssertWorktopContainsByIds {
        ids: BTreeSet<NonFungibleId>,
        resource_address: ResourceAddress,
    },
    PopFromAuthZone,
    PushToAuthZone {
        proof_id: ProofId,
    },
    ClearAuthZone,
    CreateProofFromAuthZone {
        resource_address: ResourceAddress,
    },
    CreateProofFromAuthZoneByAmount {
        amount: Decimal,
        resource_address: ResourceAddress,
    },
    CreateProofFromAuthZoneByIds {
        ids: BTreeSet<NonFungibleId>,
        resource_address: ResourceAddress,
    },
    CreateProofFromBucket {
        bucket_id: BucketId,
    },
    CloneProof {
        proof_id: ProofId,
    },
    DropProof {
        proof_id: ProofId,
    },
    CallFunction {
        package_address: PackageAddress,
        blueprint_name: String,
        function: String,
        args: Vec<ScryptoValue>,
    },
    CallMethod {
        component_address: ComponentAddress,
        method: String,
        args: Vec<ScryptoValue>,
    },
    CallMethodWithAllResources {
        component_address: ComponentAddress,
        method: String,
    },
    PublishPackage {
        code: Vec<u8>,
    },
}