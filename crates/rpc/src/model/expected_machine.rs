/*
 * SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
 * SPDX-License-Identifier: Apache-2.0
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 * http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */
use std::net::IpAddr;

use mac_address::MacAddress;
use model::expected_machine::{
    BmcIpAllocationType, ExpectedHostNic, ExpectedMachine, ExpectedMachineData,
    ExpectedMachineRequest, HostDpuPolicy, HostLifecycleProfile, LinkedExpectedMachine,
    UnexpectedMachine,
};
use model::metadata::Metadata;
use model::network_segment::NetworkSegmentType;
use uuid::Uuid;

use crate as rpc;
use crate::errors::RpcDataConversionError;
use crate::model::RpcTryFrom;

impl From<HostDpuPolicy> for rpc::forge::HostDpuPolicy {
    fn from(policy: HostDpuPolicy) -> Self {
        match policy {
            HostDpuPolicy::Manage => rpc::forge::HostDpuPolicy::Manage,
            HostDpuPolicy::UseAsNic => rpc::forge::HostDpuPolicy::UseAsNic,
            HostDpuPolicy::Ignore => rpc::forge::HostDpuPolicy::Ignore,
        }
    }
}

impl From<rpc::forge::HostDpuPolicy> for HostDpuPolicy {
    fn from(policy: rpc::forge::HostDpuPolicy) -> Self {
        match policy {
            rpc::forge::HostDpuPolicy::Manage => HostDpuPolicy::Manage,
            rpc::forge::HostDpuPolicy::UseAsNic => HostDpuPolicy::UseAsNic,
            rpc::forge::HostDpuPolicy::Ignore => HostDpuPolicy::Ignore,
            // Unspecified (0) means "use the default". Unknown raw values are
            // rejected before reaching this typed conversion.
            rpc::forge::HostDpuPolicy::Unspecified => HostDpuPolicy::default(),
        }
    }
}

impl From<HostDpuPolicy> for rpc::forge::DpuMode {
    fn from(policy: HostDpuPolicy) -> Self {
        rpc::forge::HostDpuPolicy::from(policy).into()
    }
}

impl From<rpc::forge::DpuMode> for HostDpuPolicy {
    fn from(policy: rpc::forge::DpuMode) -> Self {
        rpc::forge::HostDpuPolicy::from(policy).into()
    }
}

fn host_dpu_policy_from_rpc(
    dpu_policy: Option<i32>,
    dpu_mode: Option<i32>,
) -> Result<HostDpuPolicy, RpcDataConversionError> {
    if let Some(value) = dpu_policy {
        match rpc::forge::HostDpuPolicy::try_from(value) {
            Ok(rpc::forge::HostDpuPolicy::Unspecified) => {}
            Ok(policy) => return Ok(policy.into()),
            Err(_) => {
                return Err(RpcDataConversionError::InvalidArgument(format!(
                    "Invalid dpu_policy: {value}"
                )));
            }
        }
    }

    Ok(dpu_mode
        .and_then(|value| rpc::forge::DpuMode::try_from(value).ok())
        .map(HostDpuPolicy::from)
        .unwrap_or_default())
}

fn host_dpu_policy_to_rpc_fields(policy: HostDpuPolicy) -> (Option<i32>, Option<i32>) {
    match policy {
        HostDpuPolicy::Manage => (None, None),
        policy => (
            Some(rpc::forge::DpuMode::from(policy) as i32),
            Some(rpc::forge::HostDpuPolicy::from(policy) as i32),
        ),
    }
}

impl rpc::forge::ExpectedMachine {
    /// Applies canonical-field precedence and mirrors a non-default policy onto
    /// both protobuf fields. Batch handlers use this to return the same
    /// compatibility shape as ordinary ExpectedMachine reads.
    pub fn normalize_dpu_policy_fields(&mut self) -> Result<(), RpcDataConversionError> {
        #[allow(deprecated)]
        let policy = host_dpu_policy_from_rpc(self.dpu_policy, self.dpu_mode)?;
        let (dpu_mode, dpu_policy) = host_dpu_policy_to_rpc_fields(policy);

        #[allow(deprecated)]
        {
            self.dpu_mode = dpu_mode;
        }
        self.dpu_policy = dpu_policy;
        Ok(())
    }
}

