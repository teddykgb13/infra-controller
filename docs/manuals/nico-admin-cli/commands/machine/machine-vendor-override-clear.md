# `nico-admin-cli machine vendor-override clear`

_[Hardware commands](../../hardware.md) › [machine](./machine.md) › [vendor-override](./machine-vendor-override.md) › **clear**_

## NAME

nico-admin-cli-machine-vendor-override-clear - Clear the Redfish BMC
vendor override for a machine

## SYNOPSIS

**nico-admin-cli machine vendor-override clear** \[**--extended**\]
\[**--sort-by**\] \[**-h**\|**--help**\] \<*MACHINE*\>

## DESCRIPTION

Clear the Redfish BMC vendor override for a machine

## OPTIONS

**--extended**  
Extended result output.

This used by measured boot, where basic output contains just what you
probably care about, and "extended" output also dumps out all the
internal UUIDs that are used to associate instances.

**--sort-by** *\<SORT_BY\>* \[default: primary-id\]  
Sort output by specified field\

\
*Possible values:*

- primary-id: Sort by the primary id

- state: Sort by state

**-h**, **--help**  
Print help (see a summary with -h)

\<*MACHINE*\>  
The machine whose BMC vendor override should be cleared

## Examples

```sh
nico-admin-cli machine vendor-override clear 12345678-1234-5678-90ab-cdef01234567
```

---

**See also:** [Hardware commands](../../hardware.md) · [CLI reference index](../../README.md)
