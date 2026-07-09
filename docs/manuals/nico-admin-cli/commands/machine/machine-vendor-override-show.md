# `nico-admin-cli machine vendor-override show`

_[Hardware commands](../../hardware.md) › [machine](./machine.md) › [vendor-override](./machine-vendor-override.md) › **show**_

## NAME

nico-admin-cli-machine-vendor-override-show - Show the Redfish BMC
vendor override for a machine

## SYNOPSIS

**nico-admin-cli machine vendor-override show** \[**--extended**\]
\[**--sort-by**\] \[**-h**\|**--help**\] \<*MACHINE*\>

## DESCRIPTION

Show the Redfish BMC vendor override for a machine

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
The machine whose BMC vendor override should be shown

## Examples

```sh
nico-admin-cli machine vendor-override show 12345678-1234-5678-90ab-cdef01234567
```

---

**See also:** [Hardware commands](../../hardware.md) · [CLI reference index](../../README.md)