impl From<BmcIpAllocationType> for rpc::forge::BmcIpAllocationType {
    fn from(mode: BmcIpAllocationType) -> Self {
        match mode {
            BmcIpAllocationType::Auto => rpc::forge::BmcIpAllocationType::Auto,
            BmcIpAllocationType::Dynamic => rpc::forge::BmcIpAllocationType::Dynamic,
            BmcIpAllocationType::Fixed => rpc::forge::BmcIpAllocationType::Fixed,
            BmcIpAllocationType::Retained => rpc::forge::BmcIpAllocationType::Retained,
        }
    }
}

impl From<rpc::forge::BmcIpAllocationType> for BmcIpAllocationType {
    fn from(mode: rpc::forge::BmcIpAllocationType) -> Self {
        match mode {
            rpc::forge::BmcIpAllocationType::Auto => BmcIpAllocationType::Auto,
            rpc::forge::BmcIpAllocationType::Dynamic => BmcIpAllocationType::Dynamic,
            rpc::forge::BmcIpAllocationType::Fixed => BmcIpAllocationType::Fixed,
            rpc::forge::BmcIpAllocationType::Retained => BmcIpAllocationType::Retained,
            // Unspecified (0) or any unknown value means "use the default",
            // which preserves behavior for old clients that don't send the
            // field at all.
            rpc::forge::BmcIpAllocationType::Unspecified => BmcIpAllocationType::default(),
        }
    }
}

impl TryFrom<rpc::forge::ExpectedMachineRequest> for ExpectedMachineRequest {
    type Error = RpcDataConversionError;

    fn try_from(rpc: rpc::forge::ExpectedMachineRequest) -> Result<Self, Self::Error> {
        let id = rpc
            .id
            .map(|u| {
                Uuid::parse_str(&u.value)
                    .map_err(|_| RpcDataConversionError::InvalidArgument(u.value))
            })
            .transpose()?;
        let bmc_mac_address = if rpc.bmc_mac_address.is_empty() {
            None
        } else {
            Some(
                MacAddress::try_from(rpc.bmc_mac_address.as_str())
                    .map_err(|_| RpcDataConversionError::InvalidMacAddress(rpc.bmc_mac_address))?,
            )
        };

        Ok(ExpectedMachineRequest {
            id,
            bmc_mac_address,
        })
    }
}

impl From<ExpectedHostNic> for rpc::forge::ExpectedHostNic {
    fn from(expected_host_nic: ExpectedHostNic) -> Self {
        rpc::forge::ExpectedHostNic {
            mac_address: expected_host_nic.mac_address.to_string(),
            nic_type: expected_host_nic.nic_type,
            fixed_ip: expected_host_nic.fixed_ip.map(|ip| ip.to_string()),
            fixed_mask: expected_host_nic.fixed_mask,
            fixed_gateway: expected_host_nic.fixed_gateway.map(|ip| ip.to_string()),
            primary: expected_host_nic.primary,
            network_segment_type: expected_host_nic
                .network_segment_type
                .map(|segment_type| segment_type as i32),
        }
    }
}

impl TryFrom<rpc::forge::ExpectedHostNic> for ExpectedHostNic {
    type Error = RpcDataConversionError;

    fn try_from(expected_host_nic: rpc::forge::ExpectedHostNic) -> Result<Self, Self::Error> {
        let mac_address = expected_host_nic.mac_address.parse().map_err(|_| {
            RpcDataConversionError::InvalidMacAddress(expected_host_nic.mac_address.clone())
        })?;

        Ok(ExpectedHostNic {
            mac_address,
            nic_type: expected_host_nic.nic_type,
            fixed_ip: match expected_host_nic.fixed_ip.as_deref() {
                None | Some("") => None,
                Some(ip) => Some(ip.parse::<IpAddr>().map_err(|_| {
                    RpcDataConversionError::InvalidArgument(format!("Invalid fixed IP: {ip}"))
                })?),
            },
            fixed_mask: expected_host_nic.fixed_mask,
            fixed_gateway: match expected_host_nic.fixed_gateway.as_deref() {
                None | Some("") => None,
                Some(ip) => Some(ip.parse::<IpAddr>().map_err(|_| {
                    RpcDataConversionError::InvalidArgument(format!("Invalid fixed gateway: {ip}"))
                })?),
            },
            primary: expected_host_nic.primary,
            network_segment_type: expected_host_nic
                .network_segment_type
                .map(NetworkSegmentType::rpc_try_from)
                .transpose()?,
        })
    }
}

