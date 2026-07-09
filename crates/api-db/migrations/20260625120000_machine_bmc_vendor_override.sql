-- Add bmc_vendor_override to machines so an operator can pin the Redfish BMC
-- vendor for a machine. NULL means automatic detection. The value is a
-- RedfishVendor variant name passed down into libredfish as the forced vendor.
ALTER TABLE machines ADD COLUMN bmc_vendor_override text;
