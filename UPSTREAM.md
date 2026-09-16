# Upstream — `lau-tensor-midi`

This crate directly extends [`SuperInstance/lau-tensor-midi`](https://github.com/SuperInstance/lau-tensor-midi).
We consider ourselves a **specialization** of that upstream for the
inter-agent communication domain.

## What we kept verbatim

| Upstream | This crate |
|----------|-----------|
| `TensorMidiId` | renamed `ConvId` (more domain-appropriate) |
| `NoteVector::energy()` formula `v/127 * d/480` | replaced with `urgency * attention_cost * media_mult` |
| `AgentCadence` (BPM, swing, articulation, baseline, variation) | verbatim, same fields |
| `ConversationTensor` (bars × agents × notes) | renamed from "tensor" to "conversation tensor" |
| `Nudge` (kind, source, target, strength, tick) | extended `kind` enum |
| `ReactiveImprovEngine` (nudge → adapt_bpm → compute_draft → commit → tick) | renamed to `ConversationEngine`, draft management clarified |
| Deterministic hash for variation (`tick × 2654435761`) | verbatim |
| Taoist sigmoid BPM (`target = 60 + 60 * min(e/(e+1), 1)`) | verbatim |
| Swing offset formula | verbatim |
| Conservation invariant (`ΣE ≤ budget`) | verbatim |

## What changed and why

### `NoteVector` → `UtteranceVector`

The music domain talks about pitches. The conversation domain talks about
*intent*. We added an `Intent` enum (`Propose | Critique | Agree | Disagree |
Recall | Clarify | Wait`) and a `Scope` enum (`SubTopic | Topic | Macro |
Hold`). The energy formula was simplified — we don't need MIDI velocity +
duration normalized to a 480-tick quarter note. We have urgency ×
attention_cost with a media-mult of 0.5 (multi-modal utterances pay half).

### `NudgeType` additions

The original set was `Excitement | Pushback | Question | TopicShift | Silence`.
We added three sensor-driven kinds:

- `InPocket` — sensor confirmed: prepared utterance is good to fire. Tiny
  positive energy delta.
- `OutOfPocket` — sensor confirmed: hold and recompute. Negative energy delta.
- `Anticipation(strength)` — the next beat is yours. Higher strength = stronger
  urgency to prepare.

These are **downhill signals** — they confirm what is already happening, they
don't race the clock.

### `ReactiveImprovEngine` → `ConversationEngine`

Renamed and clarified the draft management. The engine now explicitly tracks
`last_spoke[agent] → tick` so that `should_speak` can be evaluated correctly
even after silence or interruption.

### Pre-built cadences

Same four agents, same personalities. `default_cadences()` returns all four.

## Compatibility

| This crate | Upstream |
|------------|----------|
| `cargo add quilt-conversation` | `cargo add lau-tensor-midi` |

The `serde` dependency footprint matches. Anyone migrating from
`lau-tensor-midi` to `quilt-conversation` will recognize the structure.

## Dependency policy

We do **not** depend on `lau-tensor-midi` as a Cargo dependency (no crates.io
publish yet from upstream). We reimplement the cadence/tensor logic locally
so the package stands alone. Once both crates are on crates.io we can collapse
this duplication behind a Cargo dependency in a follow-up release.

## License

Both crates: MIT.