impl From<ExpectedMachine> for rpc::forge::ExpectedMachine {
    fn from(expected_machine: ExpectedMachine) -> Self {
        let host_nics = expected_machine
            .data
            .host_nics
            .iter()
            .map(|x| x.clone().into())
            .collect();
        let (dpu_mode, dpu_policy) =
            host_dpu_policy_to_rpc_fields(expected_machine.data.dpu_policy);
        rpc::forge::ExpectedMachine {
            id: expected_machine.id.map(|u| crate::common::Uuid {
                value: u.to_string(),
            }),
            bmc_mac_address: expected_machine.bmc_mac_address.to_string(),
            bmc_username: expected_machine.data.bmc_username,
            bmc_password: expected_machine.data.bmc_password,
            chassis_serial_number: expected_machine.data.serial_number,
            fallback_dpu_serial_numbers: expected_machine.data.fallback_dpu_serial_numbers,
            metadata: Some(expected_machine.data.metadata.into()),
            sku_id: expected_machine.data.sku_id,
            rack_id: expected_machine.data.rack_id,
            host_nics,
            default_pause_ingestion_and_poweron: expected_machine
                .data
                .default_pause_ingestion_and_poweron,
            // This should be removed after few releases.
            #[allow(deprecated)]
            dpf_enabled: expected_machine.data.dpf_enabled.unwrap_or(true),
            is_dpf_enabled: expected_machine.data.dpf_enabled,
            // Optional configured BMC IP (proto optional string).
            bmc_ip_address: expected_machine
                .data
                .bmc_ip_address
                .map(|ip| ip.to_string()),
            bmc_retain_credentials: expected_machine.data.bmc_retain_credentials.filter(|&v| v),
            // Mirror non-default policies into the deprecated field so older
            // generated clients continue to observe the stored value.
            #[allow(deprecated)]
            dpu_mode,
            // The canonical field is authoritative for new clients. As before,
            // the default policy is represented by absence on the wire.
            dpu_policy,
            // Only emit `bmc_ip_allocation` when it's non-default (Auto), so an
            // unset field round-trips and older clients keep falling back to Auto.
            bmc_ip_allocation: match expected_machine.data.bmc_ip_allocation {
                BmcIpAllocationType::Auto => None,
                other => Some(rpc::forge::BmcIpAllocationType::from(other) as i32),
            },
            host_lifecycle_profile: (!expected_machine.data.host_lifecycle_profile.is_empty())
                .then_some(rpc::forge::HostLifecycleProfile {
                    disable_lockdown: expected_machine
                        .data
                        .host_lifecycle_profile
                        .disable_lockdown,
                }),
        }
    }
}

impl From<LinkedExpectedMachine> for rpc::forge::LinkedExpectedMachine {
    fn from(m: LinkedExpectedMachine) -> rpc::forge::LinkedExpectedMachine {
        rpc::forge::LinkedExpectedMachine {
            chassis_serial_number: m.serial_number,
            bmc_mac_address: m.bmc_mac_address.to_string(),
            interface_id: m.interface_id.map(|u| u.to_string()),
            explored_endpoint_address: m.address.map(|addr| addr.to_string()),
            machine_id: m.machine_id,
            expected_machine_id: m.expected_machine_id.map(|id| crate::common::Uuid {
                value: id.to_string(),
            }),
        }
    }
}

impl From<UnexpectedMachine> for rpc::forge::UnexpectedMachine {
    fn from(m: UnexpectedMachine) -> rpc::forge::UnexpectedMachine {
        rpc::forge::UnexpectedMachine {
            address: m.address.to_string(),
            bmc_mac_address: m.bmc_mac_address.to_string(),
            machine_id: m.machine_id,
        }
    }
}

/// Parses gRPC `ExpectedMachine` into persisted model data, including optional `bmc_ip_address`
/// (empty or unset proto field becomes `None`; invalid strings fail conversion).
impl TryFrom<rpc::forge::ExpectedMachine> for ExpectedMachineData {
    type Error = RpcDataConversionError;

