# METIS-CORE compatibility policy

METIS-CORE is a pre-1.0 shared foundation. Compatibility is still deliberate:
downstream graph-partitioning behavior must not drift silently.

## Protected contract

The protected surface includes:

- public exports from `src/lib.rs`;
- validated `CsrGraph`, `Partition`, `MetisParams`, and `MetisPartitioner`
  construction and behavior;
- METIS-style entry points and deterministic seed behavior;
- `PartitionError` meanings used by downstream error handling; and
- structural invariants for coverage, part ranges, occupancy, balance inputs,
  and optional contiguity.

## Versioning rules

- Additive API changes may remain within the current `0.y` line when defaults
  and existing results remain compatible.
- Breaking signatures, defaults, validation behavior, error meanings, or
  deterministic outputs require a minor-version bump while the crate is below
  `1.0`.
- Prefer deprecation plus a migration note before removing a public item.
- Algorithm-quality changes must retain structural invariants and record any
  intentional cut/balance envelope change.
- Downstream repositories should pin a commit for reproducible evidence.
  Branch consumers must run the downstream rehearsal before updating.

## Foundation tests

```powershell
cargo test --test graph_ops
cargo test --test contracts
```

These protect validated CSR construction, public parameter defaults, algorithm
contracts, structural partition invariants, and deterministic seeded output.

## Downstream breakage rehearsal

ROUTE is the required first consumer rehearsal because `route-network` builds
real service-graph regions through METIS-CORE's public graph, parameter,
partitioner, trait, result, and error surfaces.

From the ROUTE repository:

```powershell
python tools/repo_map.py write-cargo-config
cargo test -p route-network metis_partitions_service_graph_fixture
cargo test -p route-network metis_partitions_dual_route_graph_fixture
```

The generated Cargo config patches ROUTE to the sibling METIS-CORE checkout.
A compile failure exposes API breakage; an assertion or error exposes behavior
drift. Remove or regenerate the ignored local config when switching dependency
layouts.

METIS-CORE foundation changes are not ready until both its contract tests and
the ROUTE rehearsal pass.
