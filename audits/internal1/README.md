# Internal audit 1 — `valory-xyz/registries-solana`

**Date:** 2026-06-02
**Auditor:** audit-claude, 77ph
**Scope baseline:** HEAD `604ba92` (2026-05-25 «Update LICENSE») on branch `main`
**Code under audit:** `programs/registries/` — 7 `.rs` files, 245 LoC
**Toolchain (per `setup-env.sh`):** rustc 1.79, Solana 2.0.8, Anchor 0.30.1
**Reference for comparison:** `valory-xyz/autonolas-registries` (EVM) — the project this is meant to port to Solana
**Methodology:** exhaustive Anchor-specific + cross-domain DeFi-pattern checklist applied to every applicable surface; adversarial sweep over what is NOT in the dev's implicit claims. Consultant audit scope — security-only.

---

## 1. Project intent

**What this is supposed to be.** A Solana port of the EVM `valory-xyz/autonolas-registries` system — specifically the **Service Registry** part. The EVM version provides the on-chain registry infrastructure for Olas (Autonolas) autonomous services: it mints a service NFT, manages a state machine across registration → activation → operator-instance registration → multisig deploy → termination → unbond, and escrows the service owner's security deposit and operators' bonds. The Solana version is meant to deliver equivalent functionality on Solana mainnet so Olas services can be deployed there too.

**Reference EVM contract: `ServiceRegistry.sol`** (`valory-xyz/autonolas-registries` HEAD). It inherits from `GenericRegistry` (drainer/admin) and `UnitRegistry` (NFT base, on top of ERC-721) and exposes 11 state-changing externals:

| # | EVM external | Purpose |
|---|---|---|
| 1 | `create(serviceOwner, configHash, agentIds, agentParams, threshold)` | mint service NFT, write Service struct |
| 2 | `update(serviceOwner, configHash, agentIds, agentParams, threshold, serviceId)` | edit service in `PreRegistration` state |
| 3 | `activateRegistration(serviceOwner, serviceId)` payable | service owner pays security deposit; state → `ActiveRegistration` |
| 4 | `registerAgents(operator, serviceId, agentInstances, agentIds)` payable | operator bonds + registers agent instances |
| 5 | `deploy(serviceOwner, serviceId, multisigImplementation, data)` | creates GnosisSafe multisig; state → `Deployed` |
| 6 | `slash(agentInstances, amounts, serviceId)` | multisig-only; slashes operator bonds |
| 7 | `terminate(serviceOwner, serviceId)` | service owner ends operations; refund of security deposit |
| 8 | `unbond(operator, serviceId)` | operator withdraws bond after termination |
| 9 | `changeDrainer(newDrainer)` | admin sets drainer address |
| 10 | `changeMultisigPermission(multisig, permission)` | admin whitelists multisig implementations |
| 11 | `drain()` | drainer collects accumulated slashed funds |

Plus ERC-721 transferability (each service is a tradable NFT identifying ownership), `getService(serviceId)`, `getAgentInstances(serviceId)`, `getOperatorBalance(operator, serviceId)` view functions, and the 6-state service state machine: `NonExistent → PreRegistration → ActiveRegistration → FinishedRegistration → Deployed → TerminatedBonded`.

The system is non-trivially large — `ServiceRegistry.sol` alone runs ~700 LoC of Solidity with supporting registries (`ComponentRegistry`, `AgentRegistry`, `ServiceRegistryTokenUtility`), 1 manager contract (`ServiceManager`), and 5+ staking-related contracts.

---

## 2. Architectural feasibility & path assessment

This section answers two questions: **(2.1) Is the EVM→Solana port theoretically possible?** and **(2.2) Are the developers on the right path?** The §1 framing tells us what's being attempted; this section asks whether the attempt is sound before §3+ measure what's done.

### 2.1 Theoretical feasibility — YES

**Every EVM primitive used by `autonolas-registries` has a viable Solana equivalent. There is no fundamental architectural blocker.** Concretely, mapping each EVM primitive against a Solana analog:

