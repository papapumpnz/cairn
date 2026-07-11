//! Structured behavioural distillation with no teacher runtime dependency.
//!
//! Teacher packets are untrusted proposal sources. Compilation produces typed
//! language lessons and bounded invention problems. Independent local replay
//! produces an opaque verified packet that can enter a peer- and transfer-gated
//! shared registry. Raw prose, model state, and benchmark answers are never
//! accepted payloads.

use cairn_core::Hash32;
use cairn_invent::{
    BindingExample, DefinitionInventor, DefinitionProposal, InventionError, InventionLimits,
    InventionProblem,
};
use cairn_language::{
    GoalExpression, LanguageCompiler, LanguageLesson, OutputSpecification, ProgramOperation,
    ProgrammingLanguage,
};
use cairn_mechanism::{
    DerivationCertificate, InventionEvidence, Mechanism, Signature,
    SupportGeometry as MechanismSupport,
};
use cairn_memory::{
    MechanismStore, PromotionError, PromotionRunner, SupportGeometry as MemorySupport,
};
use cairn_rkl::{
    Binder, Effect, Expr, IntRange, ReferenceTheoryExecutor, RuntimeEnvironment, TheoryExecLimits,
    TheoryExecutionRequest, Type, TypeEnvironment, Value, VarName,
};
use cairn_theory::{
    CollectionInventionError, CollectionMechanismInventor, CollectionTrace,
    LearnedCollectionMechanism,
};
use cairn_verifier::PromotionVerifier;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const DISTILLATION_SEMANTICS_VERSION: &str = "1.2.0";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TeacherPacket {
    pub packet_version: String,
    pub domain: String,
    pub origin: TeacherOrigin,
    #[serde(default)]
    pub semantic_intents: Vec<TeacherIntent>,
    #[serde(default)]
    pub mechanism_lessons: Vec<ScalarMechanismLesson>,
    #[serde(default)]
    pub collection_mechanism_lessons: Vec<CollectionMechanismLesson>,
    #[serde(default)]
    pub knowledge_claims: Vec<TeacherKnowledgeClaim>,
    #[serde(default)]
    pub procedures: Vec<TeacherProcedure>,
    #[serde(default)]
    pub counterexamples: Vec<TeacherCounterexample>,
    #[serde(default)]
    pub uncertainties: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TeacherOrigin {
    pub kind: String,
    pub identifier: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TeacherIntent {
    pub utterance: String,
    pub intent: String,
    pub language: String,
    pub operation: String,
    #[serde(default)]
    pub held_out_utterances: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ScalarMechanismLesson {
    pub name: String,
    pub parameters: Vec<String>,
    pub examples: Vec<ScalarExample>,
    #[serde(default)]
    pub literals: Vec<i64>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ScalarExample {
    pub inputs: BTreeMap<String, i64>,
    pub output: i64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CollectionMechanismLesson {
    pub name: String,
    pub maximum_cardinality: u32,
    pub episodes: Vec<CollectionEpisode>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CollectionEpisode {
    pub episode: String,
    pub inputs: Vec<i64>,
    pub outputs: Vec<i64>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TeacherKnowledgeClaim {
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub verification_method: String,
    pub evidence: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TeacherProcedure {
    pub name: String,
    pub goal: String,
    pub parameters: Vec<String>,
    pub steps: Vec<TeacherProcedureStep>,
    pub preconditions: Vec<String>,
    pub postconditions: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TeacherProcedureStep {
    pub id: String,
    pub operation: String,
    pub depends_on: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TeacherCounterexample {
    pub mechanism_name: String,
    pub inputs: BTreeMap<String, i64>,
    pub forbidden_output: i64,
    pub reason: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProvisionalKnowledgeClaim {
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub verification_method: String,
    pub evidence: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedProcedureGraph {
    pub name: String,
    pub goal: String,
    pub parameters: Vec<String>,
    pub steps: Vec<TeacherProcedureStep>,
    pub preconditions: Vec<String>,
    pub postconditions: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum DistillationTrust {
    TeacherProposed,
    LocallyVerified,
    PeerVerified,
    TransferTested,
    SwarmAccepted,
    Revoked,
}

#[derive(Clone, Debug)]
pub struct CompiledTeacherPacket {
    id: Hash32,
    origin: TeacherOrigin,
    language_lessons: Vec<LanguageLesson>,
    language_tests: Vec<(String, GoalExpression)>,
    invention_problems: Vec<InventionProblem>,
    collection_lessons: Vec<CompiledCollectionLesson>,
    knowledge_candidates: Vec<ProvisionalKnowledgeClaim>,
    procedures: Vec<ValidatedProcedureGraph>,
    counterexamples: Vec<TeacherCounterexample>,
    uncertainties: Vec<String>,
}

#[derive(Clone, Debug)]
struct CompiledCollectionLesson {
    traces: Vec<CollectionTrace>,
    maximum_cardinality: u32,
    limits: InventionLimits,
}

impl CompiledTeacherPacket {
    #[must_use]
    pub const fn id(&self) -> Hash32 {
        self.id
    }

    #[must_use]
    pub const fn origin(&self) -> &TeacherOrigin {
        &self.origin
    }

    #[must_use]
    pub fn uncertainties(&self) -> &[String] {
        &self.uncertainties
    }
}

#[derive(Clone, Debug)]
pub struct VerifiedTeacherPacket {
    id: Hash32,
    origin: TeacherOrigin,
    language_lessons: Vec<LanguageLesson>,
    mechanisms: Vec<DefinitionProposal>,
    collection_mechanisms: Vec<LearnedCollectionMechanism>,
    knowledge_candidates: Vec<ProvisionalKnowledgeClaim>,
    procedures: Vec<ValidatedProcedureGraph>,
    verified_language_cases: u64,
    verified_mechanism_examples: u64,
    verified_counterexamples: u64,
}

#[derive(Clone, Debug)]
pub struct PeerVerification {
    artifact: Hash32,
    peer: AgentId,
    replayed_language_cases: u64,
    replayed_mechanism_examples: u64,
}

impl VerifiedTeacherPacket {
    #[must_use]
    pub const fn id(&self) -> Hash32 {
        self.id
    }

    #[must_use]
    pub const fn origin(&self) -> &TeacherOrigin {
        &self.origin
    }

    #[must_use]
    pub fn language_lessons(&self) -> &[LanguageLesson] {
        &self.language_lessons
    }

    #[must_use]
    pub fn mechanisms(&self) -> &[DefinitionProposal] {
        &self.mechanisms
    }

    #[must_use]
    pub fn collection_mechanisms(&self) -> &[LearnedCollectionMechanism] {
        &self.collection_mechanisms
    }

    #[must_use]
    pub const fn verified_language_cases(&self) -> u64 {
        self.verified_language_cases
    }

    #[must_use]
    pub const fn verified_mechanism_examples(&self) -> u64 {
        self.verified_mechanism_examples
    }

    #[must_use]
    pub fn knowledge_candidates(&self) -> &[ProvisionalKnowledgeClaim] {
        &self.knowledge_candidates
    }

    #[must_use]
    pub fn procedures(&self) -> &[ValidatedProcedureGraph] {
        &self.procedures
    }

    #[must_use]
    pub const fn verified_counterexamples(&self) -> u64 {
        self.verified_counterexamples
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DistillationError {
    Json,
    UnsupportedVersion,
    EmptyDomain,
    InvalidOrigin,
    EmptyPacket,
    UnsupportedIntent,
    EmptyUtterance,
    EmptyMechanism,
    DuplicateParameter,
    MissingInput(String),
    UnexpectedInput(String),
    Invention(InventionError),
    CollectionInvention(CollectionInventionError),
    InvalidCollectionLesson,
    CollectionReplay,
    LanguageReplay,
    UnknownArtifact,
    DuplicateArtifact,
    AuthorCannotVerify,
    DuplicatePeer,
    EmptyTransfer,
    Revoked,
    Promotion(PromotionError),
    PersistentAdmission,
    InvalidKnowledgeClaim,
    InvalidProcedure,
    InvalidCounterexample,
    CounterexampleReplay,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct TeacherPacketCompiler;

impl TeacherPacketCompiler {
    /// Decode the closed teacher-packet schema.
    ///
    /// # Errors
    ///
    /// Rejects malformed JSON, unknown fields, or wrong field types.
    pub fn parse(&self, bytes: &[u8]) -> Result<TeacherPacket, DistillationError> {
        serde_json::from_slice(bytes).map_err(|_| DistillationError::Json)
    }

    /// Compile supported packet fields to typed, still-untrusted artifacts.
    ///
    /// # Errors
    ///
    /// Rejects unsupported semantics, missing evidence, or malformed bindings.
    pub fn compile(
        &self,
        packet: &TeacherPacket,
        limits: InventionLimits,
    ) -> Result<CompiledTeacherPacket, DistillationError> {
        if packet.packet_version != "1.0" {
            return Err(DistillationError::UnsupportedVersion);
        }
        if packet.domain.trim().is_empty() {
            return Err(DistillationError::EmptyDomain);
        }
        if packet.origin.kind.trim().is_empty() || packet.origin.identifier.trim().is_empty() {
            return Err(DistillationError::InvalidOrigin);
        }
        if packet.semantic_intents.is_empty()
            && packet.mechanism_lessons.is_empty()
            && packet.collection_mechanism_lessons.is_empty()
            && packet.knowledge_claims.is_empty()
            && packet.procedures.is_empty()
        {
            return Err(DistillationError::EmptyPacket);
        }
        let mut language_lessons = Vec::new();
        let mut language_tests = Vec::new();
        for lesson in &packet.semantic_intents {
            if lesson.utterance.trim().is_empty() {
                return Err(DistillationError::EmptyUtterance);
            }
            let objective = compile_objective(lesson)?;
            language_lessons.push(LanguageLesson {
                utterance: lesson.utterance.clone(),
                objective: objective.clone(),
                requested_output: OutputSpecification::SourceCode,
            });
            language_tests.extend(
                lesson
                    .held_out_utterances
                    .iter()
                    .cloned()
                    .map(|utterance| (utterance, objective.clone())),
            );
        }
        let invention_problems = packet
            .mechanism_lessons
            .iter()
            .map(|lesson| compile_mechanism(lesson, limits))
            .collect::<Result<Vec<_>, _>>()?;
        let collection_lessons = packet
            .collection_mechanism_lessons
            .iter()
            .map(|lesson| compile_collection_lesson(lesson, limits))
            .collect::<Result<Vec<_>, _>>()?;
        let knowledge_candidates = packet
            .knowledge_claims
            .iter()
            .map(compile_knowledge_claim)
            .collect::<Result<Vec<_>, _>>()?;
        let procedures = packet
            .procedures
            .iter()
            .map(compile_procedure)
            .collect::<Result<Vec<_>, _>>()?;
        for counterexample in &packet.counterexamples {
            validate_counterexample(counterexample, &packet.mechanism_lessons)?;
        }
        let id = packet_identity(packet);
        Ok(CompiledTeacherPacket {
            id,
            origin: packet.origin.clone(),
            language_lessons,
            language_tests,
            invention_problems,
            collection_lessons,
            knowledge_candidates,
            procedures,
            counterexamples: packet.counterexamples.clone(),
            uncertainties: packet.uncertainties.clone(),
        })
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DistillationVerifier;

impl DistillationVerifier {
    /// Replay language cases and independently invent every executable lesson.
    ///
    /// # Errors
    ///
    /// Rejects any language mismatch or bounded invention/admission failure.
    pub fn verify(
        &self,
        compiled: CompiledTeacherPacket,
    ) -> Result<VerifiedTeacherPacket, DistillationError> {
        let mut language = LanguageCompiler::new();
        for lesson in &compiled.language_lessons {
            language.install_lesson(lesson.clone());
        }
        for (utterance, expected) in &compiled.language_tests {
            if language.compile(utterance).objective != *expected {
                return Err(DistillationError::LanguageReplay);
            }
        }
        let mut verified_mechanism_examples = 0_u64;
        let mechanisms = compiled
            .invention_problems
            .iter()
            .map(|problem| {
                let outcome = DefinitionInventor
                    .invent(problem)
                    .map_err(DistillationError::Invention)?;
                verified_mechanism_examples = verified_mechanism_examples.saturating_add(
                    u64::try_from(outcome.admission.exact_cross_implementation_replays)
                        .unwrap_or(u64::MAX),
                );
                Ok(outcome.proposal)
            })
            .collect::<Result<Vec<_>, DistillationError>>()?;
        let collection_mechanisms = compiled
            .collection_lessons
            .iter()
            .map(|lesson| {
                let learned = CollectionMechanismInventor
                    .invent(&lesson.traces, lesson.maximum_cardinality, lesson.limits)
                    .map_err(DistillationError::CollectionInvention)?;
                verified_mechanism_examples = verified_mechanism_examples
                    .saturating_add(replay_collection_lesson(&learned, lesson)?);
                Ok(learned)
            })
            .collect::<Result<Vec<_>, DistillationError>>()?;
        let mut verified_counterexamples = 0_u64;
        for counterexample in &compiled.counterexamples {
            let proposal = mechanisms
                .iter()
                .find(|proposal| proposal.definition.name == counterexample.mechanism_name)
                .ok_or(DistillationError::InvalidCounterexample)?;
            let mut inputs = RuntimeEnvironment::new();
            for parameter in &proposal.definition.parameters {
                let value = counterexample
                    .inputs
                    .get(&parameter.name.0)
                    .ok_or(DistillationError::InvalidCounterexample)?;
                inputs.insert(parameter.name.clone(), Value::Int(*value));
            }
            let actual = DefinitionInventor
                .predict(proposal, &inputs, cairn_rkl::ExecLimits::default())
                .map_err(DistillationError::Invention)?;
            if actual == Value::Int(counterexample.forbidden_output) {
                return Err(DistillationError::CounterexampleReplay);
            }
            verified_counterexamples = verified_counterexamples.saturating_add(1);
        }
        Ok(VerifiedTeacherPacket {
            id: compiled.id,
            origin: compiled.origin,
            language_lessons: compiled.language_lessons,
            mechanisms,
            collection_mechanisms,
            knowledge_candidates: compiled.knowledge_candidates,
            procedures: compiled.procedures,
            verified_language_cases: u64::try_from(compiled.language_tests.len())
                .unwrap_or(u64::MAX),
            verified_mechanism_examples,
            verified_counterexamples,
        })
    }

    /// Independently replay a freshly compiled packet under a named peer role
    /// and return an opaque receipt bound to its complete content identity.
    ///
    /// # Errors
    ///
    /// Rejects any language or executable-mechanism replay failure.
    pub fn verify_peer(
        &self,
        compiled: CompiledTeacherPacket,
        peer: AgentId,
    ) -> Result<PeerVerification, DistillationError> {
        let verified = self.verify(compiled)?;
        Ok(PeerVerification {
            artifact: verified.id,
            peer,
            replayed_language_cases: verified.verified_language_cases,
            replayed_mechanism_examples: verified.verified_mechanism_examples,
        })
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct PersistentDistillationPromoter;

impl PersistentDistillationPromoter {
    /// Promote independently invented packet mechanisms through both the
    /// mechanism evidence gate and verifier token boundary, returning a store
    /// containing only the resulting persistent proposal module.
    ///
    /// # Errors
    ///
    /// Rejects insufficient/incompatible source definitions, invalid support,
    /// failed learning evidence, or token application disagreement.
    pub fn promote(
        &self,
        packet: &VerifiedTeacherPacket,
        memory_support: MemorySupport,
        mechanism_support: MechanismSupport,
        evidence: &InventionEvidence,
    ) -> Result<MechanismStore, DistillationError> {
        promote_group(
            packet.mechanisms(),
            memory_support,
            mechanism_support,
            evidence,
        )
    }

    /// Build provisional modules from an ordered curriculum containing two
    /// independently invented definitions per mechanism. Teacher verification
    /// cannot manufacture the measured reuse required for persistence.
    ///
    /// # Errors
    ///
    /// Rejects odd/empty curricula or any pair that is structurally
    /// incompatible under ordinary proposal admission.
    pub fn propose_pairs(
        &self,
        packet: &VerifiedTeacherPacket,
        memory_support: &MemorySupport,
    ) -> Result<MechanismStore, DistillationError> {
        if packet.mechanisms().is_empty() || packet.mechanisms().len() % 2 != 0 {
            return Err(DistillationError::PersistentAdmission);
        }
        let mut aggregate = MechanismStore::new();
        for pair in packet.mechanisms().chunks_exact(2) {
            let module = PromotionRunner
                .promote(pair, memory_support.clone())
                .map_err(DistillationError::Promotion)?;
            aggregate.insert(module);
        }
        Ok(aggregate)
    }
}

fn promote_group(
    proposals: &[DefinitionProposal],
    memory_support: MemorySupport,
    mechanism_support: MechanismSupport,
    evidence: &InventionEvidence,
) -> Result<MechanismStore, DistillationError> {
    let module = PromotionRunner
        .promote(proposals, memory_support)
        .map_err(DistillationError::Promotion)?;
    let definition = module.definition().clone();
    let mechanism = Mechanism {
        signature: Signature {
            inputs: definition
                .parameters
                .iter()
                .map(|parameter| parameter.ty.clone())
                .collect(),
            output: definition.return_type.clone(),
        },
        variables: definition.parameters.clone(),
        entity_definitions: vec![definition.clone()],
        preconditions: Expr::Bool(true),
        transition_rules: Vec::new(),
        invariants: vec![Expr::Bool(true)],
        termination: Expr::Bool(true),
        effects: vec![Effect::Emit(Expr::Call {
            function: definition.name.clone(),
            arguments: definition
                .parameters
                .iter()
                .map(|parameter| Expr::Var(parameter.name.clone()))
                .collect(),
        })],
        support: mechanism_support,
        proof: DerivationCertificate {
            source_proofs: module.lineage().iter().copied().collect(),
            verifier_semantics: "distillation-dual-rkl-replay".to_owned(),
            verified_cases: evidence.held_out_verified_cases,
        },
    };
    let token = PromotionVerifier
        .verify(&mechanism, evidence)
        .map_err(|_| DistillationError::PersistentAdmission)?;
    let id = module.id();
    let mut store = MechanismStore::new();
    store.insert(module);
    if !store.apply_verified_promotion(id, &token) {
        return Err(DistillationError::PersistentAdmission);
    }
    Ok(store)
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct AgentId(pub String);

#[derive(Clone, Debug)]
pub struct SharedKnowledgeArtifact {
    pub id: Hash32,
    pub author: AgentId,
    pub semantics_version: String,
    pub trust: DistillationTrust,
    pub parent_artifacts: Vec<Hash32>,
    pub peer_verifiers: BTreeSet<AgentId>,
    pub transfer_cases: u64,
    pub origin_kind: String,
    packet: VerifiedTeacherPacket,
}

impl SharedKnowledgeArtifact {
    #[must_use]
    pub const fn packet(&self) -> &VerifiedTeacherPacket {
        &self.packet
    }
}

#[derive(Clone, Debug, Default)]
pub struct SharedKnowledgeRegistry {
    artifacts: BTreeMap<Hash32, SharedKnowledgeArtifact>,
}

impl SharedKnowledgeRegistry {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            artifacts: BTreeMap::new(),
        }
    }

    /// Publish a locally verified immutable packet under an author identity.
    ///
    /// # Errors
    ///
    /// Rejects duplicate content identities.
    pub fn publish(
        &mut self,
        packet: VerifiedTeacherPacket,
        author: AgentId,
    ) -> Result<Hash32, DistillationError> {
        let id = packet.id();
        if self.artifacts.contains_key(&id) {
            return Err(DistillationError::DuplicateArtifact);
        }
        self.artifacts.insert(
            id,
            SharedKnowledgeArtifact {
                id,
                author,
                semantics_version: DISTILLATION_SEMANTICS_VERSION.to_owned(),
                trust: DistillationTrust::LocallyVerified,
                parent_artifacts: Vec::new(),
                peer_verifiers: BTreeSet::new(),
                transfer_cases: 0,
                origin_kind: packet.origin.kind.clone(),
                packet,
            },
        );
        Ok(id)
    }

    /// Record one independent peer reproduction.
    ///
    /// # Errors
    ///
    /// Rejects unknown/revoked artifacts, author self-review, or duplicate peers.
    pub fn peer_verify(
        &mut self,
        id: Hash32,
        receipt: PeerVerification,
    ) -> Result<(), DistillationError> {
        let artifact = self
            .artifacts
            .get_mut(&id)
            .ok_or(DistillationError::UnknownArtifact)?;
        if artifact.trust == DistillationTrust::Revoked {
            return Err(DistillationError::Revoked);
        }
        if receipt.artifact != id
            || (receipt.replayed_language_cases == 0 && receipt.replayed_mechanism_examples == 0)
        {
            return Err(DistillationError::UnknownArtifact);
        }
        if receipt.peer == artifact.author {
            return Err(DistillationError::AuthorCannotVerify);
        }
        if !artifact.peer_verifiers.insert(receipt.peer) {
            return Err(DistillationError::DuplicatePeer);
        }
        artifact.trust = DistillationTrust::PeerVerified;
        promote_if_ready(artifact);
        Ok(())
    }

    /// Record exact changed-context transfer cases.
    ///
    /// # Errors
    ///
    /// Rejects empty evidence and unknown or revoked artifacts.
    pub fn record_transfer(
        &mut self,
        id: Hash32,
        exact_cases: u64,
    ) -> Result<(), DistillationError> {
        if exact_cases == 0 {
            return Err(DistillationError::EmptyTransfer);
        }
        let artifact = self
            .artifacts
            .get_mut(&id)
            .ok_or(DistillationError::UnknownArtifact)?;
        if artifact.trust == DistillationTrust::Revoked {
            return Err(DistillationError::Revoked);
        }
        artifact.transfer_cases = artifact.transfer_cases.saturating_add(exact_cases);
        artifact.trust = DistillationTrust::TransferTested;
        promote_if_ready(artifact);
        Ok(())
    }

    #[must_use]
    pub fn accepted(&self, id: Hash32) -> Option<&SharedKnowledgeArtifact> {
        self.artifacts
            .get(&id)
            .filter(|artifact| artifact.trust == DistillationTrust::SwarmAccepted)
    }

    /// Revoke an artifact without deleting its provenance.
    ///
    /// # Errors
    ///
    /// Rejects unknown artifact identities.
    pub fn revoke(&mut self, id: Hash32) -> Result<(), DistillationError> {
        let artifact = self
            .artifacts
            .get_mut(&id)
            .ok_or(DistillationError::UnknownArtifact)?;
        artifact.trust = DistillationTrust::Revoked;
        Ok(())
    }
}

fn promote_if_ready(artifact: &mut SharedKnowledgeArtifact) {
    if artifact.peer_verifiers.len() >= 2 && artifact.transfer_cases > 0 {
        artifact.trust = DistillationTrust::SwarmAccepted;
    }
}

fn compile_knowledge_claim(
    claim: &TeacherKnowledgeClaim,
) -> Result<ProvisionalKnowledgeClaim, DistillationError> {
    if claim.subject.trim().is_empty()
        || claim.predicate.trim().is_empty()
        || claim.object.trim().is_empty()
        || claim.verification_method.trim().is_empty()
        || claim.evidence.is_empty()
        || claim.evidence.iter().any(|item| item.trim().is_empty())
    {
        return Err(DistillationError::InvalidKnowledgeClaim);
    }
    Ok(ProvisionalKnowledgeClaim {
        subject: claim.subject.clone(),
        predicate: claim.predicate.clone(),
        object: claim.object.clone(),
        verification_method: claim.verification_method.clone(),
        evidence: claim.evidence.clone(),
    })
}

fn compile_procedure(
    procedure: &TeacherProcedure,
) -> Result<ValidatedProcedureGraph, DistillationError> {
    if procedure.name.trim().is_empty()
        || procedure.goal.trim().is_empty()
        || procedure.steps.is_empty()
        || procedure
            .preconditions
            .iter()
            .any(|item| item.trim().is_empty())
        || procedure
            .postconditions
            .iter()
            .any(|item| item.trim().is_empty())
    {
        return Err(DistillationError::InvalidProcedure);
    }
    let parameters = procedure.parameters.iter().collect::<BTreeSet<_>>();
    if parameters.len() != procedure.parameters.len() {
        return Err(DistillationError::InvalidProcedure);
    }
    let mut known = BTreeSet::new();
    for step in &procedure.steps {
        if step.id.trim().is_empty()
            || step.operation.trim().is_empty()
            || !known.insert(step.id.clone())
            || step
                .depends_on
                .iter()
                .any(|dependency| !known.contains(dependency))
        {
            return Err(DistillationError::InvalidProcedure);
        }
    }
    Ok(ValidatedProcedureGraph {
        name: procedure.name.clone(),
        goal: procedure.goal.clone(),
        parameters: procedure.parameters.clone(),
        steps: procedure.steps.clone(),
        preconditions: procedure.preconditions.clone(),
        postconditions: procedure.postconditions.clone(),
    })
}

fn validate_counterexample(
    counterexample: &TeacherCounterexample,
    lessons: &[ScalarMechanismLesson],
) -> Result<(), DistillationError> {
    if counterexample.reason.trim().is_empty() {
        return Err(DistillationError::InvalidCounterexample);
    }
    let lesson = lessons
        .iter()
        .find(|lesson| lesson.name == counterexample.mechanism_name)
        .ok_or(DistillationError::InvalidCounterexample)?;
    let expected = lesson.parameters.iter().collect::<BTreeSet<_>>();
    let actual = counterexample.inputs.keys().collect::<BTreeSet<_>>();
    if expected != actual {
        return Err(DistillationError::InvalidCounterexample);
    }
    Ok(())
}

fn compile_objective(lesson: &TeacherIntent) -> Result<GoalExpression, DistillationError> {
    if lesson.intent != "GenerateProgram" || lesson.language != "Python" {
        return Err(DistillationError::UnsupportedIntent);
    }
    let operation = match lesson.operation.as_str() {
        "AddNumbers" => ProgramOperation::AddNumbers,
        "IllustrativeFunction" => ProgramOperation::IllustrativeFunction,
        _ => return Err(DistillationError::UnsupportedIntent),
    };
    Ok(GoalExpression::GenerateProgram {
        language: ProgrammingLanguage::Python,
        operation,
    })
}

fn compile_collection_lesson(
    lesson: &CollectionMechanismLesson,
    limits: InventionLimits,
) -> Result<CompiledCollectionLesson, DistillationError> {
    if lesson.name.trim().is_empty() || lesson.maximum_cardinality == 0 || lesson.episodes.len() < 2
    {
        return Err(DistillationError::InvalidCollectionLesson);
    }
    let mut episode_names = BTreeSet::new();
    let mut cardinalities = BTreeSet::new();
    let mut traces = Vec::with_capacity(lesson.episodes.len());
    for episode in &lesson.episodes {
        if episode.episode.trim().is_empty()
            || !episode_names.insert(episode.episode.clone())
            || episode.inputs.is_empty()
            || episode.inputs.len() != episode.outputs.len()
            || episode.inputs.len() > lesson.maximum_cardinality as usize
            || episode
                .inputs
                .iter()
                .chain(&episode.outputs)
                .any(|value| !(-i64::from(u16::MAX)..=i64::from(u16::MAX)).contains(value))
            || episode.inputs.iter().collect::<BTreeSet<_>>().len() != episode.inputs.len()
            || episode.outputs.iter().collect::<BTreeSet<_>>().len() != episode.outputs.len()
        {
            return Err(DistillationError::InvalidCollectionLesson);
        }
        cardinalities.insert(episode.inputs.len());
        traces.push(CollectionTrace {
            episode: Hash32::digest(format!("{}:{}", lesson.name, episode.episode).as_bytes()),
            inputs: episode.inputs.clone(),
            outputs: episode.outputs.clone(),
        });
    }
    if cardinalities.len() < 2 {
        return Err(DistillationError::InvalidCollectionLesson);
    }
    Ok(CompiledCollectionLesson {
        traces,
        maximum_cardinality: lesson.maximum_cardinality,
        limits,
    })
}

fn replay_collection_lesson(
    learned: &LearnedCollectionMechanism,
    lesson: &CompiledCollectionLesson,
) -> Result<u64, DistillationError> {
    let integer = Type::Int(IntRange {
        lower: -i64::from(u16::MAX),
        upper: i64::from(u16::MAX),
    });
    let collection_type = Type::List {
        element: Box::new(integer.clone()),
        maximum: lesson.maximum_cardinality,
    };
    let mut verified = 0_u64;
    for trace in &lesson.traces {
        let mut types = TypeEnvironment::new();
        types.insert(VarName("collection".to_owned()), collection_type.clone());
        let mut values = RuntimeEnvironment::new();
        values.insert(
            VarName("collection".to_owned()),
            Value::Set {
                element_type: integer.clone(),
                maximum: lesson.maximum_cardinality,
                values: trace.inputs.iter().copied().map(Value::Int).collect(),
            },
        );
        let execution = ReferenceTheoryExecutor
            .execute(
                &learned.theory,
                &TheoryExecutionRequest {
                    types,
                    values,
                    initial_facts: BTreeSet::new(),
                    initial_state: BTreeMap::new(),
                },
                TheoryExecLimits {
                    expression: cairn_rkl::ExecLimits::default(),
                    maximum_bindings: u64::from(lesson.maximum_cardinality),
                    maximum_rule_firings: u64::from(lesson.maximum_cardinality),
                },
            )
            .map_err(|_| DistillationError::CollectionReplay)?;
        let expected = trace
            .outputs
            .iter()
            .copied()
            .map(Value::Int)
            .collect::<BTreeSet<_>>();
        if execution.rendered.facts != expected
            || execution.rule_firings.len() != trace.inputs.len()
        {
            return Err(DistillationError::CollectionReplay);
        }
        verified = verified.saturating_add(u64::try_from(trace.inputs.len()).unwrap_or(u64::MAX));
    }
    Ok(verified)
}

fn compile_mechanism(
    lesson: &ScalarMechanismLesson,
    limits: InventionLimits,
) -> Result<InventionProblem, DistillationError> {
    if lesson.name.trim().is_empty() || lesson.parameters.is_empty() || lesson.examples.is_empty() {
        return Err(DistillationError::EmptyMechanism);
    }
    let names = lesson.parameters.iter().collect::<BTreeSet<_>>();
    if names.len() != lesson.parameters.len() {
        return Err(DistillationError::DuplicateParameter);
    }
    let parameters = lesson
        .parameters
        .iter()
        .map(|name| Binder {
            name: VarName(name.clone()),
            ty: Type::Int(IntRange {
                lower: -1_000_000,
                upper: 1_000_000,
            }),
        })
        .collect::<Vec<_>>();
    let demonstrations = lesson
        .examples
        .iter()
        .map(|example| {
            if let Some(unexpected) = example.inputs.keys().find(|name| !names.contains(name)) {
                return Err(DistillationError::UnexpectedInput(unexpected.clone()));
            }
            let mut inputs = RuntimeEnvironment::new();
            for parameter in &parameters {
                let value = example
                    .inputs
                    .get(&parameter.name.0)
                    .ok_or_else(|| DistillationError::MissingInput(parameter.name.0.clone()))?;
                inputs.insert(parameter.name.clone(), Value::Int(*value));
            }
            Ok(BindingExample {
                inputs,
                output: Value::Int(example.output),
            })
        })
        .collect::<Result<Vec<_>, DistillationError>>()?;
    Ok(InventionProblem {
        definition_name: lesson.name.clone(),
        parameters,
        return_type: Type::Int(IntRange {
            lower: -1_000_000,
            upper: 1_000_000,
        }),
        literals: lesson.literals.iter().copied().map(Value::Int).collect(),
        demonstrations,
        limits,
    })
}

fn packet_identity(packet: &TeacherPacket) -> Hash32 {
    Hash32::digest(&serde_json::to_vec(packet).unwrap_or_default())
}
