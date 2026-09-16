//! # quilt-conversation
//!
//! Inter-agent conversation as a live sequencer.
//! T-minus simulation-first. Sensor-confirmed pocket.
//!
//! A conversation tensor `C ∈ ℝ^{B × A × U}` where `B` = bars,
//! `A` = agents, `U` = utterance-dimensions (intent, scope, urgency,
//! attention_cost, has_media, body).
//!
//! The tensor field encodes the full conversational state.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

// ---------------------------------------------------------------------------
// 1. ConvId — opaque newtype identifier
// ---------------------------------------------------------------------------

/// Opaque identifier for conversation entities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConvId(String);

impl ConvId {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl PartialEq for ConvId {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for ConvId {}

impl Hash for ConvId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl std::fmt::Display for ConvId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ---------------------------------------------------------------------------
// 2. Intent / Scope — utterance semantics
// ---------------------------------------------------------------------------

/// What kind of conversational move the agent is making.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Intent {
    Propose,
    Critique,
    Agree,
    Disagree,
    Recall,
    Clarify,
    Wait,
}

impl std::fmt::Display for Intent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Intent::Propose => "propose",
            Intent::Critique => "critique",
            Intent::Agree => "agree",
            Intent::Disagree => "disagree",
            Intent::Recall => "recall",
            Intent::Clarify => "clarify",
            Intent::Wait => "wait",
        };
        write!(f, "{}", s)
    }
}

/// How broad the utterance's reach is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Scope {
    SubTopic,
    Topic,
    Macro,
    Hold,
}

impl std::fmt::Display for Scope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Scope::SubTopic => "subtopic",
            Scope::Topic => "topic",
            Scope::Macro => "macro",
            Scope::Hold => "hold",
        };
        write!(f, "{}", s)
    }
}

// ---------------------------------------------------------------------------
// 3. UtteranceVector — one conversational event
// ---------------------------------------------------------------------------

/// A single utterance event: intent, scope, urgency, attention cost,
/// has_media (carries a payload or not), and body (the actual words).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UtteranceVector {
    pub intent: Intent,
    pub scope: Scope,
    pub urgency: f32,
    pub attention_cost: f32,
    pub has_media: bool,
    pub body: String,
    pub tick: u64,
    pub agent_id: ConvId,
}

impl UtteranceVector {
    pub fn new(
        intent: Intent,
        scope: Scope,
        urgency: f32,
        attention_cost: f32,
        has_media: bool,
        body: impl Into<String>,
    ) -> Self {
        Self {
            intent,
            scope,
            urgency: urgency.clamp(0.0, 1.0),
            attention_cost: attention_cost.clamp(0.0, 1.0),
            has_media,
            body: body.into(),
            tick: 0,
            agent_id: ConvId::new("anon"),
        }
    }

    /// Energy carried by this utterance.
    pub fn energy(&self) -> f32 {
        let media_mult = if self.has_media { 0.5 } else { 1.0 };
        self.urgency * self.attention_cost * media_mult
    }

    /// A zero-energy hold (the agent is reserving bandwidth but not speaking).
    pub fn silence(urgency: f32, scope: Scope) -> Self {
        Self::new(Intent::Wait, scope, urgency, 0.0, false, "")
    }

    pub fn with_tick(mut self, t: u64) -> Self {
        self.tick = t;
        self
    }

    pub fn with_agent(mut self, id: ConvId) -> Self {
        self.agent_id = id;
        self
    }
}

// ---------------------------------------------------------------------------
// 4. AgentCadence — rhythmic personality
// ---------------------------------------------------------------------------

/// The rhythmic fingerprint of an agent: BPM, swing, articulation,
/// baseline energy, variation. Determines *when* an agent speaks; the
/// content layer (LLM) decides *what* they say.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCadence {
    pub agent_id: ConvId,
    pub bpm: f32,
    pub swing: f32,
    pub articulation: f32,
    pub baseline_energy: f32,
    pub variation: f32,
}

