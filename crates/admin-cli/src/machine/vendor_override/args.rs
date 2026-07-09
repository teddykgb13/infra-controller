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

use carbide_uuid::machine::MachineId;
use clap::Parser;

#[derive(Parser, Debug, Clone)]
pub enum Args {
    #[clap(about = "Pin the Redfish BMC vendor for a machine")]
    Set(VendorOverrideSet),
    #[clap(about = "Clear the Redfish BMC vendor override for a machine")]
    Clear(VendorOverrideClear),
    #[clap(about = "Show the Redfish BMC vendor override for a machine")]
    Show(VendorOverrideShow),
}

#[derive(Parser, Debug, Clone)]
#[command(after_long_help = "\
EXAMPLES:

Force a machine's BMC vendor to Dell:
    $ nico-admin-cli machine vendor-override set 12345678-1234-5678-90ab-cdef01234567 \
    --vendor Dell

")]
pub struct VendorOverrideSet {
    #[clap(help = "The machine whose BMC vendor should be pinned")]
    pub machine: MachineId,
    #[clap(
        long,
        help = "RedfishVendor to force (e.g. Dell, Supermicro, NvidiaDpu, Hpe, Lenovo)"
    )]
    pub vendor: String,
}

#[derive(Parser, Debug, Clone)]
#[command(after_long_help = "\
EXAMPLES:

Clear a machine's BMC vendor override (return to automatic detection):
    $ nico-admin-cli machine vendor-override clear 12345678-1234-5678-90ab-cdef01234567

")]
pub struct VendorOverrideClear {
    #[clap(help = "The machine whose BMC vendor override should be cleared")]
    pub machine: MachineId,
}

#[derive(Parser, Debug, Clone)]
#[command(after_long_help = "\
EXAMPLES:

Show a machine's pinned BMC vendor (or that none is set):
    $ nico-admin-cli machine vendor-override show 12345678-1234-5678-90ab-cdef01234567

")]
pub struct VendorOverrideShow {
    #[clap(help = "The machine whose BMC vendor override should be shown")]
    pub machine: MachineId,
}
