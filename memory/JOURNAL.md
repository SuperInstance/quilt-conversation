# Ensign Journal — quilt-conversation

## 2026-09-16 — Crate lifted.

The Case for a conversation-shaped tensor:

A multi-agent system stutters because every participant is *reactive* — they wait for the last token, then rush to compose, send, and wait again. Real humans (and good jazz musicians) operate differently. They pre-position while someone else is talking. They let silence carry the message. They read the *negative space* and call it correctly.

This crate ports `lau-tensor-midi` into the inter-agent communication domain:
- The bar × agent × note tensor becomes bar × agent × utterance.
- `NoteVector` becomes `UtteranceVector` (intent, scope, urgency, attention_cost, body).
- The cadence layer survives verbatim: *time is first-class* is the same engineering claim whether the notes are MIDI pitches or LLM tokens.
- The nudge vocabulary grows: `InPocket`, `OutOfPocket`, `Anticipation` capture the sensor-driven confirmation pattern. The signal flows downhill — never uphill — so agents never race the clock.

`ConversationEngine` runs `nudge → adapt_bpm → compute_draft → commit_draft (if budget) → tick`. The `compute_draft` is the polite decision: based on energy and cadence, *should* this agent prepare an utterance now? The LLM call (or whatever produces the body) is external — the engine doesn't touch it.

The whole package ships at version `0.1.0`. 24 tests pass.

---

*The crab inherits the shell. The forge shapes the steel.*
