# Multi-Node Maintenance Executor (MNME)

## Software Design Document

## Revision History

| Version | Date | Modified By | Description |
| :---: | :---: | :---- | :---- |
| 0.1 | 06/25/2026 | Matthias Einwag | Initial version |


# Overview 

This document introduces a new component within NICo-core which allows execution of multi-node maintenance operations in a conflict-free and efficient manner.

Multi-Node maintenance executor (MNME) is a replacement for the current maintenance execution via the rack-state machine. It improves over it in the following areas:
- MNME allows users to apply multiple maintenance operations to different trays within rack concurrently as long as it determines that there are no side-effects between the operations.
- MNME allows to express safe maintenance operations for systems where maintenance side-effects do not exactly match rack boundaries. It supports architectures where the rack is no longer equivalent to the scaleup domain. That includes systems where
  - the maintenance domain (e.g. NVLink failure domain) is smaller than a rack - e.g. if a GB200 rack is subdivided into multiple "mini racks"
  - the maintenance domain spans multiple racks due to a any kind of shared resources

Implementing MNME moves most logic out of the Rack State Machine into a different component, which reduces the need for having a type visible in NICo (`message Rack`) which does not actually physically exist.

### What are multi-node maintenance operations?

Multi-node maintenance operations are any kind of maintenance operations that either directly or indirectly affect more than one node. Node here references to any concrete hardware component that NICo manages:
- Compute Trays
- Switches
- Powershelves

A multi-node maintenance operation which **directly** affects multiple nodes is the update of firmware on 5 CPU trays: All of these trays will be unavailable for their regular workloads for the duration of the update application.

A multi-node maintenance operation which **indirectly** affects multiple nodes is the reconfiguration of a NVLink domain: While it might only directly lead to a state change on a single switch tray, all attached compute trays in the datacenter might observe a service disruption.


## MNME - User experience and APIs

NICo site administrators use MNME related APIs in a similar fashion to the current rack scale maintenance APIs:

```proto
rpc OnDemandRackMaintenance(RackMaintenanceOnDemandRequest) returns (RackMaintenanceOnDemandResponse);

message RackMaintenanceOnDemandRequest {
  common.RackId rack_id = 1;
  RackMaintenanceScope scope = 2;
}

message RackMaintenanceScope {
  repeated string machine_ids = 1;
  repeated string switch_ids = 2;
  repeated string power_shelf_ids = 3;
  // Which maintenance activities to run. Empty means all activities.
  repeated MaintenanceActivityConfig activities = 4;
}

message MaintenanceActivityConfig {
  oneof activity {
    FirmwareUpgradeActivity firmware_upgrade = 1;
    ConfigureNmxClusterActivity configure_nmx_cluster = 2;
    PowerSequenceActivity power_sequence = 3;
    NvosUpdateActivity nvos_update = 4;
  }
}
```

There are however multiple key differences:
1. The user no longer specifies a `rack_id`, but just the set of nodes to perform operations on. The system will automatically detect relationships between the affected nodes and any others managed by NICo and make sure that all nodes are in a "safe" state which allows the changes to be performed in a non-service impacting fashion. E.g.
   1. if a NVSwitch update is triggered, MNME will automatically detect all compute trays associated with the same NVLink domain as the switch, and make sure they are in safeguarded state (explanation below) before applying the update - the compute tray nodes do not have to get referenced.
   2. if an update is scheduled that touches a certain set of compute trays in the rack (e.g. MachineA, MachineB) while other update on other machines are in progress (MachineC), the update will get executed immediately. If the set of machines is however overlapping, the update is scheduled behind the previous.
2. NICo site-admins can explicitly reference additional nodes that the maintenance operation is impacting - for use-cases where the system internally is not able to determine the full impact. Every referenced node will also be moved into a safeguarded state.
3. NICo site-admins can explicitly choose to perform the update application without moving the nodes into a safeguarded state. This expert option can be used.
4. Site admins will observe the progress of the update application no longer via status changes on the rack state machine, but by querying the maintenance status for the ID returned by the maintenance request.

### Proposed API shape

