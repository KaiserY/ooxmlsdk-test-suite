# olecfsdk-test

Focused public-API and coverage contracts for `olecfsdk`. This crate owns the
small, reviewable tests and the unified coverage report; deep generated corpus
lanes remain in `olecfsdk-roundtrip-tests`.

## Checks

```bash
cargo test -p olecfsdk-test
cargo clippy -p olecfsdk-test --all-targets -- -D warnings
cargo test -p olecfsdk-test --test coverage_ratchet -- --ignored
cargo run -p olecfsdk-test --bin olecfsdk-coverage
```

`file_root_public_api` uses real Apache POI fixtures and only public SDK APIs.
For DOC, XLS, and PPT it covers path/bytes/owned-CFB input, typed traversal,
transactional semantic mutation, strict CFB/bytes/path output, reopen, and
second-cycle logical stability. Separate compatible fixtures verify diagnostics,
Reject versus Preserve save policy, and the rule that a rejected save must not
overwrite an existing target.

## Coverage contract

Schema version 2 always contains exactly these domains: CFB, DOC, XLS, PPT,
VBA, OLE Property Set, OfficeArt, and Forms. Each domain conserves this file or
structural-unit inventory:

```text
discovered = excluded + strict + compatible + rejected
supported  = strict + compatible
supported  = round_tripped + round_trip_failures
```

Every exclusion, rejection, and round-trip failure is accounted for by a
machine-readable reason. Missing domains and reasonless counts invalidate the
report before ratchet comparison.

Ratchet directions are intentional:

- discovered, strict, supported, round-tripped, and positive domain metrics
  are floors;
- excluded, rejected, unknown/partial/opaque compatibility debt, and other
  domain debt metrics are ceilings.

OfficeArt is audited as an independent structural inventory across its DOC,
XLS, and PPT hosts. Its record, byte, complete/partial, opaque, incomplete, and
context-dependent counters are not inferred from file-root success.

OLE Property Set coverage is property-exhaustive rather than stream-only. Each
dictionary or typed property packet is classified independently, including
nested vector/array variants, and both unit and bounded-packet byte totals must
equal their disposition totals. The ratchet currently locks 2,760 parsed
streams, 3,255 property sets, 28,330 property packets (6,469,898 bytes), 264
dictionaries with 686 entries, and all 13 property types observed in the
corpus. Compatibility, malformed, specification-opaque, temporary-untyped, and
unknown-extension property units and bytes all have explicit zero ceilings.

VBA coverage separates version-independent structure from deliberately opaque
caches. It locks 113 projects, 541 modules, 8,926 `dir` records, 2,474
`PROJECT` records, and 541 compressed source leaves (294,400 bytes), with all
11,400 structural records typed. The 528 SRP streams and module/project
performance caches form 1,162 implementation-cache leaves (2,278,894 bytes)
that MS-OVBA requires readers to ignore and interoperable writers to remove.
Two `PROJECTlk` streams and their three license envelopes are typed separately;
their currently empty key payloads remain specification-opaque. The mandatory
four-byte zero `dir` Reserved field is modeled and checked, not counted as an
unexplained tail.

Forms coverage recursively inventories the control tree instead of counting
only roots with a control CLSID. Twelve outermost storages with direct `f`/`o`
streams contain 18 parent storages and 60 Sites, including zero-CLSID UserForms
and nested Page/Frame/MultiPage controls without double-counting them as roots.
All 60 Sites are typed. Class-table entries, external COM persistence,
compatibility, malformed, specification-opaque, temporary-untyped, and
unknown-extension Sites retain explicit ceilings.

DOC coverage walks the file root rather than stopping at text pieces. The 403
supported roots currently contain 89,587 table/data/object/content nodes:
88,276 typed nodes, 1,189 externally owned embedded-object streams, 114
deprecated numbering-cache nodes that MS-DOC says to ignore, seven malformed
known nodes, and one unknown MsoEnvelope version. Unit dispositions must equal
`content_nodes`. Every observed typed node kind has a ratchet floor; the five
non-typed DOC families have specification, upstream, concrete-fixture evidence,
and exact unit/byte ceilings.

Disposition metrics use keys of the form
`disposition.<class>.<units|records|bytes>`. The classes are typed,
specification-opaque, external-leaf, unknown-extension, compatibility,
malformed, and temporary-untyped. XLS, PPT, and OfficeArt record inventories are
exhaustive: their disposition record totals must equal the corresponding parsed
record totals. CFB payload ownership and the current DOC/VBA/OLEPS/Forms unit
inventories use the same vocabulary without pretending unlike units can be
summed. Positive dispositions are ratchet floors; every debt disposition has a
ceiling, including explicit zero ceilings.

Per-type debt uses the `debt.*` prefix and is joined to
`coverage-evidence.json`. Every emitted debt family must have structured
specification, upstream, and corpus evidence, and every evidence family must
have an explicit ratchet ceiling. A new record type therefore cannot enter the
corpus as an unexplained generic unknown.

The current PPT evidence audit distinguishes producer compatibility from true
unknown extensions. Type `0x0000` is known producer padding/legacy framing and
type `0x200A` is an observed fixed-size handout compatibility atom. The only
remaining unassigned PPT types are `0x0080` and `0x779F`, one record each.
Legacy BIFF `Unknown` records are likewise classified as compatibility in their
host context rather than mislabeled as BIFF8 extensions. The remaining
refinement is to extend this per-family evidence inventory through every
compatibility and malformed family in the remaining DOC, VBA, Forms, and host
format inventories.