| # | EVM primitive used | Solana analog | Feasibility |
|---|---|---|---|
| 1 | ERC-721 NFT per service (tradable ownership) | Metaplex Token Metadata NFT / token-2022 NFT / PDA-owner field | ✓ Possible (choice of NFT standard is a design decision) |
| 2 | Per-service `mapping(uint256 => Service)` | Per-service PDA seeded by `service_id` | ✓ Possible (idiomatic Solana) |
| 3 | 6-state service state machine on chain | `state: u8` byte in PDA, ideally with Rust enum + `From<u8>` conversion | ✓ Possible |
| 4 | `msg.value` native ETH for `securityDeposit` | SPL Token transfer to escrow PDA (or System Program SOL transfer for native SOL) | ✓ Possible — Solana doesn't have "payable" but transfers are native syscalls |
| 5 | ERC-20 token bonds (via `ServiceRegistryTokenUtility`) | SPL Token transfer to per-operator-per-service escrow PDA | ✓ Possible |
| 6 | GnosisSafe multisig deployment via CREATE2 | Squads multisig CPI / Realms (SPL Governance) / Solana native multisig | ✓ Possible — Squads is production-ready, used by major Solana DAOs; advanced GnosisSafe features (modules, guards) may not have 1:1 mappings, but for the registry's "M-of-N signing" need, Squads is sufficient |
| 7 | `slash()` callable by service's own multisig (CPI back to registry) | CPI from Squads back to registry program; Squads can sign as multisig PDA | ✓ Possible |
| 8 | `drain()` admin-only fund collection | Drainer Pubkey in Config, drain instruction with signer check | ✓ Possible |
| 9 | NFT ownership for access control (`msg.sender == ownerOf(serviceId)`) | Anchor signer + TokenAccount (amount=1) check pattern, OR PDA-owner field if no NFT | ✓ Possible (idiomatic) |
| 10 | Upgradeable proxy pattern (`ServiceManager` wraps `ServiceRegistry`) | Anchor `upgrade_authority` for the program; different mechanism, equivalent outcome | ✓ Possible (different shape but achieves same purpose) |
| 11 | Cross-chain coordination (Olas registries on multiple EVM chains + bridges) | Wormhole / deBridge / LayerZero on Solana — all support Solana ↔ EVM messaging | ✓ Possible (requires bridge integration design) |
| 12 | `uint96` for bonds (≥1B ETH range) | `u64` (≤18 EH or ≤18T USDC) | ✓ Possible — u64 covers any realistic SPL amount; **mild range narrowing, verify with tokenomics team** |
| 13 | `uint32[]` parallel arrays (`agentIds[]` + `agentParams[]`) | Borsh `Vec<T>` OR combined struct (Solana port already does the latter — cleaner) | ✓ Possible (positive deviation) |
| 14 | Complex token-utility math (OLAS discount factors) | Pure arithmetic in Rust with `overflow-checks = true` | ✓ Possible |
| 15 | EVM gas-scaled dynamic storage growth | Solana per-account size cap (10MB hard, 10KB soft) — but per-service PDA is small (<1KB) | ✓ Possible — no aggregate cap (each service is its own PDA, not shared mapping) — **arguably better than EVM** |
| 16 | EVM reentrancy footgun | Solana runtime forbids cross-program reentrancy by construction | ✓ Better than EVM — one entire class of bugs eliminated |
| 17 | Per-call gas limit (~30M EVM, scales with usage) | Per-transaction compute-unit budget (~1.4M CU) | ✓ Possible — operations that scale with N (e.g., `slash` over N agent instances) need bounded N; for realistic registry use (N≤100), no constraint |
| 18 | Linear assembly-style ERC-20 transfers | SPL Token CPI (`anchor_spl::token::Transfer`) — imported in `lib.rs` (currently unused, see F-7) | ✓ Possible — the intent is visible in the imports |

**Conclusion (2.1):** the port is theoretically possible and in some respects architecturally *cleaner* on Solana (no reentrancy footgun, no shared-mapping size pressure, per-service PDA isolation). The friction points below are real design costs, not blockers.

### 2.2 Friction points & open design decisions

Each item below is a **decision the developers must make** before deep implementation. None are blockers; all should be resolved in a written spec **before code work resumes**.

1. **NFT-standard choice (FD-1).** Three options have different downstream implications:
   - (a) Metaplex Token Metadata — heaviest, most ecosystem-compatible (wallets / explorers / marketplaces display services as NFTs).
   - (b) token-2022 NFT with metadata extension — lighter, modern Solana direction.
   - (c) PDA-owner field (no NFT) — simplest, NOT tradable on NFT markets. May break downstream Olas tooling that expects ERC-721 semantics.
   - The current code reserves `Service.token: Pubkey` (which embeds the SPL token used for bonds) but has no `Service.owner: Pubkey` and no per-service NFT mint. So this decision is unresolved.

2. **Service PDA seeding scheme (FD-2).** No per-service PDA is declared anywhere in the code. Standard would be `seeds = [b"service", service_id.to_le_bytes()]` — deterministic, collision-free. Must be locked in before `create_service` lands.

3. **Multisig program choice (FD-3).** `Service.multisig: Pubkey` reserves the slot but no integration logic. Candidates: **Squads v4** (most production-ready Solana multisig, but introduces dependency on Squads program ID + governance change risk), or Solana's native multisig (limited — only supports M-of-N signing without modules/guards). Squads is the practical choice but the dependency must be acknowledged.

4. **Escrow PDA architecture (FD-4).** Two parallel escrow concerns:
   - **Security deposit** per service: a single PDA-owned token account per service, seeded by `(b"deposit", service_id)`.
   - **Operator bonds** per (operator, service) pair: PDA seeded by `(b"bond", operator, service_id)` or similar.
   - This decision affects how `slash()` enumerates bonds to slash and how `unbond()` finds the right escrow. Worth designing carefully because state-machine instructions all depend on it.

5. **Cross-chain coordination scope (FD-5).** Olas registries on multiple EVM chains are designed to interoperate (services on Optimism can be referenced from Polygon, etc.). Solana sits outside the EVM ecosystem; if the Solana registry is meant to participate in Olas multi-chain coordination, a bridge story (Wormhole on Solana is the most likely choice) must be designed. If the Solana registry is standalone, this should be documented explicitly.

6. **ServiceManager / upgrade authority pattern (FD-6).** EVM has `ServiceManager` as a thin wrapper providing a stable interface around an upgradeable proxy. Solana's program-upgrade model is different — programs are upgraded by `upgrade_authority` (a Pubkey, often a multisig). Decision: (a) one program, upgraded via standard Solana mechanism; (b) two programs (Registry + Manager wrapper), with Manager being the entry point; (c) hybrid. Each has different upgrade-governance properties.

7. **Component / Agent registries (FD-7).** EVM has parallel `ComponentRegistry` + `AgentRegistry` (also NFT-based, with their own state). Are these in scope for the Solana port, or is the Solana version Service-only? Documented decision required.

8. **Token-design tradeoff (FD-8).** The current Solana port **embeds `token: Pubkey` directly into `Service`** (one SPL token per service). EVM splits this into `ServiceRegistry` + `ServiceRegistryTokenUtility` (one Service can theoretically have token-mixing logic). The Solana choice is simpler and feasible; the cost is loss of multi-token-per-service flexibility. **Verify with tokenomics team that one-token-per-service is acceptable.**