    fn try_from(em: rpc::forge::ExpectedMachine) -> Result<Self, Self::Error> {
        #[allow(deprecated)]
        let dpu_policy = host_dpu_policy_from_rpc(em.dpu_policy, em.dpu_mode)?;

        Ok(Self {
            bmc_username: em.bmc_username,
            bmc_password: em.bmc_password,
            serial_number: em.chassis_serial_number,
            fallback_dpu_serial_numbers: em.fallback_dpu_serial_numbers,
            sku_id: em.sku_id,
            metadata: metadata_from_request(em.metadata)?,
            host_nics: em
                .host_nics
                .into_iter()
                .map(ExpectedHostNic::try_from)
                .collect::<Result<Vec<_>, _>>()?,
            rack_id: em.rack_id,
            default_pause_ingestion_and_poweron: em.default_pause_ingestion_and_poweron,
            dpf_enabled: em.is_dpf_enabled,
            bmc_ip_address: match em.bmc_ip_address.as_deref() {
                None | Some("") => None,
                Some(s) => Some(s.parse::<IpAddr>().map_err(|_| {
                    RpcDataConversionError::InvalidArgument(format!("Invalid BMC IP address: {s}"))
                })?),
            },
            bmc_retain_credentials: em.bmc_retain_credentials,
            // A named canonical value wins. Unset/Unspecified falls back to
            // the deprecated field, whose old unknown-value behavior remains
            // the default policy.
            dpu_policy,
            // `bmc_ip_allocation` is optional on the wire; an unset field (and the
            // ::Unspecified discriminant) falls back to `BmcIpAllocationType::default()`
            // (::Auto), so old clients continue to behave as before. An unknown
            // discriminant is rejected rather than silently coerced to the default.
            bmc_ip_allocation: match em.bmc_ip_allocation {
                None => BmcIpAllocationType::default(),
                Some(i) => BmcIpAllocationType::from(
                    rpc::forge::BmcIpAllocationType::try_from(i).map_err(|_| {
                        RpcDataConversionError::InvalidArgument(format!(
                            "Invalid bmc_ip_allocation: {i}"
                        ))
                    })?,
                ),
            },
            host_lifecycle_profile: em
                .host_lifecycle_profile
                .map(|hlp| HostLifecycleProfile {
                    disable_lockdown: hlp.disable_lockdown,
                })
                .unwrap_or_default(),
        })
    }
}

/// If Metadata is retrieved as part of the ExpectedMachine creation, validate and use the Metadata
/// Otherwise assume empty Metadata
fn metadata_from_request(
    opt_metadata: Option<crate::forge::Metadata>,
) -> Result<Metadata, RpcDataConversionError> {
    Ok(match opt_metadata {
        None => Metadata {
            name: "".to_string(),
            description: "".to_string(),
            labels: Default::default(),
        },
        Some(m) => {
            // Note that this is unvalidated Metadata. It can contain non-ASCII names
            // and
            let m: Metadata = m.try_into()?;
            m.validate(false)
                .map_err(|e| RpcDataConversionError::InvalidArgument(e.to_string()))?;
            m
        }
    })
}

// default_uuid removed; ids are optional to support legacy rows with NULL ids

#[cfg(test)]
mod tests {
    use carbide_test_support::Outcome::*;
    use carbide_test_support::{Check, check_values, scenarios, value_scenarios};
    use prost::Message;

    use super::*;

    /// The canonical protobuf policy maps each named value onto its model twin,
    /// while Unspecified retains the model default.
    #[test]
    fn rpc_host_dpu_policy_maps_to_model() {
        value_scenarios!(
            run = HostDpuPolicy::from;
            "unspecified maps to default" {
                rpc::forge::HostDpuPolicy::Unspecified => HostDpuPolicy::default(),
            }

            "manage round trips" {
                rpc::forge::HostDpuPolicy::Manage => HostDpuPolicy::Manage,
            }

            "use as NIC round trips" {
                rpc::forge::HostDpuPolicy::UseAsNic => HostDpuPolicy::UseAsNic,
            }

            "ignore round trips" {
                rpc::forge::HostDpuPolicy::Ignore => HostDpuPolicy::Ignore,
            }
        );
    }

    /// The deprecated protobuf enum remains a supported ingress boundary for
    /// clients generated before `ExpectedMachine.dpu_policy` was introduced.
    #[test]
    fn legacy_rpc_dpu_mode_maps_to_model() {
        value_scenarios!(
            run = HostDpuPolicy::from;
            "unspecified maps to default" {
                rpc::forge::DpuMode::Unspecified => HostDpuPolicy::default(),
            }

            "DPU mode maps to manage" {
                rpc::forge::DpuMode::DpuMode => HostDpuPolicy::Manage,
            }

            "NIC mode maps to use as NIC" {
                rpc::forge::DpuMode::NicMode => HostDpuPolicy::UseAsNic,
            }

            "no DPU maps to ignore" {
                rpc::forge::DpuMode::NoDpu => HostDpuPolicy::Ignore,
            }
        );
    }

