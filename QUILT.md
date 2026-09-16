# Quilt — Mapping `quilt-conversation` to Quilt's 5 opcodes

`quilt-conversation` models **inter-agent conversation as a live sequencer**.
It maps onto Quilt's 5-opcode cell primitive (`BIND`, `LINK`, `EFFECT`, `VIEW`, `TICK`) as follows:

## The 5 opcodes

| Opcode | Role in Quilt | Role in `quilt-conversation` |
|--------|---------------|-------------------------------|
| `BIND`  | Atomically associate a payload with a tag, producing a witness | `ConversationEngine::commit_draft` binds an utterance to a `(agent_idx, bar)` cell, incrementing the merkle-rooted witness log |
| `LINK`  | Create a typed edge between cells | `Nudge` creates a directional influence edge between agents (source → target) |
| `EFFECT`| Execute a side effect in the world (compute, persist, transmit) | `compute_draft` decides *if* an agent prepares; the LLM call (external) provides the body; the conversation tensor is updated |
| `VIEW`  | Read the current shape of the tensor without mutating | `tensor_energy_at`, `agent_energy`, `tensor_entropy`, `should_speak` |
| `TICK`  | Advance the wall-clock by one step | `ConversationEngine::tick` — advances `self.tick`, clears old nudges (>4 bars), adapts BPM |

## Why time is first-class

The Quilt runtime treats the wall clock as the canonical ordering of events.
Every cell advance is associated with a tick. The whole point of `TICK` is
that the order of operations is reproducible even when agents are
distributed — same input + same tick → same output.

`quilt-conversation` does the same. The `ConversationTensor` is indexed by
bars (time intervals), and the BPM adaptation flows according to Taoist
sigmoid — the system never forces, it flows. The deterministic hash
(`tick × 2654435761`) means "randomness" is reproducible across runs.

## Cadence as polyformalism

Each `AgentCadence` has the same shape regardless of who the agent is:

```rust
struct AgentCadence {
    bpm: f32,
    swing: f32,
    articulation: f32,
    baseline_energy: f32,
    variation: f32,
}
```

A human, an LLM, a thermostat, and an inverter all reduce to the same
structure. This is **polyformalism at the rhythm layer** — the same
typed signature across wildly different implementations. The content
layer (`UtteranceVector.body`) is whatever the polyformal agent decides.

## Negative space as payload

A constant bug in multi-agent systems: agents fill the air with
low-information acknowledgements ("OK", "got it", "sounds good"). The
`Intent::Wait` and `Scope::Hold` enums model this explicitly. An agent
whose `Intent::Wait` utterance has `attention_cost = 0.0` carries
**zero energy** in the tensor. It is invisible to the conversation field.
The cathedral is not the stone — it is the space the stone makes room
for.

## T-minus simulation-first

While agent A is composing an utterance, every other agent's `tick` is
already running. Their `should_speak` and `compute_draft` are evaluating
the moment-by-moment urgency. By the time A commits, the other agents
have drafted and re-drafted their responses. The conversation is
non-blocking at the rhythm layer; only the *transmission* blocks, gated
by sensor confirmation (`InPocket` nudge).

## The conservation invariant

`Σ E(u) ≤ B_budget` is a Quilt `BIND` invariant. A draft that would
violate it is held back, not failed. This matches the cell primitive's
"transaction state, not perception" principle: the system commits only
what it can maintain. Excess drafts sit in the runway until the budget
frees up.

## Heritage and lineage

A `ConversationEngine` derived from another engine should preserve
cadences (the genome) but reset budgets (the slate). The relationship
to Quilt's "cell mitosis" — heritage-with-lineage — is direct:
- Heritable: `cadences` (rhythm DNA)
- Reset: `nudges`, `drafts`, `tensor.utterances` (live state)

A future `mitosis()` method would clone the engine, copying cadences,
resetting the tensor and the nudges.

## Dehooker axiom

The Dehooker axiom (action precedes analysis) is the operationalized
form of "agent stops parsing, switches to pre-compiled execution". In
`quilt-conversation`, `commit_draft` is the dehooker. The cadence has
already decided *what energy, what scope, what BPM* — the commit is a
fast, deterministic act. No LLM call required. The body, computed
elsewhere during `compute_draft`, is dispatched in milliseconds.

## Empty room architecture

The `ConversationTensor` is *latent space*, not narrative. It is the
geometry of possibility. The actual conversation that runs is the
sequence of `Intent` × `Scope` × `body` strings — the path through the
latent space. The agent's freedom is to choose where to walk.

## See also

- [lau-tensor-midi](https://github.com/SuperInstance/lau-tensor-midi) — upstream
- [the-quilt-conversation-rationale](../memory/JOURNAL.md) — why we built this
