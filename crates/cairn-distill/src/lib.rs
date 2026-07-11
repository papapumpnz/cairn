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
use cairn_rkl::{Binder, IntRange, RuntimeEnvironment, Type, Value, VarName};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const DISTILLATION_SEMANTICS_VERSION: &str = "1.0.0";

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
    uncertainties: Vec<String>,
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
    verified_language_cases: u64,
    verified_mechanism_examples: u64,
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
    pub const fn verified_language_cases(&self) -> u64 {
        self.verified_language_cases
    }

    #[must_use]
    pub const fn verified_mechanism_examples(&self) -> u64 {
        self.verified_mechanism_examples
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
    LanguageReplay,
    UnknownArtifact,
    DuplicateArtifact,
    AuthorCannotVerify,
    DuplicatePeer,
    EmptyTransfer,
    Revoked,
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
        if packet.semantic_intents.is_empty() && packet.mechanism_lessons.is_empty() {
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
        let id = packet_identity(packet);
        Ok(CompiledTeacherPacket {
            id,
            origin: packet.origin.clone(),
            language_lessons,
            language_tests,
            invention_problems,
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
        Ok(VerifiedTeacherPacket {
            id: compiled.id,
            origin: compiled.origin,
            language_lessons: compiled.language_lessons,
            mechanisms,
            verified_language_cases: u64::try_from(compiled.language_tests.len())
                .unwrap_or(u64::MAX),
            verified_mechanism_examples,
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