    /// The host DPU policy default is Manage, which is what the Unspecified mapping
    /// above relies on.
    #[test]
    fn host_dpu_policy_default_is_manage() {
        assert_eq!(HostDpuPolicy::default(), HostDpuPolicy::Manage);
    }

    /// The policy refactor must not change the protobuf bytes consumed by
    /// existing clients: field 16 remains a varint and values 0 through 3 retain
    /// their legacy meanings.
    #[allow(deprecated)]
    #[test]
    fn host_dpu_policy_preserves_legacy_wire_encoding() {
        check_values(
            [
                Check {
                    scenario: "unspecified remains 0",
                    input: rpc::forge::DpuMode::Unspecified,
                    expect: vec![0x80, 0x01, 0x00],
                },
                Check {
                    scenario: "manage remains DPU_MODE 1",
                    input: rpc::forge::DpuMode::DpuMode,
                    expect: vec![0x80, 0x01, 0x01],
                },
                Check {
                    scenario: "use-as-NIC remains NIC_MODE 2",
                    input: rpc::forge::DpuMode::NicMode,
                    expect: vec![0x80, 0x01, 0x02],
                },
                Check {
                    scenario: "ignore remains NO_DPU 3",
                    input: rpc::forge::DpuMode::NoDpu,
                    expect: vec![0x80, 0x01, 0x03],
                },
            ],
            |policy| {
                rpc::forge::ExpectedMachine {
                    dpu_mode: Some(policy as i32),
                    ..Default::default()
                }
                .encode_to_vec()
            },
        );
    }

    /// The canonical policy uses the additive field 19 while retaining the
    /// same enum discriminants as the legacy compatibility mapping.
    #[test]
    fn host_dpu_policy_uses_additive_canonical_wire_field() {
        check_values(
            [
                Check {
                    scenario: "unspecified is 0",
                    input: rpc::forge::HostDpuPolicy::Unspecified,
                    expect: vec![0x98, 0x01, 0x00],
                },
                Check {
                    scenario: "manage is 1",
                    input: rpc::forge::HostDpuPolicy::Manage,
                    expect: vec![0x98, 0x01, 0x01],
                },
                Check {
                    scenario: "use-as-NIC is 2",
                    input: rpc::forge::HostDpuPolicy::UseAsNic,
                    expect: vec![0x98, 0x01, 0x02],
                },
                Check {
                    scenario: "ignore is 3",
                    input: rpc::forge::HostDpuPolicy::Ignore,
                    expect: vec![0x98, 0x01, 0x03],
                },
            ],
            |policy| {
                rpc::forge::ExpectedMachine {
                    dpu_policy: Some(policy as i32),
                    ..Default::default()
                }
                .encode_to_vec()
            },
        );
    }

    /// Reflection-backed clients see both the deprecated compatibility surface
    /// and the canonical policy surface with stable names and tags.
    #[test]
    fn host_dpu_policy_descriptor_exposes_canonical_and_legacy_fields() {
        let descriptor_set =
            prost_types::FileDescriptorSet::decode(rpc::REFLECTION_API_SERVICE_DESCRIPTOR).unwrap();
        let forge = descriptor_set
            .file
            .iter()
            .find(|file| file.package.as_deref() == Some("forge"))
            .unwrap();
        let expected_machine = forge
            .message_type
            .iter()
            .find(|message| message.name.as_deref() == Some("ExpectedMachine"))
            .unwrap();
        let legacy_field = expected_machine
            .field
            .iter()
            .find(|field| field.number == Some(16))
            .unwrap();

        assert_eq!(legacy_field.name.as_deref(), Some("dpu_mode"));
        assert_eq!(legacy_field.json_name.as_deref(), Some("dpuMode"));
        assert_eq!(legacy_field.type_name.as_deref(), Some(".forge.DpuMode"));
        assert_eq!(
            legacy_field
                .options
                .as_ref()
                .and_then(|options| options.deprecated),
            Some(true)
        );

        let canonical_field = expected_machine
            .field
            .iter()
            .find(|field| field.number == Some(19))
            .unwrap();
        assert_eq!(canonical_field.name.as_deref(), Some("dpu_policy"));
        assert_eq!(canonical_field.json_name.as_deref(), Some("dpuPolicy"));
        assert_eq!(
            canonical_field.type_name.as_deref(),
            Some(".forge.HostDpuPolicy")
        );

        let policy_enum = forge
            .enum_type
            .iter()
            .find(|enumeration| enumeration.name.as_deref() == Some("DpuMode"))
            .unwrap();
        let names_and_numbers = policy_enum
            .value
            .iter()
            .map(|value| (value.name.as_deref().unwrap(), value.number.unwrap()))
            .collect::<Vec<_>>();
        for legacy_value in [
            ("DPU_MODE_UNSPECIFIED", 0),
            ("DPU_MODE", 1),
            ("NIC_MODE", 2),
            ("NO_DPU", 3),
        ] {
            assert!(names_and_numbers.contains(&legacy_value));
        }

        let canonical_enum = forge
            .enum_type
            .iter()
            .find(|enumeration| enumeration.name.as_deref() == Some("HostDpuPolicy"))
            .unwrap();
        let canonical_names_and_numbers = canonical_enum
            .value
            .iter()
            .map(|value| (value.name.as_deref().unwrap(), value.number.unwrap()))
            .collect::<Vec<_>>();
        for canonical_value in [
            ("HOST_DPU_POLICY_UNSPECIFIED", 0),
            ("HOST_DPU_POLICY_MANAGE", 1),
            ("HOST_DPU_POLICY_USE_AS_NIC", 2),
            ("HOST_DPU_POLICY_IGNORE", 3),
        ] {
            assert!(canonical_names_and_numbers.contains(&canonical_value));
        }

        assert_eq!(
            rpc::forge::HostDpuPolicy::Manage.as_str_name(),
            "HOST_DPU_POLICY_MANAGE"
        );
        assert_eq!(rpc::forge::DpuMode::DpuMode.as_str_name(), "DPU_MODE");
    }