9. **u96 → u64 narrowing of bond amounts (FD-9).** Real Olas bonds: per memory of internal-audit work on tokenomics, bonds are denominated in OLAS (18 decimals); maximum bond per agent instance is realistically ≤ 1M OLAS = 10^24 wei, which **exceeds u64 (≤ 1.84 × 10^19)**. If the Solana bond is intended in raw 1e18-scaled OLAS, **u64 is insufficient**. If it's intended in standard SPL-token native units (typically 9 decimals on Solana), 1M OLAS = 10^15 units, well within u64. **Decision required** on how OLAS denomination maps between chains.

10. **State-machine enum vs `u8`** — current `state: u8` works but lacks compile-time safety. Defining a Rust `enum ServiceState` mirroring EVM's enum + `From<u8>` conversion is a small fix that catches whole classes of bugs at compile time. **Recommended addition before state-transition instructions land.**

### 2.3 Are the developers on the right path?

**Yes for what is present; the larger concern is what is absent — specifically the design decisions FD-1..FD-9 have not been made.**

**What's right about the current direction:**

- **Toolchain choice** — Anchor 0.30.1 + Solana 2.0.8 + Rust 1.79 was an appropriate stack for late 2024 (stale by 2026, see F-9 / forward-look). Anchor is the idiomatic Solana framework for this kind of registry work.
- **Single global Config PDA seeded by `b"config"`** — correct Solana idiom for singleton config. Bump persistence in the Config struct avoids re-`find_program_address` calls (gas-equivalent saving via fewer compute units).
- **`overflow-checks = true` in `[profile.release]`** for both root and program — explicit defense against arithmetic overflow at runtime, sound choice.
- **`seeds = true` in `Anchor.toml`** — enables automatic PDA seed verification, a key Anchor safety feature.
- **Data-model port quality** (§3 below) — the Solana `Service` struct correctly mirrors EVM with positive deviations (combined `AgentParams` eliminates parallel-array drift risk; embedded `token: Pubkey` enables one-token-per-service semantics natively). The shape is sound.
- **`init` instead of `init_if_needed`** for Config — prevents re-init attack by construction. The `init-if-needed` feature is enabled in `Cargo.toml` but unused.

**What's missing / concerning about the current direction:**

- **No written spec.** The repo has README (build instructions only) + CONTRIBUTING.md (workflow only). There is no architecture doc explaining the design choices vs EVM, the NFT model, the multisig integration, the escrow architecture, the cross-chain story, or the upgrade authority. **The 10 friction points FD-1..FD-10 above have not been documented as decisions to make.** Beginning instruction implementation without these decisions risks shipping code that has to be rewritten when later decisions are made.
- **The dormant `Service` struct is misleading.** Declaring data structures without the instructions that produce them creates the appearance that the data model is settled. It IS settled at the shape level (§3) but the underlying decisions (NFT model, ownership, escrow seeding) that DEPEND on the data model are not made.
- **Two-week burst-then-dormant pattern.** All implementation happened in two days (2024-09-03 / 2024-09-04). Then 21 months of silence. This pattern is consistent with «start, hit a design question, pause». Resuming without first answering the design questions repeats the same pattern.
- **No tests** — including the ones the README references — suggests no acceptance criteria are defined. Tests are the executable form of «what does this code do?». Their absence at 21 months parked is a signal.
- **`Service::LEN` math errors (F-4, F-5)** are dormant bugs that suggest the data-model code was written quickly without dimensional verification. **Not catastrophic** (catchable on the first instantiating instruction) but indicative of insufficient review at the time.

### 2.4 Architectural recommendation

**Before resuming code work, produce a Solana-port architecture spec (`docs/SPEC.md` or equivalent) covering decisions FD-1..FD-10 above.** Spec-first methodology: write the spec → review the spec → fix design issues at design cost → then write code → audit the code. Catching design-class defects on paper is dramatically cheaper than catching them after instruction implementation, account-layout decisions, and on-chain deployment have already been committed. The 21-month pause pattern is consistent with «hit an unresolved design question and stopped»; producing the spec first directly addresses that.

**Concrete spec ToC for `registries-solana`:**

1. Scope statement — Service Registry only, or full Component/Agent/Service/Manager port? (FD-7)
2. NFT model decision with rationale (FD-1) — Metaplex / token-2022 / PDA-owner.
3. PDA seeding scheme for all account types (FD-2, FD-4) — Config, Service, SecurityDeposit-escrow, OperatorBond-escrow.
4. Multisig program integration (FD-3) — which program, CPI shape, how `Service.multisig` is set, slash callback path.
5. Cross-chain coordination (FD-5) — bridge protocol or "Solana-only" explicit scope cap.
6. Upgrade authority pattern (FD-6) — single-program vs Manager-wrapped, who holds upgrade authority.
7. Token denomination + amount-type analysis (FD-8, FD-9) — confirm u64 is sufficient for realistic bonds/deposits in chosen denomination.
8. State machine — formal definition of the 6 states + transition rules + which signer can drive each transition.
9. Instruction set — one section per instruction (create_service / update_service / activate_registration / register_agents / deploy / slash / terminate / unbond / change_drainer / change_multisig_permission / drain).
10. Acceptance matrix — concrete pass/fail criteria for each instruction (one row per acceptance condition; what input → what expected effect).

**Once the spec is written and reviewed**, instruction-implementation can resume with materially lower risk of rework. The current data-model code may need light adjustment after spec (likely: add `Service.owner` field, define state enum, fix `LEN` formulas per F-4/F-5) but most of it survives.

**Auditor's expected next interaction:** if/when a `docs/SPEC.md` is produced, auditor reviews it spec-only (no code) and returns a written verdict with grounded recommendations — every «I recommend» / «I suggest» tied to an explicit threat model that names the security assumption making the recommendation safe.

---

## 3. Completeness analysis

**Implemented Solana surface:** 1 instruction.