impl AgentCadence {
    pub fn new(name: impl Into<String>, bpm: f32) -> Self {
        Self {
            agent_id: ConvId::new(name),
            bpm: bpm.clamp(30.0, 240.0),
            swing: 0.3,
            articulation: 0.5,
            baseline_energy: 0.5,
            variation: 0.4,
        }
    }

    pub fn with_swing(mut self, s: f32) -> Self {
        self.swing = s.clamp(0.0, 1.0);
        self
    }
    pub fn with_articulation(mut self, a: f32) -> Self {
        self.articulation = a.clamp(0.0, 1.0);
        self
    }
    pub fn with_baseline_energy(mut self, e: f32) -> Self {
        self.baseline_energy = e.clamp(0.0, 1.0);
        self
    }
    pub fn with_variation(mut self, v: f32) -> Self {
        self.variation = v.clamp(0.0, 1.0);
        self
    }

    /// Milliseconds per tick at current BPM. (4 ticks per beat at 120 BPM = 125ms.)
    pub fn tick_interval(&self) -> f32 {
        60_000.0 / (self.bpm.max(1.0) * 4.0)
    }

    /// Timing offset in ticks for swing feel at a given beat position.
    pub fn swing_offset(&self, beat: u64) -> f32 {
        if beat % 2 == 1 {
            self.swing * (self.tick_interval() / 3.0) / self.tick_interval()
        } else {
            0.0
        }
    }

    /// When the agent should next speak given current energy and tick.
    pub fn next_onset(&self, current_tick: u64, energy: f32) -> u64 {
        let jitter = deterministic_hash(current_tick);
        let interval = (4.0 * (1.0 - 0.6 * energy) * (0.5 + self.variation * jitter)) as u64;
        current_tick + interval.max(1)
    }

    /// Whether the agent should speak now, given last time they spoke.
    pub fn should_speak(&self, now: u64, last_spoke: u64) -> bool {
        let next = self.next_onset(last_spoke, self.baseline_energy);
        now >= next
    }
}

/// Deterministic hash for reproducible variation (Knuth multiplicative).
pub fn deterministic_hash(tick: u64) -> f32 {
    let h = (tick.wrapping_mul(2_654_435_761)) as u64;
    ((h % 1000) as f32) / 1000.0
}

// ---------------------------------------------------------------------------
// 5. ConversationTensor — bars × agents × utterances
// ---------------------------------------------------------------------------

/// Tensor field holding the entire conversation state.
/// Shape: utterances[agent_idx][bar] = Vec<UtteranceVector>
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationTensor {
    pub bars: usize,
    pub agents: Vec<ConvId>,
    pub utterances: Vec<Vec<Vec<UtteranceVector>>>,
    pub energy_total: f32,
    pub energy_by_bar: Vec<f32>,
    pub energy_by_agent: HashMap<String, Vec<f32>>,
    pub bpm: f32,
}

impl ConversationTensor {
    pub fn new(agents: Vec<ConvId>, bars: usize) -> Self {
        let energy_by_agent = agents
            .iter()
            .map(|a| (a.as_str().to_string(), vec![0.0; bars]))
            .collect::<HashMap<_, _>>();

        let n = agents.len();
        Self {
            bars,
            agents,
            utterances: (0..n).map(|_| vec![Vec::new(); bars]).collect(),
            energy_total: 0.0,
            energy_by_bar: vec![0.0; bars],
            energy_by_agent,
            bpm: 80.0,
        }
    }

    /// Re-initialize the utterance tensor if dimensions changed.
    pub fn init_utterances(&mut self, n_agents: usize) {
        self.utterances = (0..n_agents).map(|_| vec![Vec::new(); self.bars]).collect();
    }

    /// Total energy at a given tick (across all bars).
    pub fn energy_at(&self, tick: u64) -> f32 {
        let bar = (tick as usize) % self.bars;
        self.energy_by_bar[bar]
    }