    #[test]
    fn expected_machine_prefers_canonical_dpu_policy_and_falls_back_to_legacy() {
        scenarios!(
            run = |(dpu_policy, dpu_mode)| {
                host_dpu_policy_from_rpc(dpu_policy, dpu_mode).map_err(drop)
            };
            "both fields omitted" {
                (None, None) => Yields(HostDpuPolicy::Manage),
            }

            "canonical manage overrides legacy no-DPU" {
                (
                    Some(rpc::forge::HostDpuPolicy::Manage as i32),
                    Some(rpc::forge::DpuMode::NoDpu as i32),
                ) => Yields(HostDpuPolicy::Manage),
            }

            "canonical use-as-NIC overrides legacy no-DPU" {
                (
                    Some(rpc::forge::HostDpuPolicy::UseAsNic as i32),
                    Some(rpc::forge::DpuMode::NoDpu as i32),
                ) => Yields(HostDpuPolicy::UseAsNic),
            }

            "canonical ignore overrides legacy DPU mode" {
                (
                    Some(rpc::forge::HostDpuPolicy::Ignore as i32),
                    Some(rpc::forge::DpuMode::DpuMode as i32),
                ) => Yields(HostDpuPolicy::Ignore),
            }

            "canonical unspecified falls back to legacy NIC mode" {
                (
                    Some(rpc::forge::HostDpuPolicy::Unspecified as i32),
                    Some(rpc::forge::DpuMode::NicMode as i32),
                ) => Yields(HostDpuPolicy::UseAsNic),
            }

            "missing canonical field falls back to legacy no-DPU" {
                (None, Some(rpc::forge::DpuMode::NoDpu as i32))
                    => Yields(HostDpuPolicy::Ignore),
            }

            "unknown legacy value preserves the old default" {
                (None, Some(i32::MAX)) => Yields(HostDpuPolicy::Manage),
            }

            "unknown canonical value is rejected" {
                (Some(i32::MAX), Some(rpc::forge::DpuMode::NicMode as i32)) => Fails,
            }
        );
    }

    #[allow(deprecated)]
    #[test]
    fn expected_machine_mirrors_non_default_policy_to_both_rpc_fields() {
        scenarios!(
            run = |policy| {
                let expected_machine = ExpectedMachine {
                    id: None,
                    bmc_mac_address: "AA:BB:CC:DD:EE:FF".parse().map_err(drop)?,
                    data: ExpectedMachineData {
                        dpu_policy: policy,
                        ..Default::default()
                    },
                };
                let rpc_machine = rpc::forge::ExpectedMachine::from(expected_machine);
                Ok::<_, ()>((rpc_machine.dpu_mode, rpc_machine.dpu_policy))
            };
            "default manage remains absent" {
                HostDpuPolicy::Manage => Yields((None, None)),
            }

            "use as NIC is mirrored" {
                HostDpuPolicy::UseAsNic => Yields((
                    Some(rpc::forge::DpuMode::NicMode as i32),
                    Some(rpc::forge::HostDpuPolicy::UseAsNic as i32),
                )),
            }

            "ignore is mirrored" {
                HostDpuPolicy::Ignore => Yields((
                    Some(rpc::forge::DpuMode::NoDpu as i32),
                    Some(rpc::forge::HostDpuPolicy::Ignore as i32),
                )),
            }
        );
    }