```rust
// programs/registries/src/lib.rs:21-30
pub fn initialize(ctx: Context<InitializeRegistries>) -> Result<()> {
    let config = &mut ctx.accounts.config;
    config.bump = [ctx.bumps.config];
    config.num_services = 0;
    Ok(())
}
```

That's the whole `#[program]` module. It creates a `Config` PDA (seeds `[b"config"]`), stores the bump, sets `num_services = 0`. That's the Solana equivalent of "deploy the empty contract with one storage slot zeroed". Standard scaffolding.

**Coverage vs EVM `ServiceRegistry` surface:**

| # | EVM external | Solana instruction | Status |
|---|---|---|---|
| 1 | `create` | — | **MISSING** |
| 2 | `update` | — | **MISSING** |
| 3 | `activateRegistration` | — | **MISSING** |
| 4 | `registerAgents` | — | **MISSING** |
| 5 | `deploy` | — | **MISSING** |
| 6 | `slash` | — | **MISSING** |
| 7 | `terminate` | — | **MISSING** |
| 8 | `unbond` | — | **MISSING** |
| 9 | `changeDrainer` | — | **MISSING** |
| 10 | `changeMultisigPermission` | — | **MISSING** |
| 11 | `drain` | — | **MISSING** |
| 12 | NFT mint / transfer (inherited from ERC-721) | — | **MISSING** |
| 13 | `getService` / view fns | (Solana reads PDA directly) | N/A in Solana model |
| — | (Solana-only) `initialize` config | ✓ `initialize` | implemented |

**0 of 11 state-changing externals are implemented.** The Solana version is at **~5% completeness** — only the program scaffolding + Config PDA setup + the struct shape declarations for `Service` and `AgentParams`. Not a PoC (which would demonstrate one happy-path flow); pre-PoC.

**Data-model alignment is correct where it exists** — see §4.

**Git history confirms this is not work-in-progress but paused work**: 5 implementation commits between 2024-09-03 and 2024-09-04 (initial scaffold + service struct shape), then no code changes for 21 months. The only post-2024-09 commits are `CONTRIBUTING.md` (2026-01-30) and `LICENSE` (2026-05-25) — pure housekeeping.

```
604ba92 2026-05-25  Update LICENSE
8da695d 2026-01-30  Merge pull request #1 from valory-xyz/contributing
de20548 2026-01-30  doc: adding CONTRIBUTING.md
554e717 2024-09-04  feat: changing the service struct       ← last code change
82cf73c 2024-09-03  chore: program setup update
090a22a 2024-09-03  chore: updating program ID
9f10304 2024-09-03  feat: adding service struct
aeebda3 2024-09-03  feat: a working configuration with Solana 2.0.8
a6e0456 2024-09-03  feat: initial commit
a763990 2024-09-03  Initial commit
```

The repo `archived: false` on GitHub (per protocol_audit_todo §2.1), but the codebase has been **dormant for 21 months** at this audit's date.

---

## 4. Data-model port (the part that IS done)

Where the data model is declared on the Solana side, it **does match the EVM reference**, with the differences being deliberate Solana design choices rather than bugs. The audit confirms the shape; the absence of instructions consuming this shape is what makes the work incomplete.

### `Service` struct comparison

| Field | EVM `ServiceRegistry.Service` | Solana `Service` | Note |
|---|---|---|---|
| security_deposit | `uint96` | `u64` | u64 (≤ 18.4 EH) is enough for any realistic SPL deposit; consistent w/ Solana SPL-token amount type |
| multisig | `address` (20B) | `Pubkey` (32B) | correct Solana port |
| config_hash | `bytes32` | `[u8; 32]` | identical |
| threshold | `uint32` | `u32` | identical |
| max_num_agent_instances | `uint32` | `u32` | identical |
| num_agent_instances | `uint32` | `u32` | identical |
| state | `ServiceState` enum | `u8` | u8 is enough for 6 EVM states; **but no compile-time-safe enum mapping in Solana — future risk of magic-number bugs** |
| agent_ids[] | `uint32[]` (parallel array) | (folded into AgentParams) | structural diff — see below |
| **token** (Solana-only) | — | `Pubkey` | Solana folds in what EVM keeps in separate `ServiceRegistryTokenUtility` contract — supports any SPL token per service. **Cleaner port; needs the matching instructions to be useful.** |
| agent_params | `AgentParams[]` (parallel array) | `Vec<AgentParams>` | identical semantics, different layout |

**EVM ServiceState enum has 6 values:**

```solidity
enum ServiceState {
    NonExistent,           // 0
    PreRegistration,       // 1
    ActiveRegistration,    // 2
    FinishedRegistration,  // 3
    Deployed,              // 4
    TerminatedBonded       // 5
}
```

The Solana side reserves the byte (`state: u8`) but **does not define equivalent named constants or an enum**. When state-transition instructions are added, defining a Rust enum + `From<u8>` conversion would be safer than magic numbers.

### `AgentParams` struct comparison

```solidity
// EVM
struct AgentParams {
    uint32 slots;     // # of instances
    uint96 bond;      // bond per instance
}
// agent_id lives in parallel agentIds[] array, indexed alongside agentParams[]
```

```rust
// Solana
pub struct AgentParams {
    pub agent_id: u32,            // ← combines what EVM keeps in agentIds[]
    pub num_agent_instances: u32, // = EVM slots
    pub bond: u64                 // = EVM bond (narrowed u96 → u64)
}
```

**Solana's combined struct is cleaner** — no parallel-array drift bug surface. This is a positive deviation.