    /// Per-agent energy within a bar.
    pub fn agent_energy(&self, id: &ConvId, bar: usize) -> f32 {
        self.energy_by_agent
            .get(id.as_str())
            .and_then(|v| v.get(bar))
            .copied()
            .unwrap_or(0.0)
    }

    /// Total energy in a bar.
    pub fn bar_energy(&self, bar: usize) -> f32 {
        self.energy_by_bar.get(bar).copied().unwrap_or(0.0)
    }

    /// Shannon entropy of utterance distribution across bars.
    pub fn tensor_entropy(&self) -> f32 {
        let total: usize = self.utterances.iter()
            .flat_map(|agent_bars| agent_bars.iter())
            .map(|bar| bar.len())
            .sum();
        if total == 0 {
            return 0.0;
        }
        let mut h = 0.0;
        for agent_bars in &self.utterances {
            for bar in agent_bars {
                let p = bar.len() as f32 / total as f32;
                if p > 0.0 {
                    h -= p * p.log2();
                }
            }
        }
        h
    }

    /// Adapt BPM (Taoist sigmoid flow).
    pub fn adapt_bpm(&mut self, energy: f32) {
        let target = 60.0 + 60.0 * (energy / (energy + 1.0)).min(1.0);
        self.bpm = 0.8 * self.bpm + 0.2 * target;
    }

    /// Check if total energy stays within budget.
    pub fn is_conserved(&self, budget: f32, tol: f32) -> bool {
        self.energy_total <= budget + tol
    }

    /// Append an utterance to the tensor and update energy bookkeeping.
    pub fn push(&mut self, agent_idx: usize, bar: usize, u: UtteranceVector) {
        let e = u.energy();
        if bar < self.bars && agent_idx < self.utterances.len() {
            self.utterances[agent_idx][bar].push(u);
            self.energy_by_bar[bar] += e;
            if let Some(agent_id) = self.agents.get(agent_idx) {
                if let Some(v) = self.energy_by_agent.get_mut(agent_id.as_str()) {
                    v[bar] += e;
                }
            }
            self.energy_total += e;
        }
    }
}

// ---------------------------------------------------------------------------
// 6. Nudge — reactive signal
// ---------------------------------------------------------------------------

/// A reactive signal that perturbs the conversation field. Arrive from
/// sensors (audio, tokens, websocket frames) or the system itself.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum NudgeType {
    Excitement(f32),
    Pushback(f32),
    Question(f32),
    TopicShift(f32),
    Silence(f32),
    /// Sensor confirmed: the prepared utterance is in the pocket — fire it.
    InPocket,
    /// Sensor confirmed: you're out of pocket — hold, recompute.
    OutOfPocket,
    /// Next beat is yours; prepare your draft.
    Anticipation(f32),
}

impl NudgeType {
    /// Energy delta introduced by this nudge.
    pub fn energy_delta(&self) -> f32 {
        match self {
            NudgeType::Excitement(v) => *v,
            NudgeType::Pushback(v) => -*v * 0.5,
            NudgeType::Question(v) => *v * 0.3,
            NudgeType::TopicShift(v) => *v * 0.2,
            NudgeType::Silence(v) => -*v * 0.4,
            NudgeType::InPocket => 0.1,
            NudgeType::OutOfPocket => -0.2,
            NudgeType::Anticipation(v) => *v * 0.5,
        }
    }

    /// Effect on cadence (swing, articulation).
    pub fn cadence_effect(&self) -> f32 {
        match self {
            NudgeType::Excitement(_) => 0.1,
            NudgeType::Pushback(v) => -0.05 * v,
            NudgeType::Question(_) => 0.05,
            NudgeType::TopicShift(_) => 0.2,
            NudgeType::Silence(_) => -0.1,
            NudgeType::InPocket => 0.0,
            NudgeType::OutOfPocket => 0.0,
            NudgeType::Anticipation(_) => 0.05,
        }
    }
}

