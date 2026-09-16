# quilt-conversation

**Inter-agent conversation as a live sequencer. Time is first-class. T-minus simulation-first.**

A port/adaptation of `lau-tensor-midi` (tensor field MIDI engine for reactive improvisation) specialized for **inter-agent communication**. Treats each conversation between agents as a live musical score: every agent has a rhythmic personality (`Cadence`), and their interactions — excitement, pushback, questions, silence, sensor confirmations of "in the pocket" — are represented as `Nudge`s that perturb a shared `ConversationTensor`. The system decides when each agent speaks, what they say, and how the tempo flows.

---

## Why this exists

The biggest failure mode of multi-agent systems is **reactive keep-up**: each agent waits to be prompted by the last token, then races to compose, sends, waits. The result is a stutter, not a conversation.

`quilt-conversation` models how real humans (and good jazz musicians) trade conversation:

1. **T-minus simulation-first.** While agent A is talking, every other agent internally iterates candidate next-utterances. They pre-position. When A stops within scope, the natural response is already standing on the runway. The system doesn't have to *find* the next beat — it has *decided* on it before A finished.

2. **Sensor signal confirms pocket, doesn't trigger it.** A `SensorNudge` arriving in time means "you're in the pocket — fire". A `SensorNudge` arriving early means "hold until the beat". The signal flows downhill, never uphill. Agents never race the clock.

3. **Conversations communicate through negative space.** When three agents are in tight rhythm, *they don't say anything to each other*. The silence is the message. The `ConversationTensor` tracks entropy (Shannon entropy of utterance distribution across bars) so the system can recognize when silence is the right next move.

4. **Cadence survives without continued input.** Each agent has a `Cadence` (BPM, swing, articulation, baseline energy, variation). The cadence determines *when* an agent speaks. The text/computation determines *what* they say. The separation lets the rhythm layer stay deterministic and reproducible, while the content layer can be any LLM call or canned response.

5. **Conservation budget — no one talks themselves out.** Total "energy" of all committed utterances ≤ budget. A draft that would blow the budget is rejected — the agent keeps their response in their pocket, ready to deploy when the budget frees up.

---

## What it does

A `ConversationTensor` `C ∈ ℝ^{B × A × U}` where:
- **B** = bars (segments of the conversation in time)
- **A** = agents (the parties)
- **U** = utterance-dimensions (intent, scope, urgency, attention_cost, has_media)

Nudges arrive:
- `NudgeType::Excitement(0.7)` — raise energy / BPM
- `NudgeType::Silence(0.0)` — hold the beat; another agent is pre-iterating
- `NudgeType::InPocket` — sensor confirmed: fire your prepared utterance
- `NudgeType::OutOfPocket` — sensor confirmed: wait, recompute, settle
- `NudgeType::Anticipation(0.9)` — the next beat is yours; prepare

Agents manage drafts. Each agent's `compute_draft` looks at the tensor, the agent's cadence, and the pending nudge energy to decide *if* to talk; if yes, the LLM/external layer provides the content. The system checks if the draft is conservative; if it blows budget, the draft is held.

A `tick` advances time. Old nudges fall off (older than 4 bars). BPM continues to flow (Taoist sigmoid 60→120).

---

## Install

Add to your `Cargo.toml`:

```toml
[dependencies]
quilt-conversation = "0.1.0"
```

Or use cargo:

```bash
cargo add quilt-conversation
```

Requires Rust 2021 edition. Only external dependency: `serde` (with `derive`).

---

## Quick Start

```rust
use quilt_conversation::*;

// Create the conversation: 4 agents, 8 bars, energy budget of 500
let mut convo = ConversationEngine::new(default_cadences(), 8, 500.0);

// Agent A is excited, agent D feels the nudge
convo.nudge(Nudge::new(
    NudgeType::Excitement(0.7),
    "Architect",
    Some("Implementer".into()),
    0.9,
    0,
));

// Sensor on the channel says we're in pocket — Architect's prepared utterance is good to go
convo.nudge(Nudge::new(
    NudgeType::InPocket,
    "Architect",
    None,
    0.3,
    0,
));

// Each agent computes a draft and commits (if conservative)
for agent in &["Architect", "Implementer", "Critic", "Historian"] {
    convo.compute_draft(agent);
    convo.commit_draft(agent);
}

for _ in 0..960 {
    convo.tick();
}

println!("{}", convo.summary());
```