    /// Direct Serde consumers (notably admin file input) accept both the new
    /// policy vocabulary and every legacy DpuMode spelling they used before.
    #[test]
    fn host_dpu_policy_deserializes_canonical_and_legacy_values() {
        scenarios!(
            run = |json| serde_json::from_str::<rpc::forge::HostDpuPolicy>(json).map_err(drop);
            "canonical policy values" {
                r#""manage""# => Yields(rpc::forge::HostDpuPolicy::Manage),
                r#""use_as_nic""# => Yields(rpc::forge::HostDpuPolicy::UseAsNic),
                r#""ignore""# => Yields(rpc::forge::HostDpuPolicy::Ignore),
            }

            "legacy Rust enum values" {
                r#""DpuMode""# => Yields(rpc::forge::HostDpuPolicy::Manage),
                r#""NicMode""# => Yields(rpc::forge::HostDpuPolicy::UseAsNic),
                r#""NoDpu""# => Yields(rpc::forge::HostDpuPolicy::Ignore),
            }

            "legacy config values" {
                r#""dpu_mode""# => Yields(rpc::forge::HostDpuPolicy::Manage),
                r#""nic_mode""# => Yields(rpc::forge::HostDpuPolicy::UseAsNic),
                r#""no_dpu""# => Yields(rpc::forge::HostDpuPolicy::Ignore),
            }
        );
    }

    #[test]
    fn host_dpu_policy_serializes_and_round_trips_canonical_values() {
        scenarios!(
            run = |policy| {
                let json = serde_json::to_string(&policy).map_err(drop)?;
                let recovered =
                    serde_json::from_str::<rpc::forge::HostDpuPolicy>(&json).map_err(drop)?;
                Ok::<_, ()>((json, recovered))
            };
            "unspecified boundary value" {
                rpc::forge::HostDpuPolicy::Unspecified => Yields((
                    r#""unspecified""#.to_string(),
                    rpc::forge::HostDpuPolicy::Unspecified,
                )),
            }

            "manage" {
                rpc::forge::HostDpuPolicy::Manage => Yields((
                    r#""manage""#.to_string(),
                    rpc::forge::HostDpuPolicy::Manage,
                )),
            }

            "use as NIC" {
                rpc::forge::HostDpuPolicy::UseAsNic => Yields((
                    r#""use_as_nic""#.to_string(),
                    rpc::forge::HostDpuPolicy::UseAsNic,
                )),
            }

            "ignore" {
                rpc::forge::HostDpuPolicy::Ignore => Yields((
                    r#""ignore""#.to_string(),
                    rpc::forge::HostDpuPolicy::Ignore,
                )),
            }
        );
    }

    /// Deprecated DpuMode JSON output remains stable for consumers using the
    /// generated Rust enum names, even though the CLI exposes policy vocabulary.
    #[test]
    fn legacy_rpc_dpu_mode_serializes_and_round_trips_legacy_values() {
        scenarios!(
            run = |policy| {
                let json = serde_json::to_string(&policy).map_err(drop)?;
                let recovered =
                    serde_json::from_str::<rpc::forge::DpuMode>(&json).map_err(drop)?;
                Ok::<_, ()>((json, recovered))
            };
            "unspecified boundary value" {
                rpc::forge::DpuMode::Unspecified => Yields((
                    r#""Unspecified""#.to_string(),
                    rpc::forge::DpuMode::Unspecified,
                )),
            }

            "DPU mode" {
                rpc::forge::DpuMode::DpuMode => Yields((
                    r#""DpuMode""#.to_string(),
                    rpc::forge::DpuMode::DpuMode,
                )),
            }

            "NIC mode" {
                rpc::forge::DpuMode::NicMode => Yields((
                    r#""NicMode""#.to_string(),
                    rpc::forge::DpuMode::NicMode,
                )),
            }

            "no DPU" {
                rpc::forge::DpuMode::NoDpu => Yields((
                    r#""NoDpu""#.to_string(),
                    rpc::forge::DpuMode::NoDpu,
                )),
            }
        );
    }