/// A nudge with metadata: source, target, strength, and arrival time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Nudge {
    pub kind: NudgeType,
    pub source: String,
    pub target: Option<String>,
    pub strength: f32,
    pub tick: u64,
}

impl Nudge {
    pub fn new(kind: NudgeType, source: impl Into<String>, target: Option<String>, strength: f32, tick: u64) -> Self {
        Self {
            kind,
            source: source.into(),
            target,
            strength: strength.clamp(0.0, 1.0),
            tick,
        }
    }
}

// ---------------------------------------------------------------------------
// 7. Draft — an utterance prepared but not yet committed
// ---------------------------------------------------------------------------

/// A draft utterance awaiting commit. Contains the candidate body and
/// the agent's prepared intent/scope/urgency profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Draft {
    pub agent: ConvId,
    pub utterance: UtteranceVector,
    pub prepared_at_tick: u64,
}

impl Draft {
    pub fn new(agent: ConvId, utterance: UtteranceVector, tick: u64) -> Self {
        Self { agent, utterance, prepared_at_tick: tick }
    }
}

// ---------------------------------------------------------------------------
// 8. ConversationEngine — the full system
// ---------------------------------------------------------------------------

/// The full system: tensor + nudges + drafts + budget.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationEngine {
    pub tensor: ConversationTensor,
    pub cadences: HashMap<String, AgentCadence>,
    pub nudges: Vec<Nudge>,
    pub drafts: HashMap<String, Draft>,
    pub budget: f32,
    pub tick: u64,
    pub last_spoke: HashMap<String, u64>,
}

impl ConversationEngine {
    pub fn new(cadences: Vec<AgentCadence>, bars: usize, budget: f32) -> Self {
        let agents: Vec<ConvId> = cadences.iter().map(|c| c.agent_id.clone()).collect();
        let cadence_map = cadences.iter().map(|c| (c.agent_id.as_str().to_string(), c.clone())).collect();
        let mut tensor = ConversationTensor::new(agents, bars);
        tensor.init_utterances(cadences.len());
        Self {
            tensor,
            cadences: cadence_map,
            nudges: Vec::new(),
            drafts: HashMap::new(),
            budget,
            tick: 0,
            last_spoke: HashMap::new(),
        }
    }

    /// Submit a nudge.
    pub fn nudge(&mut self, n: Nudge) {
        self.nudges.push(n);
    }

    /// Compute a draft for an agent. The content/body is supplied by the
    /// caller (typically an LLM call); this just decides *whether* to
    /// prepare one given the agent's cadence and the current tensor energy.
    pub fn compute_draft(&mut self, agent: &str) {
        let cadence = match self.cadences.get(agent) {
            Some(c) => c.clone(),
            None => return,
        };

        // Effective energy: baseline + nudge influence
        let nudge_e: f32 = self.nudges.iter()
            .filter(|n| n.source == agent || n.target.as_deref() == Some(agent))
            .map(|n| n.kind.energy_delta() * n.strength)
            .sum();
        let energy = (cadence.baseline_energy + nudge_e).clamp(0.0, 1.0);

        // Should this agent prepare a draft right now?
        let last = *self.last_spoke.get(agent).unwrap_or(&0);
        let should = cadence.should_speak(self.tick, last);

        if should {
            // Body is supplied externally in real use; here we make a token
            // placeholder so the engine has a draft to commit.
            let body = format!("[draft from {} at tick {} | energy {:.2}]", agent, self.tick, energy);
            let u = UtteranceVector::new(Intent::Propose, Scope::Topic, energy, 0.3, false, body)
                .with_tick(self.tick)
                .with_agent(cadence.agent_id.clone());
            self.drafts.insert(agent.to_string(),
                Draft::new(cadence.agent_id.clone(), u, self.tick));
        }
    }

