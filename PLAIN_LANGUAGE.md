# Plain Language — What `quilt-conversation` actually does

*For developers, product folks, and the curious. No physics, no LLM jargon, no music theory.*

## The problem

When you have multiple AI agents talking to each other (or to you), the
conversation **stutters**. Each agent waits for the other to finish
typing, then races to compose a response, sends it, then waits again.
The result feels mechanical and halting — like a group of people who
have never met each other trying to have a conversation one sentence at
a time.

You see this all the time in chatbots and AI assistants. The system is
*reactive* — it never gets ahead of the user.

## What good conversation looks like

Watch two people who know each other well play a board game, or two
musicians trade **fours** (four bars of jazz improvisation each).

They don't wait. While one is talking, the other is **already
composing** their response. They're not "thinking" — they're running
possibilities in the background. When the first person stops, the
second person is already saying the right next thing. Sometimes they
say nothing at all — and that's the right answer. The silence is the
message.

This is what `quilt-conversation` lets you build into a system of AI
agents.

## The mechanism

The crate models each agent as having a **cadence** — a rhythmic
personality that determines *when* they speak, not *what* they say.

```
AgentCadence {
    bpm              // tempo (30 to 240, like a heartbeat)
    swing            // how loose the timing is (0 = rigid, 1 = very loose)
    articulation     // short and clipped vs long and flowing
    baseline_energy  // how loud they are by default
    variation        // how much randomness in their timing
}
```

A "fast-chatting implementer" might have bpm=120, swing=0.1, low
variation. A "thoughtful historian" might have bpm=70, swing=0.7, high
variation. Just like people.

The library calculates, for each agent, when they're next likely to
speak. It does this with a **deterministic hash** — no random numbers,
just a mathematical rule. Same input = same output, every time. Useful
for testing and reproducibility.

## What the agent actually says

When an agent's turn comes up, the library asks *the agent's content
layer* — typically an LLM (large language model) call — to produce the
actual words. This library **does not write those words**. It only
handles the *timing*.

This separation is important. It means you can:

- Test the rhythm layer without paying for any LLM calls.
- Swap out the content layer for any mechanism (LLM, canned response,
  human override, rule-based system).
- Run the same conversation 100 times and get the same timing pattern.

## The conversation tensor

The library keeps track of the whole conversation in a **tensor** —
just a fancy word for a 3D table:

```
          bar-0  bar-1  bar-2  ...  bar-N
agent-A   [u,u]  [u]    []
agent-B   [u]    []     [u,u]
agent-C   []     [u]    [u]
...
```

Each cell holds the utterances made by that agent in that time
interval. The library can compute:

- **Total energy in a bar** — how much activity is happening at once.
- **Per-agent energy** — how loud each participant is.
- **Shannon entropy** — how evenly distributed the activity is across
  the conversation. High entropy = lots of back-and-forth. Low entropy
  = one person dominating.

## The conservation budget

Every conversation has an **energy budget**. The total energy of all
committed utterances can't exceed this budget. If an agent's draft
would push past the limit, the draft is **held back** — not deleted,
just waiting for someone else to retire their energy first.

This prevents conversations from spiraling into 10-message chains.
Someone always has to back off. The rhythm of natural conversation.

## Sensor nudges

The library understands six kinds of "nudges" — events that arrive
from outside (or from inside the system) and adjust the conversation:

1. **Excitement** — raise the energy and tempo.
2. **Pushback** — lower the energy; someone is disagreeing.
3. **Question** — slightly raise energy; someone needs clarification.
4. **Topic shift** — bigger BPM change; new topic, fresh beat.
5. **Silence** — drain energy; someone is choosing not to speak yet.
6. **InPocket** — sensor says "fire your prepared utterance" (positive).
7. **OutOfPocket** — sensor says "hold, recompute" (negative).
8. **Anticipation** — the next beat is yours; prepare.

The InPocket / OutOfPocket / Anticipation triad is the **t-minus** part.
These don't race the clock. They confirm timing. The agent decides
internally when it's ready, and the sensor confirms that the moment has
arrived. No reactive keep-up.

## Why all this matters

If you build multi-agent systems with this kind of timing discipline,
three things become possible:

1. **Conversations stay coherent** — agents don't pile on top of each
   other or leave long awkward pauses.
2. **Silences become meaningful** — when all agents are quiet, the
   library knows that this is because the moment calls for it, not
   because everyone is loading.
3. **You can scale up** — adding more agents doesn't make the conversation
   more chaotic. Each one has its own cadence; the tensor manages the
   global shape.

## A concrete example

```rust
use quilt_conversation::*;

let mut convo = ConversationEngine::new(
    default_cadences(),  // Architect, Implementer, Critic, Historian
    8,                    // 8 bars
    500.0,                // energy budget
);

// Architect is excited; Implementer feels the influence.
convo.nudge(Nudge::new(
    NudgeType::Excitement(0.7),
    "Architect",
    Some("Implementer".into()),
    0.9,
    0,
));

// Each agent checks: "should I prepare an utterance now?"
for agent in &["Architect", "Implementer", "Critic", "Historian"] {
    convo.compute_draft(agent);
    convo.commit_draft(agent);
}

for _ in 0..960 {
    convo.tick();
}

println!("{}", convo.summary());
```

This gives you:

```
🗨  Conversation Field: 4 agents, 0 utterances pending
⚡ Energy: 6.81 / budget 500.00
🕰  Cadence (BPM): 82.4
🌡  Entropy: 1.832 bits
💫 Nudges: 0 active
📝 Drafts: 0 pending
📍 Tick: 960
```

The system ran 960 ticks (about 5 minutes at 82 BPM). It produced some
utterances, retried others when the budget was tight, and let some
moments pass in productive silence.

## Where the code lives

The crate is split into three layers:

1. **`UtteranceVector`** — one conversation event (intent, scope,
   urgency, attention cost, body).
2. **`AgentCadence`** + **`ConversationTensor`** — the rhythm engine.
3. **`ConversationEngine`** — the system that ties it together, with
   nudge handling, draft management, and budget enforcement.

Each layer is independently testable (24 tests in the suite).

## Who built this, and why

This crate is part of the **SuperInstance** project — a fleet of
AI-related tools that experiment with cellular computation, citation
graphs, and edge-deployed multi-agent systems. We keep the connection
to upstream [`lau-tensor-midi`](https://github.com/SuperInstance/lau-tensor-midi)
explicit in `UPSTREAM.md`, the philosophy in `QUILT.md`, and this file
(`PLAIN_LANGUAGE.md`) for the non-curious.

The MIT license means you can use this in commercial or non-commercial
work without permission, attribution appreciated but not required.