    /// `BmcIpAllocationType::from(rpc::forge::BmcIpAllocationType)` maps each
    /// named variant onto its model twin, and Unspecified (what old clients send)
    /// onto the default — keeping existing deployments behaving as before. The
    /// named rows also stand in for the model -> rpc -> model round trip, since
    /// the rpc input is exactly what `rpc::forge::BmcIpAllocationType::from(model)`
    /// produces.
    #[test]
    fn rpc_bmc_ip_allocation_maps_to_model() {
        value_scenarios!(
            run = BmcIpAllocationType::from;
            "unspecified maps to default" {
                rpc::forge::BmcIpAllocationType::Unspecified => BmcIpAllocationType::default(),
            }

            "auto round trips" {
                rpc::forge::BmcIpAllocationType::Auto => BmcIpAllocationType::Auto,
            }

            "dynamic round trips" {
                rpc::forge::BmcIpAllocationType::Dynamic => BmcIpAllocationType::Dynamic,
            }

            "fixed round trips" {
                rpc::forge::BmcIpAllocationType::Fixed => BmcIpAllocationType::Fixed,
            }

            "retained round trips" {
                rpc::forge::BmcIpAllocationType::Retained => BmcIpAllocationType::Retained,
            }
        );
    }

    /// The BmcIpAllocationType default is Auto, which is what the Unspecified
    /// mapping above relies on.
    #[test]
    fn bmc_ip_allocation_default_is_auto() {
        assert_eq!(BmcIpAllocationType::default(), BmcIpAllocationType::Auto);
    }

    #[test]
    fn expected_host_nic_rejects_invalid_mac_address() {
        let err = ExpectedHostNic::try_from(rpc::forge::ExpectedHostNic {
            mac_address: "not-a-mac".into(),
            ..Default::default()
        })
        .unwrap_err();

        assert!(
            matches!(err, RpcDataConversionError::InvalidMacAddress(mac) if mac == "not-a-mac")
        );
    }

    #[test]
    fn expected_machine_data_rejects_invalid_host_nic_mac_address() {
        let mut rpc_machine = make_rpc_expected_machine(None);
        rpc_machine.host_nics.push(rpc::forge::ExpectedHostNic {
            mac_address: "not-a-mac".into(),
            ..Default::default()
        });

        let Err(err) = ExpectedMachineData::try_from(rpc_machine) else {
            panic!("expected invalid host NIC MAC address");
        };

        assert!(
            matches!(err, RpcDataConversionError::InvalidMacAddress(mac) if mac == "not-a-mac")
        );
    }

    fn make_rpc_expected_machine(disable_lockdown: Option<bool>) -> rpc::forge::ExpectedMachine {
        rpc::forge::ExpectedMachine {
            bmc_mac_address: "AA:BB:CC:DD:EE:FF".into(),
            bmc_username: "root".into(),
            bmc_password: "pass".into(),
            chassis_serial_number: "SN-1".into(),
            host_lifecycle_profile: disable_lockdown.map(|dl| rpc::forge::HostLifecycleProfile {
                disable_lockdown: Some(dl),
            }),
            ..Default::default()
        }
    }

    /// `disable_lockdown` survives the rpc -> data -> rpc round trip: each input
    /// is projected to (data-side disable_lockdown, back-side host_lifecycle_profile
    /// mapped to its disable_lockdown). A `None` input yields no profile on the way
    /// back, so the back-side projection is `None` rather than `Some(None)`.
    #[test]
    fn disable_lockdown_round_trips_through_proto() {
        scenarios!(
            run = |disable_lockdown| {
                let data =
                    ExpectedMachineData::try_from(make_rpc_expected_machine(disable_lockdown))
                        .map_err(drop)?;
                let data_side = data.host_lifecycle_profile.disable_lockdown;

                let em = ExpectedMachine {
                    id: None,
                    bmc_mac_address: "AA:BB:CC:DD:EE:FF".parse().map_err(drop)?,
                    data,
                };
                let back: rpc::forge::ExpectedMachine = em.into();
                let back_side = back.host_lifecycle_profile.map(|p| p.disable_lockdown);

                Ok::<_, ()>((data_side, back_side))
            };
            "true" {
                Some(true) => Yields((Some(true), Some(Some(true)))),
            }

            "false" {
                Some(false) => Yields((Some(false), Some(Some(false)))),
            }

            "none" {
                None => Yields((None, None)),
            }
        );
    }
}