    /// Commit a draft if the budget permits.
    pub fn commit_draft(&mut self, agent: &str) -> bool {
        let draft = match self.drafts.remove(agent) {
            Some(d) => d,
            None => return false,
        };

        let e = draft.utterance.energy();
        if self.tensor.energy_total + e > self.budget {
            // Reinsert draft; budget would be blown.
            self.drafts.insert(agent.to_string(), draft);
            return false;
        }

        let bar = (self.tick as usize) % self.tensor.bars;
        let agent_idx = self.tensor.agents.iter()
            .position(|a| a.as_str() == agent);
        if let Some(idx) = agent_idx {
            self.tensor.push(idx, bar, draft.utterance);
            self.last_spoke.insert(agent.to_string(), self.tick);
            // Wake BPM based on this utterance's urgency
            self.tensor.adapt_bpm(e);
            return true;
        }
        false
    }

    /// Advance the simulation by one tick.
    pub fn tick(&mut self) {
        // Clean up old nudges (>4 bars old).
        let cutoff = self.tick.saturating_sub(4 * self.tensor.bars as u64);
        self.nudges.retain(|n| n.tick >= cutoff);

        self.tick = self.tick.wrapping_add(1);
    }

    /// Human-readable state.
    pub fn summary(&self) -> String {
        format!(
            "🗨  Conversation Field: {} agents, {} utterances pending\n\
             ⚡ Energy: {:.3} / budget {:.3}\n\
             🕰  Cadence (BPM): {:.1}\n\
             🌡  Entropy: {:.3} bits\n\
             💫 Nudges: {} active\n\
             📝 Drafts: {} pending\n\
             📍 Tick: {}",
            self.tensor.agents.len(),
            self.drafts.len(),
            self.tensor.energy_total,
            self.budget,
            self.tensor.bpm,
            self.tensor.tensor_entropy(),
            self.nudges.len(),
            self.drafts.len(),
            self.tick
        )
    }
}

// ---------------------------------------------------------------------------
// 9. Pre-built cadences
// ---------------------------------------------------------------------------

pub fn architect_cadence() -> AgentCadence {
    AgentCadence::new("Architect", 80.0)
        .with_swing(0.3)
        .with_articulation(0.7)
        .with_baseline_energy(0.6)
        .with_variation(0.4)
}

pub fn implementer_cadence() -> AgentCadence {
    AgentCadence::new("Implementer", 120.0)
        .with_swing(0.1)
        .with_articulation(0.3)
        .with_baseline_energy(0.8)
        .with_variation(0.2)
}

pub fn critic_cadence() -> AgentCadence {
    AgentCadence::new("Critic", 90.0)
        .with_swing(0.5)
        .with_articulation(0.5)
        .with_baseline_energy(0.4)
        .with_variation(0.6)
}

pub fn historian_cadence() -> AgentCadence {
    AgentCadence::new("Historian", 70.0)
        .with_swing(0.7)
        .with_articulation(0.9)
        .with_baseline_energy(0.3)
        .with_variation(0.8)
}