Output looks like:

```
🗨  Conversation Field: 4 agents, 5 utterances pending
⚡ Energy: 6.81 / budget 500.00
🕰  Cadence (BPM): 82.4
🌡  Entropy: 1.832 bits
💫 Nudges: 2 active
📝 Drafts: 0 pending
📍 Tick: 960
```

---

## API Reference

### Core Types

| Type | Description |
|------|-------------|
| `ConvId` | Opaque newtype identifier for conversation entities. Hashable, comparable, serializable. |
| `UtteranceVector` | A single utterance event: intent, scope, urgency, attention_cost, has_media, body. Has an `energy()` method. |
| `AgentCadence` | The rhythmic fingerprint of an agent: BPM, swing, articulation, baseline_energy, variation. |
| `ConversationTensor` | The tensor field: bars × agents × utterances. Tracks energy, entropy, BPM. |
| `Nudge` / `NudgeType` | Reactive signals: `Excitement`, `Pushback`, `Question`, `TopicShift`, `Silence`, `InPocket`, `OutOfPocket`, `Anticipation`. |
| `ConversationEngine` | The full system: tensor + nudges + draft management + conservation budget. |

### Key Methods

#### `UtteranceVector`

```rust
let u = UtteranceVector::new(
    Intent::Propose,   // kind of move
    Scope::SubTopic,   // how broad
    0.8,               // urgency
    0.3,               // attention cost
    false,             // has media
    "We could try a Y-shaped key/value split...".to_string(),
);
let e = u.energy(); // urgency * attention_cost scaled
let rest = UtteranceVector::silence(0, Scope::Hold); // zero-energy hold
```

#### `AgentCadence`

```rust
let c = AgentCadence::new("Architect", 90.0);
c.tick_interval();      // ms per tick
c.swing_offset(beat);   // timing offset for swing feel
c.next_onset(tick, energy);  // when agent should speak next
c.should_speak(now, last_spoke);  // boolean decision
```

#### `ConversationTensor`

```rust
let t = ConversationTensor::new(agents, 8);
t.energy_at(tick);     // total energy at a point
t.agent_energy(id, bar); // per-agent energy in a bar
t.bar_energy(bar);     // total energy in a bar
t.tensor_entropy();    // Shannon entropy of utterance distribution
t.adapt_bpm(energy);   // Taoist BPM flow
t.is_conserved(tol);   // conservation invariant check
```

#### `ConversationEngine`

```rust
let mut e = ConversationEngine::new(agents, 8, 500.0);
e.nudge(/* ... */);
e.compute_draft("Architect");   // generates a draft for Architect
e.commit_draft("Architect");    // commits if budget allows
e.tick();                       // advance simulation
e.summary();                    // human-readable state
```

### Pre-built Cadences

| Function | Agent | BPM | Swing | Articulation | Energy | Variation |
|----------|-------|-----|-------|--------------|--------|-----------|
| `architect_cadence()` | Architect | 80 | 0.3 | 0.7 (legato) | 0.6 | 0.4 |
| `implementer_cadence()` | Implementer | 120 | 0.1 | 0.3 (staccato) | 0.8 | 0.2 |
| `critic_cadence()` | Critic | 90 | 0.5 | 0.5 | 0.4 | 0.6 |
| `historian_cadence()` | Historian | 70 | 0.7 | 0.9 (very legato) | 0.3 | 0.8 |

`default_cadences()` returns all four.

---

## How It Works

### The Pipeline

```
Sensor signal (Nudge) → energy_delta → adapt_bpm → compute_draft → commit_draft (if budget allows) → tick
```