```proto
rpc ScheduleMultiNodeMaintenance(MultiNodeMaintenanceRequest) returns (MultiNodeMaintenanceResponse);
rpc FindMultiNodeMaintenanceOperationsByIds(MultiNodeMaintenanceIds) returns (FindMultiNodeMaintenanceOperationsByIdsResponse);

message MultiNodeMaintenanceRequest {
  // The set of nodes that the operation will be carried out on directly.
  // Additional nodes which will be affected by the change will be determined by the system automatically
  MaintenanceScope scope = 1;

  // Which maintenance activities to run. Empty means all activities.
  repeated MaintenanceActivityConfig activities = 2;

  // Additional nodes that need to be moved into a safeguarded state while the maintenance is performed
  MaintenanceScope extra_safeguard_nodes = 3;

  // Allows to skip any safeguarding operations. The maintenance operation will be performed immediately
  bool danger_skip_safeguarding = 4;

  // By default the system will only start updates on nodes if all of them are healthy. Adding health alert classifications to this list will allow to enforce update scheduling on nodes which are unhealthy for specific reasons
  repeated string ignored_health_alert_classifications = 5;
}

message MultiNodeMaintenanceResponse {
  // An ID that can be used to query the status of the maintenance operation
  common.UUID maintenance_id;
}

// Contains the set of nodes a maintenance operation is impacting
message MaintenanceScope {
  repeated string machine_ids = 1;
  repeated string switch_ids = 2;
  repeated string power_shelf_ids = 3;
  // Optionally specified if a one or multiple Racks are target of the request
  repeated RackId racks = 4;
}

message MaintenanceActivityConfig {
  oneof activity {
    FirmwareUpgradeActivity firmware_upgrade = 1;
    ConfigureNmxClusterActivity configure_nmx_cluster = 2;
    PowerSequenceActivity power_sequence = 3;
    NvosUpdateActivity nvos_update = 4;
  }
}

message FindMultiNodeMaintenanceOperationsByIdsResponse {
  repeated MultiNodeMaintenanceStatus status;
}

message MultiNodeMaintenanceStatus {
  common.UUID id = 1;
  // Current state of the operation
  LifecycleStatus status = 2;
  // Results. These will be available as soon as parts of the operations finish
  repeated MultiNodeMaintenanceResult results = 3;
}

// Additional APIs not yet specified here
// - Cancel update requests that are not yet in progress
// - List IDs of all scheduled update requests
```

### What is a "safeguarded state"?

A safeguarded state is a state of the node where there exist guarantees that
- the node is not used by any NICo tenant. For compute trays the safeguarded state is usually a state where the scout image is booted. In such a state, the tenant has no ownership of the tray.
- the node is not affected by any concurrent maintenance operations

## MNME implementation

MNME is a new independent sub-component within NICo-core. It is scheduled independently as a periodic task, just like SiteExplorer, NVLink manager and other components.

In each iteration of the periodic task, MNME will perform the following steps:
1. Identify executable maintenance operations and start them via the following steps:
   1. Query the list of requested maintenance operations. These will need to get stored in the NICo database after receiving a `ScheduleMultiNodeMaintenance` call.
   2. For each planned maintenance operation, determine the full set of impacted nodes. This contains the directly impacted nodes, as well as indirectly impacted nodes. Indirectly impacted nodes can be determined based on the activity type and various links between entities, e.g.
     - Pure compute tray firmware updates do not directly have side effects on other nodes -> The list of indirectly impacted nodes is empty
     - Switch operations will impact all nodes that share the same NVLink domain
   3. If the set of impacted nodes does not intersect with any update that is already in progress, advance the update task from "Scheduled" to "In Progress". The intersection could be determined in 2 ways:
      1. Directly load information about all in-progress updates and perform intersection operation.
      2. Check health-alerts on affected nodes (`MultiNodeUpdateInProgress`). This requires the start of the update to place the health alert atomatically on all affected nodes.
2. For all "In Progress" updates, perform the following steps:
   1. Initiate triggers that let the nodes to start moving all impacted nodes (direct and indirect impact) into a guard state. E.g. for Compute Trays, set a flag that make the nodes boot scout on the next restart.
   2. Wait for these nodes to reach the guard state.
   3. Apply the actual update. This can e.g. happen via calling RMS APIs, and waiting for the results of the update.
   4. After update completion, set a flag that releases the nodes from the guard state (in their individual state machines)
   5. Wait for all nodes to finish exiting the guard state

The steps within 2) are equivalent to the steps executed by the Rack state machine for on-demand maintenance.

### Implementation options

As described in the previous chapter, MNME contains 2 major components: A set of deployment-wide operations which identify newly executable maintenance operations, followed by a the execution of all in-progress options, which can get parallelized.

These 2 steps can be implemented in the following fashions:
- A single task performs both of step 1) and 2). Step 2) could be parallelized via some fork/join mechanism (using `tokio::spawn` or `tokio::task::JoinSet`). This model would be equivalent to components like NVLink Monitor.
- The periodic MNME main task only performs step 1). In order to schedule updates, it creates `MultiNodeMaintenance` objects, where each object is lifecycle is managed by the existing state controller framework. This model would allow to reuse code for concurrent state management on a set of maintenance objects.

## Supported maintenance activities

MNME should ideally support all kinds of maintenance activities that are possible on NICo managed components. This includes various kinds of software updates (both in-band and out-of-band), as well as performing various kinds of configuration deployment that could lead to service disruptions.

DPU updates and maintenance could be supported by the same framework - but further investigation is required to determine the complexity of extracting these from the ManagedHost state machines.

## Out of scope

MNMEs primary scope is to identify which maintenence operations are safe to execute at any point in time and to start them. It enforces synchronization with individual node state machines before triggering the actual update.

It is thereby more of a building block and not a fully featured fleet management system

- MNME will not decide which updates to install at any point in time. This is left to external callers of MNME APIs.
- MNME does not have any capability to schedule maintenance further in the future ("next monday, 2 am").
- MNME does not not have any capability to determine whether execution of an update would violate minimum fleet health SLAs. Any update scheduling according to fleet health would need to be initiated by an external system.

The MNME APIs are expected to be used by an external maintenance management system according to the fleet health requirements of the affected deployment.