pub fn default_cadences() -> Vec<AgentCadence> {
    vec![
        architect_cadence(),
        implementer_cadence(),
        critic_cadence(),
        historian_cadence(),
    ]
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conv_id_basic() {
        let id = ConvId::new("agent-1");
        assert_eq!(id.as_str(), "agent-1");
        assert_eq!(format!("{}", id), "agent-1");
    }

    #[test]
    fn test_utterance_vector_energy() {
        let u = UtteranceVector::new(Intent::Propose, Scope::Topic, 0.5, 0.6, false, "");
        assert!((u.energy() - 0.3).abs() < 1e-5);

        let u2 = UtteranceVector::new(Intent::Propose, Scope::Topic, 0.5, 0.6, true, "");
        assert!((u2.energy() - 0.15).abs() < 1e-5);
    }

    #[test]
    fn test_utterance_silence() {
        let u = UtteranceVector::silence(0.0, Scope::Hold);
        assert_eq!(u.energy(), 0.0);
        assert_eq!(u.intent, Intent::Wait);
    }

    #[test]
    fn test_agent_cadence_tick_interval() {
        let c = AgentCadence::new("a", 120.0);
        // 120 BPM = 0.5s per beat = 125ms per tick (4 ticks/beat)
        assert!((c.tick_interval() - 125.0).abs() < 0.1);
    }

    #[test]
    fn test_agent_cadence_should_speak() {
        let c = AgentCadence::new("a", 60.0);
        assert!(!c.should_speak(80, 90));  // 80 < next_onset(~91) — not yet
        assert!(c.should_speak(100, 90));  // 100 >= 91 — speak
    }

    #[test]
    fn test_deterministic_hash() {
        let h1 = deterministic_hash(42);
        let h2 = deterministic_hash(42);
        let h3 = deterministic_hash(43);
        assert_eq!(h1, h2);
        assert_ne!(h1, h3);
        assert!(h1 >= 0.0 && h1 < 1.0);
    }

    #[test]
    fn test_tensor_init_and_energy() {
        let agents = vec![ConvId::new("a"), ConvId::new("b")];
        let mut t = ConversationTensor::new(agents, 8);
        t.init_utterances(2);
        assert_eq!(t.bar_energy(0), 0.0);
        assert_eq!(t.energy_total, 0.0);
    }

    #[test]
    fn test_tensor_push_updates_energy() {
        let agents = vec![ConvId::new("a"), ConvId::new("b")];
        let mut t = ConversationTensor::new(agents, 8);
        t.init_utterances(2);
        let u = UtteranceVector::new(Intent::Propose, Scope::Topic, 0.5, 0.4, false, "");
        t.push(0, 0, u);
        assert!(t.energy_total > 0.0);
        assert_eq!(t.bar_energy(0), 0.5 * 0.4);
    }

    #[test]
    fn test_tensor_entropy_empty_is_zero() {
        let agents = vec![ConvId::new("a")];
        let mut t = ConversationTensor::new(agents, 8);
        t.init_utterances(1);
        assert_eq!(t.tensor_entropy(), 0.0);
    }

    #[test]
    fn test_tensor_adapt_bpm() {
        let agents = vec![ConvId::new("a")];
        let mut t = ConversationTensor::new(agents, 8);
        t.init_utterances(1);
        let initial = t.bpm;
        t.adapt_bpm(10.0); // high energy should push toward 120
        assert!(t.bpm > initial); // BPM increased
        assert!(t.bpm >= 60.0 && t.bpm <= 120.0);
    }

    #[test]
    fn test_tensor_is_conserved() {
        let agents = vec![ConvId::new("a")];
        let mut t = ConversationTensor::new(agents, 8);
        t.init_utterances(1);
        t.energy_total = 100.0;
        assert!(t.is_conserved(200.0, 10.0));
        assert!(!t.is_conserved(50.0, 10.0));
    }

    #[test]
    fn test_nudge_energy_delta() {
        assert!(NudgeType::Excitement(0.8).energy_delta() > 0.0);
        assert!(NudgeType::Pushback(0.5).energy_delta() < 0.0);
        assert!(NudgeType::Question(0.5).energy_delta() > 0.0);
        assert!(NudgeType::Silence(0.5).energy_delta() < 0.0);
    }

    #[test]
    fn test_nudge_cadence_effect() {
        assert!(NudgeType::TopicShift(0.5).cadence_effect() > 0.0);
        assert!(NudgeType::Silence(0.5).cadence_effect() < 0.0);
        assert_eq!(NudgeType::InPocket.cadence_effect(), 0.0);
        assert_eq!(NudgeType::OutOfPocket.cadence_effect(), 0.0);
    }

    #[test]
    fn test_engine_new() {
        let e = ConversationEngine::new(default_cadences(), 8, 500.0);
        assert_eq!(e.tensor.agents.len(), 4);
        assert_eq!(e.budget, 500.0);
        assert_eq!(e.tick, 0);
    }

    #[test]
    fn test_engine_nudge_adds() {
        let mut e = ConversationEngine::new(default_cadences(), 8, 500.0);
        assert_eq!(e.nudges.len(), 0);
        e.nudge(Nudge::new(NudgeType::Excitement(0.5), "Architect", None, 0.7, 0));
        assert_eq!(e.nudges.len(), 1);
    }

    #[test]
    fn test_engine_compute_and_commit() {
        let mut e = ConversationEngine::new(default_cadences(), 8, 500.0);
        e.nudge(Nudge::new(NudgeType::Excitement(0.5), "Architect", None, 0.7, 0));
        e.nudge(Nudge::new(NudgeType::Anticipation(0.9), "Architect", None, 0.9, 0));
        e.tick = 200; // way past next_onset for any cadence
        e.compute_draft("Architect");
        assert!(e.drafts.contains_key("Architect"));
        let committed = e.commit_draft("Architect");
        assert!(committed);
        assert!(e.tensor.energy_total > 0.0);
    }

    #[test]
    fn test_engine_budget_enforcement() {
        let mut e = ConversationEngine::new(default_cadences(), 8, 10.0); // tiny budget
        for i in 0..100 {
            e.tick = i;
            e.compute_draft("Architect");
            e.commit_draft("Architect");
        }
        assert!(e.tensor.energy_total <= 10.0 + 0.01); // budget held
    }

    #[test]
    fn test_engine_tick_clears_old_nudges() {
        let mut e = ConversationEngine::new(default_cadences(), 8, 500.0);
        e.nudge(Nudge::new(NudgeType::Excitement(0.5), "x", None, 0.7, 0));
        assert_eq!(e.nudges.len(), 1);
        for _ in 0..100 {
            e.tick();
        }
        // 100 ticks > 4 bars (32 ticks), so nudge should be cleared
        assert_eq!(e.nudges.len(), 0);
    }

    #[test]
    fn test_engine_summary_format() {
        let e = ConversationEngine::new(default_cadences(), 8, 500.0);
        let s = e.summary();
        assert!(s.contains("Conversation Field"));
        assert!(s.contains("Tick"));
        assert!(s.contains("budget"));
    }

    #[test]
    fn test_in_pocket_nudge() {
        let n = Nudge::new(NudgeType::InPocket, "Architect", None, 0.3, 42);
        assert_eq!(n.kind.energy_delta(), 0.1);
        assert_eq!(n.kind.cadence_effect(), 0.0);
    }

    #[test]
    fn test_out_of_pocket_nudge() {
        let n = Nudge::new(NudgeType::OutOfPocket, "Architect", None, 0.3, 42);
        assert_eq!(n.kind.energy_delta(), -0.2);
    }

    #[test]
    fn test_anticipation_nudge() {
        let n = Nudge::new(NudgeType::Anticipation(0.9), "Architect", None, 0.9, 42);
        assert!(n.kind.energy_delta() > 0.0);
    }

    #[test]
    fn test_full_pipeline() {
        let mut e = ConversationEngine::new(default_cadences(), 8, 500.0);
        e.nudge(Nudge::new(NudgeType::Excitement(0.7), "Architect",
            Some("Implementer".into()), 0.9, 0));
        e.nudge(Nudge::new(NudgeType::InPocket, "Architect", None, 0.3, 0));
        for agent in &["Architect", "Implementer", "Critic", "Historian"] {
            e.compute_draft(agent);
            e.commit_draft(agent);
        }
        for _ in 0..960 {
            e.tick();
        }
        let s = e.summary();
        assert!(s.contains("Tick: 960"));
    }

    #[test]
    fn test_prebuilt_cadences() {
        assert_eq!(architect_cadence().agent_id.as_str(), "Architect");
        assert_eq!(implementer_cadence().agent_id.as_str(), "Implementer");
        assert_eq!(critic_cadence().agent_id.as_str(), "Critic");
        assert_eq!(historian_cadence().agent_id.as_str(), "Historian");
        assert_eq!(default_cadences().len(), 4);
    }
}
