# `nico-admin-cli machine vendor-override`

_[Hardware commands](../../hardware.md) › [machine](./machine.md) › **vendor-override**_

## NAME

nico-admin-cli-machine-vendor-override - Pin or clear the Redfish BMC
vendor override for a machine

## SYNOPSIS

**nico-admin-cli machine vendor-override** \[**--extended**\]
\[**--sort-by**\] \[**-h**\|**--help**\] \<*subcommands*\>

## DESCRIPTION

Pin or clear the Redfish BMC vendor override for a machine

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

## Subcommands

| Subcommand | Description |
|---|---|
| [`set`](./machine-vendor-override-set.md) | Pin the Redfish BMC vendor for a machine |
| [`clear`](./machine-vendor-override-clear.md) | Clear the Redfish BMC vendor override for a machine |
| [`show`](./machine-vendor-override-show.md) | Show the Redfish BMC vendor override for a machine |

---

**See also:** [Hardware commands](../../hardware.md) · [CLI reference index](../../README.md)