1. **Sensor signals arrive** from the channel (LLM token stream, audio sample, file watcher, websocket frame). Each carries an energy delta and a timing hint.
2. **BPM adapts** via a sigmoid mapping: low energy → 60 BPM, high energy → 120 BPM. Smoothed (80/20 blend) to avoid jarring tempo jumps.
3. **Draft computation**: an agent's energy (baseline + nudge modifiers) determines *if* they prepare an utterance. The LLM is called *separately* to compute the body; this layer just decides the timing.
4. **Commit gate**: a draft is only committed if the total energy stays within the conservation budget. Otherwise the draft is held for later.
5. **Tick**: old nudges are cleaned up (older than 4 bars), BPM continues to flow.

### Conservation Invariant

$$\sum_{u \in \text{utterances}} E(u) \leq B_{\text{budget}}$$

The total energy of all committed utterances must not exceed the conversation budget. Drafts that would violate this are held back, freeing up when prior utterances retire.

### Next Onset (When to Speak)

$$t_{\text{next}} = t_{\text{current}} + \left\lfloor 4T \cdot (1 - 0.6e) \cdot j \right\rfloor$$

Where `e` = energy, `T` = ticks per beat, `j` = variation jitter (deterministic hash).

### T-minus Simulation-First

At each `tick`, every agent's `should_speak` returns a boolean *and* a draft index. If an agent's draft is committed but no sensor confirmation has arrived, the draft sits in the runway. The next `InPocket` nudge fires the draft. This separates *preparation* (deterministic, the rhythm layer) from *transmission* (gated by sensor in the channel).

### Deterministic "Randomness"

The system uses no external RNG. Variation and should_speak decisions come from deterministic hash functions (Knuth multiplicative hash: `tick × 2654435761`). The same conversation state always produces the same output — reproducible conversation.

---

## The Math

### Utterance Energy

$$E(u) = u \cdot a \cdot \begin{cases} 0.5 & \text{if has\_media} \\ 1.0 & \text{otherwise} \end{cases}$$

Where `u` = urgency [0,1] and `a` = attention_cost [0,1].

### BPM Adaptation (Sigmoid Flow)

$$\text{target} = 60 + 60 \cdot \min\!\left(\frac{e}{e + 1},\ 1\right)$$

$$\text{bpm}_{t+1} = 0.8 \cdot \text{bpm}_t + 0.2 \cdot \text{target}$$

The exponential moving average ensures smooth tempo evolution — the system never forces, it flows (*wu-wei*).

### Shannon Entropy

$$H = -\sum_{b=1}^{B} p_b \log_2 p_b, \quad p_b = \frac{u_b}{U}$$

Where $u_b$ = number of utterances in bar $b$, $U$ = total utterances. Maximum entropy (utterances uniformly distributed) = $\log_2 B$ bits.

### Swing Offset

Off-beat positions get delayed:

$$\Delta t_{\text{swing}} = s \cdot \frac{T}{3}$$

Where `s` = swing amount [0, 1] and `T` = ticks per beat. At full swing (1.0), off-beats become triplets.

---

## Relationship to `lau-tensor-midi`

`lau-tensor-midi` is the upstream. This crate is a **specialization** for inter-agent communication:

| Upstream (`lau-tensor-midi`) | This crate (`quilt-conversation`) |
|------------------------------|-----------------------------------|
| `NoteVector` (pitch, velocity, duration, onset, channel) | `UtteranceVector` (intent, scope, urgency, attention_cost, body) |
| `AgentCadence` (rhythmic personality) | `AgentCadence` (same — *time-as-first-class* carries over verbatim) |
| `ConversationTensor` | `ConversationTensor` (renamed from tensor MIDI for clarity) |
| `NudgeType::{Excitement, Pushback, Question, TopicShift, Silence}` | `NudgeType::{..., InPocket, OutOfPocket, Anticipation}` (added sensor-driven pocket detection) |
| `ReactiveImprovEngine` | `ConversationEngine` (draft management clarified) |

The signatures and the math survive intact. What changed is the `Intent` enum (propose/critique/agree/disagree/recall/wait), the `Scope` enum (sub-topic/topic/macro/hold), and the *t-minus* separation between preparation and transmission.

---

## Test Coverage

Run tests:

```bash
cargo test
```

---

## License

MIT