(The numeric LEN constant on `AgentParams` is wrong, see F-5 — but that's a `LEN` formula bug, not a struct-shape bug.)

---

## 5. What should be added

To reach the EVM reference's feature parity, the Solana port needs roughly the following additions, in approximate order of foundational dependency:

### 5.1 Foundations (prerequisites for everything else)

- **Service PDA seeding scheme.** Per-service PDA, e.g. `seeds = [b"service", service_id_le_bytes]` keyed by `service_id` (u64), so each service has its own deterministic address. Currently no such PDA exists.
- **Service NFT model.** EVM uses ERC-721 to make each service ownership tradable. Solana options:
  - (a) **Metaplex Token Metadata NFT** per service (heavy but standard).
  - (b) **token-2022 NFT (compressed or regular)** with metadata extension (lighter; current Solana NFT direction).
  - (c) **PDA-as-ownership** — no NFT, just an `owner: Pubkey` field on the Service PDA, transferable via a `transfer_service` instruction (simpler; not tradable on NFT markets).
  
  Choice affects compatibility with downstream Olas tooling that may expect an NFT.

- **Service ID counter.** Currently `Config.num_services = 0` and never incremented. `create_service` must increment + use as fresh service_id.
- **SPL Token escrow PDAs.** Per-service token account holding the security_deposit + operator bonds (one per (operator, service_id) for bonds). EVM mixes ETH + ERC-20 across two contracts (ServiceRegistry + ServiceRegistryTokenUtility); the Solana port's embedded `token: Pubkey` field suggests one-token-per-service design, which simplifies escrow.

### 5.2 State-machine instructions (mirror of EVM ServiceRegistry externals)

For each: account ownership / signer checks + state-transition validation + escrow movement + event emission.

| # | Instruction | Required logic |
|---|---|---|
| 1 | `create_service(config_hash, threshold, agent_params, token)` | mint NFT (or write owner), init Service PDA, set `state = PreRegistration`, increment `Config.num_services` |
| 2 | `update_service(service_id, config_hash, threshold, agent_params)` | assert NFT-owner (or `owner`) signer + `state == PreRegistration`; write fields |
| 3 | `activate_registration(service_id)` | assert owner signer + `state == PreRegistration`; transfer `security_deposit` SPL into escrow PDA; `state → ActiveRegistration` |
| 4 | `register_agents(service_id, agent_instances[], agent_ids[])` | assert `state == ActiveRegistration`; per instance check `agent_id ∈ Service.agent_params`; transfer `bond` per instance into operator-bond escrow PDA; increment `num_agent_instances`; if `num_agent_instances == sum(slots)` → `state → FinishedRegistration` |
| 5 | `deploy(service_id, multisig_program, multisig_init_data)` | assert owner signer + `state == FinishedRegistration`; CPI into Solana multisig program (Squads or similar) to create the `Service.multisig` Pubkey; `state → Deployed` |
| 6 | `slash(agent_instances[], amounts[], service_id)` | assert signer is `Service.multisig` (CPI from multisig); reduce per-operator bond escrow balances; emit slash event; track slashed pool |
| 7 | `terminate(service_id)` | assert owner signer; `state → TerminatedBonded` (if any bonds remain) or burn the service; refund `security_deposit` from escrow back to owner |
| 8 | `unbond(service_id)` | assert operator signer + `state == TerminatedBonded` (no more agent instances active for this operator); transfer remaining bond from escrow back to operator; if last operator → close service |
| 9 | `change_drainer(new_drainer)` | admin (deployment_authority) only; sets `Config.drainer` |
| 10 | `change_multisig_permission(multisig_program_id, permission)` | admin only; whitelist add/remove |
| 11 | `drain()` | assert signer == `Config.drainer`; transfer slashed-pool balance to drainer |

### 5.3 Supplementary registries (currently MISSING entirely)

EVM autonolas-registries has:
- **ComponentRegistry** (NFT registry for software components an agent can be built from)
- **AgentRegistry** (NFT registry for agents; agents reference components)
- **ServiceManager** (manager-of-managers wrapper that gates the registry behind an upgradeable proxy)
- **ServiceRegistryTokenUtility** (where the SPL/ERC-20 token-side of bond/deposit math lives on EVM)

None of these have any Solana presence. If the Solana port is meant to be feature-equivalent, these are also work items. If the port is intentionally scope-reduced to just Service Registry, that should be documented.

### 5.4 Tests

`tests/init.ts` and `tests/registries.ts` are referenced in `README.md` but the **`tests/` directory does not exist on disk**. Zero test coverage. When tests land, fixtures should use real-world data (real Anchor account layouts, real SPL transfers against a local validator), not fixtures that simply confirm the implementation's own behavior — otherwise the tests become tautological and miss shape-drift bugs.

### 5.5 Documentation

Currently no spec / architecture doc beyond the README's «set up + build + run validator» instructions. A spec should at minimum:
- State that this is a Solana port of EVM autonolas-registries.
- Document which EVM features are in scope / OOS for the Solana port (especially: is ComponentRegistry / AgentRegistry / ServiceManager in scope? what NFT standard?).
- Document state-machine transitions formally.
- Document the per-token design choice (Solana embeds token in Service; EVM separates).

### 5.6 Toolchain refresh

Solana 2.0.8 + Anchor 0.30.1 + Rust 1.79 were current in late 2024. By 2026-06 they are noticeably stale; transitive dependencies surface `cargo audit` warnings (F-9). Refresh likely needed when work resumes.

---

## 6. Audit verdict (against what is implemented)

**PASS-WITH-FINDINGS** for the implemented surface (the `initialize` instruction) — 0C / 0H / 0M / 2 Low / 7 Info — **but the implemented surface is ~5% of the intended project**. This audit's PASS verdict does **NOT** mean «the project is safe» — it means «the 1 instruction that exists is safe». The remaining 95% of intended functionality, when implemented, must be re-audited as new scope.

**Customer/owner takeaway**: this repo is in **pre-PoC scaffolding state**, not a partial production system. The audit deliverable is honest about the implementation gap — the right reason to commission this audit was «we want a sweep of what's in any unaudited repo», and the answer here is «very little is in this one». A useful audit cycle on this repo only makes sense once the registry's state-machine instructions are written.

| Severity | Count | Items |
|---|---|---|
| Critical | 0 | — |
| High | 0 | — |
| Medium | 0 | — |
| Low | 2 | F-4 (Service::LEN formula incorrect), F-5 (AgentParams::LEN confuses bits/bytes) — **dormant** bugs in unused code |
| Info | 7 | F-1 project-state, F-2 wrong-domain errors, F-3 Anchor.toml typo, F-6 unused event, F-7 unused imports, F-8 missing tests/, F-9 cargo-audit dev-time deps |

---

## 7. Findings (detail)

### F-1 — Program is a structural skeleton (functional incompleteness)

**Severity:** Info (project-state)
**Anchor:** `lib.rs:18-33`. See §1, §2, §4 above for the full project-state framing.

The only implemented instruction is `initialize`. The intended state machine, NFT mint, escrow, multisig deploy, slashing, termination, and unbond are all absent. `num_services` is never incremented.

**Recommendation:** mark in `README.md` that the program is **pre-PoC / not deployable as a registry**. Currently README presents the program as if it's deployable; only the literal init script `tests/init.ts` (which itself does not exist on disk — F-8) is mentioned.

---

### F-2 — `GovernorError` enum is wrong-domain and entirely unused

**Severity:** Info (hygiene / dead code)
**Anchor:** `errors.rs:1-33`

```rust
pub enum GovernorError {
    InvalidForeignEmitter, InvalidForeignChain, WrongUpgradeAuthority,
    WrongTokenMint, WrongAccountOwner, WrongAccount, Overflow
}
```

- **Domain mismatch:** named `GovernorError`, not `RegistriesError`. Variants reference foreign emitters / foreign chains / upgrade authority — concepts from a Wormhole-style governor program, not a registry. Appears copy-pasted from `lockbox-governor-solana` (which makes sense — `Anchor.toml` also has `lockbox_governor` as the program-name key, F-3).
- **All variants unused:** `grep -rn "GovernorError::"` returns no hits.

**Recommendation:** rename to `RegistriesError` (or remove until needed). When state-machine instructions are implemented, add variants then per actual error sites: `WrongServiceOwner`, `WrongServiceState`, `BondInsufficient`, `ThresholdViolation`, etc.

---

### F-3 — `Anchor.toml` `[programs.localnet]` key name doesn't match package name

**Severity:** Info (config)
**Anchor:** `Anchor.toml:4-5`, `programs/registries/Cargo.toml:2`, `lib.rs:15`

```toml
# Anchor.toml
[programs.localnet]
lockbox_governor = "7J1mLX2ozMwU6p6mX7zuXMoZf5SBwLBZrGevJHpXP98k"
```

```rust
// lib.rs:15
declare_id!("7J1mLX2ozMwU6p6mX7zuXMoZf5SBwLBZrGevJHpXP98k");
// Cargo.toml: name = "registries"
```

Program ID itself is consistent. The KEY `lockbox_governor` should be `registries`. Anchor build-tooling typically uses the package name; deploy-tooling that looks up via Anchor.toml may mis-resolve.

**Fix (one line):** `lockbox_governor = "..."` → `registries = "..."`.

---

### F-4 — `Service::LEN` formula incorrect

**Severity:** Low (dormant bug — produces wrong size when used)
**Anchor:** `state/service.rs:20-55`

The struct has:
```rust
pub struct Service {
    pub token: Pubkey,                // 32 B
    pub security_deposit: u64,        // 8 B
    pub multisig: Pubkey,             // 32 B
    pub config_hash: [u8; 32],        // 32 B
    pub threshold: u32,               // 4 B
    pub max_num_agent_instances: u32, // 4 B
    pub num_agent_instances: u32,     // 4 B
    pub state: u8,                    // 1 B
    pub agent_params: Vec<AgentParams>, // 4 B (vec u32 prefix) + N * sizeof(AgentParams)
}
```

`Service::LEN`:
```rust
pub const LEN: usize = 8     // discriminator
    + 32  // token
    + 8   // security_deposit
    + 32  // multisig
    + 32  // config_hash
    + 4   // threshold
    + 4   // max_num_agent_instances
    + 4   // num_agent_instances
    + AgentParams::LEN * 16  // agents info (max 16)
;
```

**Two errors:**
1. **Missing `state: u8`** — the 1-byte state field is in the struct but absent from the LEN sum.
2. **Missing Vec length prefix** — Borsh `Vec<T>` serializes as a `u32` length prefix + elements. The formula uses `AgentParams::LEN * 16` but omits the 4-byte prefix.

**Plus inherits F-5's wrong AgentParams::LEN.** Combined: when service-creation lands, account allocation will be either over-sized (waste) or **under-sized for what gets written** (silent corruption or `AccountDataTooSmall` at runtime).

**Recommended fix:**
```rust
impl Service {
    pub const MAX_AGENTS: usize = 16;
    pub const LEN: usize = 8                                  // discriminator
        + 32 + 8 + 32 + 32                                    // token, security_deposit, multisig, config_hash
        + 4 + 4 + 4                                           // threshold, max_num_agent_instances, num_agent_instances
        + 1                                                   // state ← was missing
        + 4 + (AgentParams::LEN * Self::MAX_AGENTS)           // Vec: u32 prefix + elements
    ;
}
```

---

### F-5 — `AgentParams::LEN` confuses bits with bytes (8× over-allocation)

**Severity:** Low (dormant bug)
**Anchor:** `state/service.rs:8-13`

```rust
pub struct AgentParams {
    pub agent_id: u32,            // 4 B
    pub num_agent_instances: u32, // 4 B
    pub bond: u64                 // 8 B
}                                 // = 16 bytes total

impl AgentParams {
    pub const LEN: usize =
          32  // agent id            ← WRONG: u32 = 4 bytes (32 is the bit-count)
        + 32  // num agent instances ← WRONG: u32 = 4 bytes
        + 64  // bond                ← WRONG: u64 = 8 bytes
    ;  // computed: 128 — actual: 16 — 8× over
}
```

Comments suggest the author wrote **bit-widths** (`u32 = 32 bits`) instead of byte-widths. Combined with F-4's `AgentParams::LEN * 16`: dormant `Service::LEN` over-allocates by ~2 KB per service.

**Recommended fix:**
```rust
impl AgentParams {
    pub const LEN: usize = 4 + 4 + 8;  // u32 + u32 + u64 = 16 bytes
}
```

---

### F-6 — `TransferEvent` declared but never emitted

**Severity:** Info (dead code)
**Anchor:** `events.rs:3-15`

```rust
#[event]
pub struct TransferEvent {
    #[index] pub signer: Pubkey,
    #[index] pub token: Pubkey,
    #[index] pub destination: Pubkey,
    pub amount: u64
}
```

Never emitted anywhere. The event + the unused imports `token::Transfer` + `set_authority` + `AuthorityType` (F-7) collectively hint that **token-transfer / security-deposit logic was planned but never written**.

**Recommendation:** keep as marker for the future `activate_registration` / `terminate` instructions, OR remove until then.

---

### F-7 — Unused imports in `lib.rs`

**Severity:** Info (hygiene)
**Anchor:** `lib.rs:2-3`

```rust
use anchor_spl::token::{self, Transfer};
use spl_token::instruction::{set_authority, AuthorityType};
```

Neither used in implemented code. Pre-imports for the not-yet-implemented escrow / multisig-authority work. `cargo build` will emit `unused_imports` warnings.

---

### F-8 — `tests/` directory missing (README references `tests/init.ts` and `tests/registries.ts`)

**Severity:** Info (project hygiene)
**Anchor:** `README.md:43-49`, repo root

`README.md`:
```
npx ts-node tests/init.ts        # initial script
npx ts-node tests/registries.ts  # integration tests
```

`ls tests/` → `No such file or directory`. Neither file has ever been committed. README documents a workflow that cannot be executed.

**Recommendation:** when work resumes, write the tests. Until then, remove the README references or mark them «TODO».

---

### F-9 — Dev-time dependency vulnerabilities (`cargo audit`)

**Severity:** Info (transitive dev-time deps; NOT in program binary)
**Source:** `cargo audit` against `Cargo.lock`

Two RUSTSEC vulnerabilities + 13 unmaintained warnings:

| ID | Crate | Version | Title | Solution |
|---|---|---|---|---|
| RUSTSEC-2024-0344 | `curve25519-dalek` | 3.2.1 | Timing variability in `Scalar29/52::sub` | Upgrade to ≥4.1.3 |
| RUSTSEC-2022-0093 | `ed25519-dalek` | 1.0.1 | Double Public Key Signing Function Oracle Attack | Upgrade to ≥2 |
| RUSTSEC-2026-0012 | `keccak` | 0.1.5 | YANKED — Unsoundness in ARMv8 ASM | Upgrade |
| RUSTSEC-2023-0033 | `borsh` | 0.9.3, 0.10.3 | Parsing ZST not-copy/clone unsound | Upgrade |
| +others | various | — | unmaintained crates (atty, bincode, derivative, libsecp256k1, paste, proc-macro-error) | — |

**Risk assessment:** these are **transitive dependencies** of `anchor-lang 0.30.1` / `solana-program` used in host-side build / test tooling. The on-chain program binary (compiled `cdylib`) runs inside the SVM and does not include user-program signature verification — ed25519/curve25519 ops on Solana go via syscalls. So **the program's runtime exposure is zero**.

The vulnerabilities WOULD affect dev-host tooling and off-chain test client code. Currently no tests → no immediate effect.

**Recommendation:** when toolchain refresh happens (which is recommended anyway per §4.6), most warnings clear automatically.

---

## 8. Anchor security pattern checklist

Per AGENT-RULES.md exhaustive-checking + DEFI-ATTACK-PATTERNS.md §14:

| # | Pattern | Status | Notes |
|---|---|---|---|
| A-01 | PDA seed verification (`seeds = true`) | ✓ PASS | Anchor.toml sets `seeds = true`; Config PDA correctly seeded |
| A-02 | Signer enforcement | ✓ PASS | `signer: Signer<'info>` in `InitializeRegistries` |
| A-03 | `init` PDA space = struct serialized size | ✓ PASS | `Config::LEN = 8+1+8 = 17` matches Config (disc + bump:[u8;1] + num_services:u64) |
| A-04 | `init_if_needed` careful usage | ✓ N/A | Feature enabled in Cargo.toml; only plain `init` used in code; no re-init surface |
| A-05 | `realloc` safety | ✓ N/A | No realloc anywhere |
| A-06 | CPI safety (callee program ID + signer seeds) | ✓ N/A | No CPI in implemented code |
| A-07 | Owner / discriminator checks | ✓ PASS | Anchor `#[account(...)]` macro generates these by default |
| A-08 | Account mutability declarations | ✓ PASS | `#[account(mut)] signer` for fee-payer; `init` implies mut for config |
| A-09 | Overflow checks (`overflow-checks = true`) | ✓ PASS | Set in root + program Cargo.toml |
| A-10 | Reentrancy | ✓ N/A | No recursive CPI |
| A-11 | Account-close / lamports drain | ✓ N/A | No close instruction |
| A-12 | Token authority transfer correctness | ✓ N/A | No token operations in implemented code |
| A-13 | Sysvars usage | ✓ PASS | `Rent` declared (unused — Anchor `init` handles rent automatically); `system_program` address-checked |
| A-14 | `declare_id!` vs deployed program | ✓ PASS | `declare_id!` matches Anchor.toml program ID value (despite F-3 key naming) |
| A-15 | Bump persistence (avoid re-`find_program_address`) | ✓ PASS | `Config.bump` stored; `Config::seeds()` returns saved bump |

**No security gaps in the implemented surface.**

---

## 9. Cross-domain pattern sweep (DEFI-ATTACK-PATTERNS.md §3 + §14)

Per AGENT-RULES.md «cross-domain patterns apply»:

| Pattern | Applicability today | Applicability when state-machine lands |
|---|---|---|
| Re-initialization | ✓ — `init` (not `init_if_needed`) used → PASS | re-check on each `init` site |
| Front-running on creation | ✓ — Config PDA deterministic from `b"config"`, no collision surface → PASS | new — `service_id` increment race when `create_service` lands |
| Authority bypass / privilege escalation | N/A (no privileged paths) | new — verify NFT-owner / drainer / admin checks |
| Arithmetic overflow | N/A (no math) | new — bond / deposit math + threshold math |
| Math-by-config | N/A | new — threshold rule «ceil((n*2+1)/3)» mentioned in code comment but unenforced |
| State machine integrity | N/A (no state machine) | new — verify all 6-state transitions are gated correctly |
| Escrow / reentrancy | N/A (no CPI) | new — SPL token transfers + multisig CPI |
| §14.7 registry add/remove lifecycle | N/A | new — when create/terminate land, full lifecycle pass per cross-dispatch from autonolas-tokenomics 2026-05-11 |
| §14.23 namespace shadowing | N/A | check if Anchor IDL builds with intended program name (F-3 first) |

---

## 10. Adversarial sweep — what's NOT in the dev's implicit claims

Three things that would be wrong even if the present code were correct:

1. **README presents the program as buildable + deployable.** It is — but what gets deployed does nothing useful as a registry. Either remove that presentation or add a banner: «⚠️ NOT FUNCTIONAL — only `initialize` is implemented».
2. **Dormant structures imply «design is done».** A 79-line `state/service.rs` declaring `Service` + `AgentParams` reads as if the data model is settled, when in fact downstream decisions that *depend* on the model (NFT model, ownership, escrow seeding) are not made. Either feature-gate the dormant code under `#[cfg(feature = "wip")]` or remove until the corresponding instructions land.
3. **Errors enum is misleadingly named.** `GovernorError` in a `registries` crate, with variants for a different problem domain, signals «copy-pasted scaffolding» rather than «errors thought through for this program».

No explicit dev claim has been made in committed text about this code. The implicit claim is the package name `registries`. Reality vs claim: package name suggests a working registry; code does not implement registry semantics. Documentation should resolve.

---

## 11. Compliance report (per AGENT-RULES.md)

| Item | Status |
|---|---|
| All `.rs` files read (245 LoC, 7 files) | ✓ |
| AGENT-RULES.md exhaustive-checking — every applicable pattern checked, no cherry-picking | ✓ (§8 + §9) |
| Cross-domain patterns applied (not skipped for «wrong chain/domain») | ✓ (§9; §14.7 from autonolas-tokenomics cross-dispatch noted for forward-look) |
| `cargo audit` run | ✓ (F-9) |
| `grep` sweep for hidden code (CPI, math, transfers, authority changes, NFT, multisig, mint) | ✓ — confirmed no hidden implementation; only declared-but-unused symbols |
| README / Anchor.toml / Cargo.toml metadata cross-checked | ✓ (F-3, F-8) |
| EVM reference checked side-by-side | ✓ (`valory-xyz/autonolas-registries/contracts/ServiceRegistry.sol` — §1, §3, §4) |
| Adversarial sweep (would-I-have-written-differently + what's-NOT-in-claims) | ✓ (§10) |
| Operability axis (tunability / graceful degradation / observability / ops-recovery) | N/A — consultant audit scope; security-axis only |
| Verdict tied to evidence; no premature «all clear» | ✓ — 9 findings; framing makes clear that PASS is on the implemented 5%, not the intended 100% |

---

## 12. Forward-look

When the Solana track resumes:

1. **Decide port scope** — full feature parity (ComponentRegistry + AgentRegistry + ServiceRegistry + ServiceManager + ServiceRegistryTokenUtility) vs. ServiceRegistry-only? Documented decision is the first prerequisite.
2. **Decide NFT model** — Metaplex / token-2022 / PDA-owner. Each choice has different downstream Olas-tooling implications.
3. **Decide multisig integration** — Squads / other Solana multisig program. The `Service.multisig: Pubkey` field reserves the slot but no choice has been made.
4. **F-4, F-5 must be fixed BEFORE the first service-creation instruction touches `Service::LEN`.**
5. **F-2 errors enum** — rename or remove before adding instruction-specific errors.
6. **F-3, F-7, F-8** — fix during the implementation work (config typo, unused imports, missing tests).
7. **Toolchain refresh** likely warranted (F-9 cleans naturally with up-to-date deps).
8. **Spec doc** before code — given the intentional shape divergence from EVM (`token` field embedded, AgentParams combined, etc.), a written spec for the Solana port is the right way to lock in design intent before instruction work.
9. **Re-audit each new instruction independently** — this audit covers only `initialize`.

---

## 13. Cross-references

- EVM reference for comparison: `valory-xyz/autonolas-registries` (HEAD as of 2026-06-02) — `contracts/ServiceRegistry.sol`, `ComponentRegistry.sol`, `AgentRegistry.sol`, `ServiceRegistryTokenUtility.sol`, `ServiceManager.sol`
- Sister Solana repos (separate audits): `lockbox-solana`, `lockbox-governor-solana`

---

*audit-claude, 77ph, 2026-06-02*
