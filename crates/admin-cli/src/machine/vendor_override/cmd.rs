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
use rpc::Machine;

use super::args::{Args, VendorOverrideClear, VendorOverrideSet, VendorOverrideShow};
use crate::errors::{CarbideCliError, CarbideCliResult};
use crate::rpc::ApiClient;

pub async fn vendor_override(api_client: &ApiClient, cmd: Args) -> CarbideCliResult<()> {
    match cmd {
        Args::Set(cmd) => set(api_client, cmd).await,
        Args::Clear(cmd) => clear(api_client, cmd).await,
        Args::Show(cmd) => show(api_client, cmd).await,
    }
}

async fn fetch_machine(api_client: &ApiClient, machine_id: MachineId) -> CarbideCliResult<Machine> {
    let mut machines = api_client
        .get_machines_by_ids(&[machine_id])
        .await?
        .machines;
    machines.pop().ok_or_else(|| {
        CarbideCliError::GenericError(format!("Machine with ID {machine_id} was not found"))
    })
}

async fn set(api_client: &ApiClient, cmd: VendorOverrideSet) -> CarbideCliResult<()> {
    api_client
        .update_machine_bmc_vendor_override(cmd.machine, Some(cmd.vendor))
        .await
}

async fn clear(api_client: &ApiClient, cmd: VendorOverrideClear) -> CarbideCliResult<()> {
    api_client
        .update_machine_bmc_vendor_override(cmd.machine, None)
        .await
}

async fn show(api_client: &ApiClient, cmd: VendorOverrideShow) -> CarbideCliResult<()> {
    let machine = fetch_machine(api_client, cmd.machine).await?;
    match machine.bmc_vendor_override.as_deref() {
        Some(vendor) => println!("{vendor}"),
        None => println!("not set (automatic detection)"),
    }
    Ok(())
}
