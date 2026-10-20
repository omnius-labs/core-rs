## 1. About this review

This is a diff review of the rewrite of the RocketPack compiler and the accompanying migration of the omnikit wire messages to generated code.

### 1.1 Division of responsibility between documents

| Document | What it covers |
| --- | --- |
| This document | What range was examined from which viewpoints on 2026-08-10, and what was decided |
| [DESIGN.md](../DESIGN.md) | Why it has this shape: invariants, rationale for adoption, and open questions |
| [ISSUES.md](../ISSUES.md) | Clear defects confirmed by reading the code, until they are fixed |

### 1.2 Review date and target

- Review date: 2026-08-10
- Review type: diff review
- Target commits: base `95dd9438d39654ac064f27301bd220579e3a626a` (origin/main), head `54cbfdb05d559ba35590bc6b9f6af86d3bfb28fd` (feat/rocketpack-compiler-rewrite)
- Reviewers: Claude (z-ai/glm-5.2) and Claude (claude-opus-5). The former drafted the findings, and the latter re-verified all findings before they were issued

## 2. Scope and viewpoints

### 2.1 What was examined

This is a diff review, and the files changed between base and head were read as actual files.
The main targets were the compiler side under `entrypoints/rocketpack-compiler/src/` (`parser.rs`, `parser/lexer.rs`, `semantic.rs`, `codegen/rust.rs`, `config.rs`, `error.rs`, `main.rs`), the runtime codec under `modules/rocketpack/src/` (`rocket_pack_encoder.rs`, `rocket_pack_decoder.rs`, `rocket_pack_struct.rs`, `primitive/timestamp.rs`), the omnikit migration under `modules/omnikit/src/` (`generated/`, `model/`, `service/connection/secure/auth.rs`, `service/remoting/`), and `modules/migration/src/`.
The generated `__rpf_*.rs` files were read as spot checks, and compilation and tests were run.
In the re-verification, a minimal `.rpf` that reproduces each finding was written in a scratch area, code was generated with the real binary, and the generated output was run through `cargo build` in a crate that depends on `omnius-core-rocketpack`.
For the codec findings, wire byte sequences were assembled and actually decoded in an executable program that depends on omnikit.

### 2.2 What was not examined

- Files outside the diff. `modules/base`, `omnius-core-cloud`, `omnius-core-image`, `omnius-core-yamux`, and `omnius-core-testkit` were not changed this time and were not read because they are out of scope.
- Every line of the five generated `__rpf_*.rs` files. Spot checks of the symmetry of encoding and decoding and of the constraints were done, but the whole generated output was not compared exhaustively, because the time was needed for other viewpoints.
- Test code coverage. The tests were run and confirmed to pass, but whether a corresponding test exists for each finding was not examined, because it was not included in the viewpoints.
- The accuracy of the contents of the design documents `docs/DESIGN.md` and `docs/design/rocketpack-compiler.md`. The headings and the deferred chapters were read to confirm the structure of the place where findings are transcribed, but the descriptions were not checked against the code. This is handled in a separate review.

### 2.3 Viewpoints

- Correctness of generated code: whether a type accepted as a schema becomes Rust that compiles in codegen
- Input checking of the codec: the decoder's boundaries and overflow protection against hostile or malformed wire input
- Wire format compatibility: whether the external contracts (wire format, persisted format, signature preimage) are unchanged in the migration from hand-coded to generated code
- Input validation of the compiler: whether out-of-range schema values (tags, array lengths) are rejected
- Runtime efficiency: the number of traversals on the encode path of the generated code
- Maintainability: duplication of type definitions between codegen and semantic

## 3. Overall assessment

The newly built compiler part has defects where a schema is accepted but the generated code cannot compile (R-1, R-2, R-3), a defect that always fails at runtime (R-4), and defects that silently generate a different value (R-5, R-6).
In every case, it cannot be merged until it is settled either by rejecting it up front in the parser and semantic stage or by handling it correctly in codegen.
The breaking changes to the wire format (R-11, R-12) read as an intentional migration, but there is no compatibility or migration path and it is fail-closed, so a decision is needed between keeping backward compatibility and allowing a flag day.

## 4. Findings

| ID | Summary | Type | Confidence | Handling |
| --- | --- | --- | --- | --- |
| [R-1](#r-1) | Placing Option inside Vec or Map makes the generated code fail to compile with a type mismatch | Defect | High | Filed in ISSUES.md as I-1 |
| [R-2](#r-2) | A field named count or decoder collides with a local variable and the generated code fails to compile | Defect | High | Filed in ISSUES.md as I-2 |
| [R-3](#r-3) | sanitize_ident does not check collisions and generates duplicate members for type and type_ | Defect | High | Filed in ISSUES.md as I-3 |
| [R-4](#r-4) | u128 and i128 generate stubs that always fail at runtime | Defect | High | Filed in ISSUES.md as I-4 |
| [R-5](#r-5) | Tags and array lengths are silently truncated from u128 | Defect | High | Filed in ISSUES.md as I-5 |
| [R-6](#r-6) | The default value of an Option field is not reflected in the generated code | Defect | High | Filed in ISSUES.md as I-6 |
| [R-7](#r-7) | Decoding a signed positive number wraps around a value beyond the iN range and stores it | Defect | High | Filed in ISSUES.md as I-7 |
| [R-8](#r-8) | Vec::with_capacity is called with a count from the wire, amplifying memory use | Defect | High | Filed in ISSUES.md as I-8 |
| [R-9](#r-9) | skip_field treats info 28 as 16 bytes, inconsistent with read_raw_len | Defect | High | Filed in ISSUES.md as I-9 |
| [R-10](#r-10) | The example config still declares base_dir: rpfs after rpfs was deleted, so it cannot compile | Defect | High | Filed in ISSUES.md as I-10 |
| [R-11](#r-11) | The wire format changed from 0-based tags to 1-based with no compatibility path | Design issue | High | Filed in DESIGN.md §11.2 Deferred as "Backward compatibility of the wire format" |
| [R-12](#r-12) | The preimage of the signature hash was changed to the whole wire export | Design issue | High | Filed in DESIGN.md §11.2 Deferred as "Stability of the signature hash preimage" |
| [R-13](#r-13) | pack traverses the value tree one more pass for validate | Improvement | High | Filed in ISSUES.md as I-11 (severity low) |
| [R-14](#r-14) | codegen redeclares the whole type system of semantic | Improvement | High | Filed in ISSUES.md as I-12 (severity low) |
| [R-15](#r-15) | wip_tests remains as a parallel test harness that bypasses the real path | Improvement | High | Filed in ISSUES.md as I-13 (severity low) |

R-1 to R-6 share a common origin: there is a gap between the range that the parser and semantic accept and the range that codegen can handle correctly.
They break in three ways.
R-1 to R-3 produce output that does not pass compilation, R-4 passes compilation and always fails at runtime, and R-5 and R-6 both compile and run successfully but behave differently from the schema that was written.
The last two are harder to notice because they do not fail.
R-7 to R-9 are input checks in the runtime codec, and stem from an asymmetry in which the positive side has no guard compared with the negative side, and from a mismatch in how headers are interpreted when skipping unknown fields.
R-11 and R-12 come from the same migration, and are incompatible with the old format in both the wire format and the signature preimage.

<a id="r-1"></a>
### R-1. Placing Option as an element of Vec or Map makes the generated code fail to compile with a type mismatch

**Type: defect** / **Confidence: high**

**Location**
[`codegen/rust.rs#L672`](../../entrypoints/rocketpack-compiler/src/codegen/rust.rs#L672), [`semantic.rs#L358`](../../entrypoints/rocketpack-compiler/src/semantic.rs#L358)

**What the problem is**
`write_encode_value` and `write_decode_value` transparently strip `ResolvedType::Option(inner)` and delegate to the encoding and decoding of the inner type.
So when `Vec<Option<u32>>`, `Map<String, Option<u32>>`, `Option<Option<u32>>`, or `[Option<u32>; N]` is written in a schema, the decode side pushes a `u32` into a `Vec<Option<u32>>` and the encode side passes an `Option<u32>` to `write_u32`, causing a type mismatch on both sides.

**Evidence**
We read that `resolve_type_inner` in `semantic.rs` recursively resolves the inside of `Type::Option` and does not reject an Option inside a container.
We read that the `Option` branch of codegen recurses inward and calls the encoding of the element, and confirmed by compiling that `Vec<Option<u32>>` produces E0308 with this combination.

**Impact**
A schema that uses such a type fails when building the generated code.
It has not surfaced yet because the current omnikit schemas have no such type.

**Handling**
Filed in ISSUES.md as I-1.

<a id="r-2"></a>
### R-2. A field named count or decoder collides with a local variable and the generated code fails to compile

**Type: defect** / **Confidence: high**

**Location**
[`codegen/rust.rs#L718`](../../entrypoints/rocketpack-compiler/src/codegen/rust.rs#L718)

**What the problem is**
The struct unpack generates `let count = decoder.read_map()?;`, and the enum unpack generates `let mut result: Option<Self> = None;`.
If a struct uses `count` or `decoder` as a field name, or an enum record variant uses `result` as a field name, it shadows the local variable of the same name, and an assignment inside the match becomes a type error.

**Evidence**
We generated and compiled a struct with `@1 count: u32;` and confirmed that E0308 and E0599 on the immutable `count` appear.
We generated an enum record variant containing `result` and confirmed that `result = Some(Self::A { .. })` becomes an assignment to a local variable of type `Option<u32>` and produces E0308.
We read that the parser does not reject these field names.

`count` in an enum record variant does not collide.
The outer `for _ in 0..count` evaluates the range first, so shadowing `count` inside the arm of a variant has no effect.
We confirmed this by generating code and seeing it compile.

**Impact**
A schema with a field such as `@1 count: u32;` fails when building the generated code.
It has not surfaced yet because the current omnikit schemas have no such field.

**Handling**
Filed in ISSUES.md as I-2.

<a id="r-3"></a>
### R-3. sanitize_ident does not check collisions and generates duplicate members for type and type_

**Type: defect** / **Confidence: high**

**Location**
[`codegen/rust.rs#L1903`](../../entrypoints/rocketpack-compiler/src/codegen/rust.rs#L1903)

**What the problem is**
`sanitize_ident` only appends `_` to Rust keywords.
`type` becomes `type_`, which is the same identifier as an existing `type_`.
The symbol and package layers have collision checks, but the field and variant layer does not, so duplicate members are generated and compilation fails.

**Evidence**
We read that the implementation of `sanitize_ident` checks only `is_rust_keyword` and no collisions, and that `render_module_tree` checks only symbols and packages.
We confirmed that placing `type` and `type_` in the same struct makes both `type_`.

**Impact**
A schema that places a pair of names such as `type` and `type_` in the same struct or enum fails when building the generated code.
It has not surfaced yet because the current omnikit schemas have no such pair.

**Handling**
Filed in ISSUES.md as I-3.

<a id="r-4"></a>
### R-4. u128 and i128 generate stubs that always fail at runtime

**Type: defect** / **Confidence: high**

**Location**
[`codegen/rust.rs#L655`](../../entrypoints/rocketpack-compiler/src/codegen/rust.rs#L655), [`semantic.rs#L569`](../../entrypoints/rocketpack-compiler/src/semantic.rs#L569)

**What the problem is**
semantic accepts `u128` and `i128` as canonical builtins, but codegen generates the stubs `return Err(IoError(Unsupported, "u128 encode is not supported"))` and `Other("u128 decode is not supported")`.
`@1 id: u128;` compiles successfully and always fails at runtime.

**Evidence**
We read that `builtin_type` in semantic returns `"u128" => Some(BuiltinType::U128)` and that the `BuiltinType::U128` branch of codegen is a stub that returns an error.

**Impact**
All encoding and decoding of fields that use u128 or i128 fail at runtime.
It has not been settled between rejecting at compile time and implementing a fixed-width form.

**Handling**
Filed in ISSUES.md as I-4.

<a id="r-5"></a>
### R-5. Tags and array lengths are silently truncated from u128

**Type: defect** / **Confidence: high**

**Location**
[`parser.rs#L625`](../../entrypoints/rocketpack-compiler/src/parser.rs#L625)

**What the problem is**
`expect_int_u32`, `expect_int_u32_spanned`, and `expect_int_u64` silently truncate the `n: u128` of `Token::Int(n)` with `n as u32` and `n as u64`.
`@4294967296` is generated as tag 0, and `[u32; 18446744073709551617]` is generated as `[u32; 1]`.
Neither produces an error, and code with a wire format different from the schema that was written is emitted.

**Evidence**
We read that `expect_int_u32_spanned` casts with `Spanned::new(n as u32, ...)` and that there is no check for out-of-range values.
We generated a struct that has only `@4294967296` and confirmed that `encoder.write_u64(0)` and `0 => {` appear.
We generated `[u32; 18446744073709551617]` and confirmed that `pub a: [u32; 1]` and `if __count_0 != 1` appear and that it compiles as well.

If the truncated result collides with an existing tag, semantic catches it.
We confirmed that placing `@0` and `@4294967296` side by side is rejected with `duplicate field tag @0 in S`.
Therefore a duplicate match arm is not generated, and this path does not become an unreachable pattern.
What is missing is only the truncation check by itself, and unless it collides, it passes all the way through.

**Impact**
Tags of `2^32` or more and array lengths of `2^64` or more become different values without an error.
The generated encoding and decoding are self-consistent, so the discrepancy cannot be noticed until they are compared against another implementation or another language's generated output that shares the schema.

**Handling**
Filed in ISSUES.md as I-5.

<a id="r-6"></a>
### R-6. The default value of an Option field is not reflected in the generated code

**Type: defect** / **Confidence: high**

**Location**
[`codegen/rust.rs#L774`](../../entrypoints/rocketpack-compiler/src/codegen/rust.rs#L774)

**What the problem is**
`render_value_init` returns early when resolved is `Option(_)` and completely ignores the field's default value.
With `@1 retry_count: Option<u32> = 3;`, when tag 1 is missing it returns `None` instead of `Some(3)`.
The default of a required field takes effect through `unwrap_or`, but not for an Option field, which is asymmetric.

**Evidence**
We read that the beginning of `render_value_init` returns early with `if matches!(resolved, ResolvedType::Option(_)) { return Ok(value_ident.to_string()); }` without looking at the default argument, and that there is a path of `unwrap_or(default)` after that.
We read that semantic accepts the combination of an Option field and a default value.

**Impact**
Even if a default value is written for an Option field, it is not filled in when it is missing on the wire.
A schema that intended a backward-compatible default does not work.

**Handling**
Filed in ISSUES.md as I-6.

<a id="r-7"></a>
### R-7. Decoding a signed positive number wraps around a value beyond the iN range and stores it

**Type: defect** / **Confidence: high**

**Location**
[`rocket_pack_decoder.rs#L295`](../../modules/rocketpack/src/rocket_pack_decoder.rs#L295)

**What the problem is**
The positive branch of `read_i64`, `(0, 27) => return Ok(u64::from_be_bytes(...) as i64)`, is an unchecked `u64 as i64` cast.
If `2^63` is sent on the wire, `read_i64` wraps around to a negative number without an error and stores it in the struct.
`read_i8`, `read_i16`, and `read_i32` have the same form.
The negative side checks the range with the top bit, but the positive side has no guard.

**Evidence**
We read that the positive branch of `read_i64` casts with `as i64` and does not check the range, and that the negative branch checks with `current_raw_byte() & 0x80`.
We confirmed that the encoder does not write a positive number in this range, but that it can be reproduced with an external or hostile CBOR producer.

**Impact**
An arbitrary negative number can be put into an i64 field with hostile input.
This is data corruption, and it has not surfaced because no path for omnikit to receive CBOR from outside is currently wired.

**Handling**
Filed in ISSUES.md as I-7.

<a id="r-8"></a>
### R-8. Vec::with_capacity is called with a count from the wire, amplifying memory use

**Type: defect** / **Confidence: high**

**Location**
[`codegen/rust.rs#L886`](../../entrypoints/rocketpack-compiler/src/codegen/rust.rs#L886)

**What the problem is**
The generated decode for an unconstrained `Vec<T>` uses the count from the wire as it is in `Vec::with_capacity(count as usize)`.
The remaining-bytes check of `read_array` assumes at least 1 byte per element and is slipped through in a range that allows allocating about 24 times the input size at once.

**Evidence**
We read the generated `Vec::with_capacity(count as usize)` and the remaining-bytes check of `read_array`, and confirmed that for a `Vec<String>` with empty strings of 1 byte per element lined up, it allocates about 24 times the input.

**Impact**
A hostile buffer of 10 MiB can make it allocate about 240 MiB.
It has not surfaced because no external input path is wired.

**Handling**
Filed in ISSUES.md as I-8.

<a id="r-9"></a>
### R-9. skip_field treats info 28 as 16 bytes, inconsistent with read_raw_len

**Type: defect** / **Confidence: high**

**Location**
[`rocket_pack_decoder.rs#L490`](../../modules/rocketpack/src/rocket_pack_decoder.rs#L490)

**What the problem is**
`skip_field` treats additional info 28 of major 0 and 1 as a 16-byte payload `Some(16)`.
`read_raw_len` treats info 28 and above as `None`, and `read_u64` returns `Mismatch`, so the three interpretations do not agree.
If `0x1C` is placed in the value of an unknown tag, only `skip_field` skips 16 bytes and the stream position shifts.

**Evidence**
We read `28 => Some(16)` in `skip_field` and the `None` handling of info 28 and above in `read_raw_len`, and confirmed that the interpretations do not agree.
The encoder does not generate info 28, so it fires only with hostile input.

**Impact**
Even for a valid buffer, placing an invalid header in the value of an unknown tag can remotely cause a decode failure.
It has not surfaced because no external input path is wired.

**Handling**
Filed in ISSUES.md as I-9.

<a id="r-10"></a>
### R-10. The example config still declares base_dir: rpfs after rpfs was deleted, so it cannot compile

**Type: defect** / **Confidence: high**

**Location**
[`data/rocketpack.yaml#L5`](../../entrypoints/rocketpack-compiler/data/rocketpack.yaml#L5) (as of head)

**What the problem is**
`data/rocketpack.yaml` at head declares `base_dir: rpfs`, but the `rpfs` directory has been deleted.
`cargo run -- compile ./data` fails with `source base_dir is not a directory`.
The fix is only the deletion of the yaml in the working tree, and it is uncommitted.

**Evidence**
We confirmed that at head `data/rocketpack.yaml` has `base_dir: rpfs` and that the `rpfs` directory does not exist.
We confirmed with `git status` that `data/rocketpack.yaml` is deleted in the working tree.

**Impact**
A person who newly clones this commit fails at `compile ./data`.
The deletion of the example config and the deletion of rpfs are not bundled into the same commit, so the intermediate state is broken.

**Handling**
Filed in ISSUES.md as I-10.

<a id="r-11"></a>
### R-11. The wire format changed from 0-based tags to 1-based with no compatibility path

**Type: design issue** / **Confidence: high**

**Location**
[`__rpf_omni_secure_77c8b91862421dd1.rs#L173`](../../modules/omnikit/src/generated/omni_secure/__rpf_omni_secure_77c8b91862421dd1.rs#L173)

**What the problem is**
All omnikit wire messages were replaced wholesale, from the old hand-coded 0-based tags and enums encoded as u32 or strings, to the new generated code's 1-based tags and enum-as-map.
`session_id` was tightened to `bytes[32..=32]`.
There is no compatible decode path or version negotiation, so old nodes and data persisted in the old format become undecodable.

**Evidence**
We read that the decoding of the generated ProfileMessage uses 1-based tags and `read_bytes_bounded(32, 32)`, and that the deleted `message.rs` used 0-based tags and a u32 `auth_type`.
We passed the byte sequence of a ProfileMessage encoded in the old format through the new `unpack` and confirmed that it fails with `MismatchFieldType { position: 37, field_type: U8 }`.
Because the tags are shifted by one, it fails trying to read the `auth_type` of old tag 1 (a u32 of major 0) as the `session_id` of new tag 1 (bytes of major 2).
It fails with a type mismatch and not with a missing field, so it cannot be absorbed by default values or by filling in missing fields.
Whether to have a compatibility path is a design decision, and from the code it reads as an intentional change, but there is no description of the migration policy.

**Impact**
When coexistence with rolling deployment or with data stored in the old format is needed, it becomes undecodable in a fail-closed manner.
Within this repo, the tests pass because it is self-consistent.

**Handling**
Filed in DESIGN.md §11.2 Deferred as "Backward compatibility of the wire format".

<a id="r-12"></a>
### R-12. The preimage of the signature hash was changed to the whole wire export

**Type: design issue** / **Confidence: high**

**Location**
[`auth.rs#L179`](../../modules/omnikit/src/service/connection/secure/auth.rs#L179)

**What the problem is**
The signing target of `gen_hash` changed from the old "explicit per-field hash" to "all bytes of the wire `export()` of ProfileMessage and OmniAgreementPublicKey".
Signature verification now depends on a bit-exact match of the CBOR serialization.
It will also silently become a signature mismatch when fields are added in the future.

**Evidence**
We read that `gen_hash` calls `hasher.update(profile_message.export()?)` and `hasher.update(agreement_public_key.export()?)`, and that the old code explicitly hashed the scalar of each field.
We confirmed that it is self-consistent within the same build, but that signatures from old builds and signatures persisted in the old format are structurally unverifiable.

**Impact**
Signature verification depends on the stability of the serialization.
Whether there is a compatibility or migration path is a design decision.

**Handling**
Filed in DESIGN.md §11.2 Deferred as "Stability of the signature hash preimage".

<a id="r-13"></a>
### R-13. pack traverses the value tree one more pass for validate

**Type: improvement** / **Confidence: high**

**Location**
[`codegen/rust.rs#L489`](../../entrypoints/rocketpack-compiler/src/codegen/rust.rs#L489)

**What the problem is**
The generated `pack` calls `Self::validate(value)?` at the beginning.
For Named-type fields, `has_length_constraints` is always considered true, so the whole subtree is validated even when there are no constraints.
Nested structures are revalidated at each node, and encoding takes at least 2 passes.

**Evidence**
We read that the beginning of `pack` calls `Self::validate(value)?` and that `has_length_constraints` returns true for `Named` and Timestamp.

**Impact**
The encoding cost grows to the product of O(n) and the depth.
Nothing is broken.

**Handling**
Filed in ISSUES.md as I-11 (severity low).

<a id="r-14"></a>
### R-14. codegen redeclares the whole type system of semantic

**Type: improvement** / **Confidence: high**

**Location**
[`codegen/rust.rs#L64`](../../entrypoints/rocketpack-compiler/src/codegen/rust.rs#L64), [`codegen/rust.rs#L971`](../../entrypoints/rocketpack-compiler/src/codegen/rust.rs#L971), [`codegen/rust.rs#L1598`](../../entrypoints/rocketpack-compiler/src/codegen/rust.rs#L1598)

**What the problem is**
codegen redeclares `BuiltinType` and `ResolvedType` from `semantic.rs` and maps them with `convert_builtin` and `convert_resolved_type`.
The string table from builtins to Rust types is duplicated in two places, `render_resolved_type` and `render_semantic_builtin`.
Adding a new builtin requires editing six places at the same time.

**Evidence**
We read that `BuiltinType` and `ResolvedType` of codegen have the same elements as semantic, and that the two render functions each have the same mapping from builtins to Rust types separately.

**Impact**
If the two tables diverge in the future, the generated types silently disagree.
Nothing is broken.

**Handling**
Filed in ISSUES.md as I-12 (severity low).

<a id="r-15"></a>
### R-15. wip_tests remains as a parallel test harness that bypasses the real path

**Type: improvement** / **Confidence: high**

**Location**
[`codegen/rust.rs#L2122`](../../entrypoints/rocketpack-compiler/src/codegen/rust.rs#L2122)

**What the problem is**
`mod wip_tests` has its own test harness, constructs `RustOptions` directly, and calls `render_rust_file`.
So it bypasses the options validation of `validate_options` and `validate_required_mappings`, and the atomic replacement and backup of `publish_generated_files`.
It exists in parallel even though the neighboring `mod tests` drives `generate()` end to end.

**Evidence**
We read that `mod wip_tests` has its own `ParsedSource` and `GeneratedRustFile`, that `render_sources` constructs `RustOptions` directly and does not go through `publish_generated_files`, and that `mod tests` goes through `generate()`.
We read that only the `generate()` side calls `validate_options`.

It does not bypass the semantic validation.
`render_sources` actually goes through `ManifestGraph::load` and `SemanticGraph::build`, and these two are the same as the real path.

**Impact**
A gap remains in the detection of regressions in the options validation and the publish path.
Nothing is broken.

**Handling**
Filed in ISSUES.md as I-13 (severity low).

## 5. What was checked and found to be fine

- Whether the decoder's slice boundaries and `pos + len` overflow protection are sound. `read_raw_bytes` and `read_raw_fixed_bytes` check `remaining() < len`, and the remaining-length computation of `skip_field` uses `checked_add` and `checked_mul`, so we judged that there are no panics from lengths that come from the wire.
- The symmetry of encoding and decoding in the generated code. Both directions are generated from the same schema in tag order, and unknown tags are skipped with `_ => skip_field()`, so we judged that there is no mismatch of field order and no orphaned type id.
- The deletion of `EmptyRocketMessage` and `rocketpack::prelude`. We confirmed with grep that no references remain, and judged that there are no external callers.
- The deletion of `strum` and `enumflags2`. We confirmed with grep that no uses remain.
- The `&mut self` change in migration. We confirmed that there are no external callers in the workspace and that the tests are updated.
- Build and tests of the whole workspace. We ran them and confirmed that compiler 34, rocketpack 25, and omnikit 6 all pass. omnikit also has 3 other ignored tests, which were not run.

## 6. Afterward

- 2026-08-10: Issued. Filed R-1 to R-10 and R-13 to R-15 in ISSUES.md as I-1 to I-13. Filed R-11 and R-12 in the DESIGN.md §11.2 Deferred section.
- 2026-08-11: Fixed R-10. Deleted `entrypoints/rocketpack-compiler/data/rocketpack.yaml` and resolved the state where the example config pointed to the removed `rpfs`. I-10 in ISSUES.md has been deleted. The three projects in `entrypoints/rocketpack-compiled-example/` take over as the compiler's examples.
- 2026-08-12: Fixed R-1 to R-9, R-13, and R-15, and deleted the corresponding ISSUES.md items. R-14 remains as the unresolved I-12. R-11 was decided as a flag-day migration and R-12 as an explicit semantic preimage, and both were recorded in the decided section of DESIGN.md.
