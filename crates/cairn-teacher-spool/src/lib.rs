//! Network-free orchestration for external teacher request/response spools.
//!
//! This crate serializes API-shaped request artifacts and imports saved response
//! artifacts. It has no socket, HTTP, process, credential, or provider SDK
//! capability. A human or separately governed external system must move bytes
//! between the spool and a teacher. Imported payloads remain untrusted and must
//! enter `cairn-distill` normally.

use cairn_core::Hash32;
use cairn_curriculum::{DeterminismStatus, LearningFailureCore, RedactionStatus};
use cairn_distill::{TeacherPacket, TeacherPacketCompiler};
use serde_json::{json, Value};

pub const TEACHER_SPOOL_SEMANTICS_VERSION: &str = "1.0.0";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WireProtocol {
    ResponsesSchema,
    MessagesTool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OfflineTeachingJob {
    pub job_name: String,
    pub model_identifier: String,
    pub teaching_contract: String,
    pub curriculum_request: String,
    pub packet_schema: Value,
    pub maximum_output_tokens: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpoolArtifact {
    pub protocol: WireProtocol,
    pub request_hash: Hash32,
    pub request_bytes: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImportedTeacherPacket {
    pub response_hash: Hash32,
    pub packet: TeacherPacket,
    pub packet_bytes: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SpoolError {
    EmptyJob,
    EmptyModel,
    EmptyContract,
    EmptyCurriculum,
    EmptySchema,
    EmptyOutputBound,
    Serialize,
    ResponseJson,
    MissingPayload,
    InvalidPacket,
    UnsafeFailureCore,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct OfflineTeacherOrchestrator;

#[derive(Clone, Copy, Debug, Default)]
pub struct GapDrivenJobBuilder;

impl GapDrivenJobBuilder {
    /// Construct a provider-neutral, benchmark-redacted teaching job from
    /// structural failure cores. The teacher is asked for curricula and
    /// counterexamples, never task solutions.
    ///
    /// # Errors
    ///
    /// Rejects empty cores or any core claiming to contain raw observations or
    /// task identifiers.
    pub fn build(
        &self,
        job_name: &str,
        model_identifier: &str,
        packet_schema: Value,
        cores: &[LearningFailureCore],
        maximum_output_tokens: u32,
    ) -> Result<OfflineTeachingJob, SpoolError> {
        if cores.is_empty()
            || cores
                .iter()
                .any(|core| core.redaction != RedactionStatus::ContentFree)
        {
            return Err(SpoolError::UnsafeFailureCore);
        }
        let redacted_cores =
            serde_json::to_string_pretty(cores).map_err(|_| SpoolError::Serialize)?;
        let curriculum_focus = if cores
            .iter()
            .any(|core| core.determinism == DeterminismStatus::ConflictingCompleteInputs)
        {
            "Current observations are causally insufficient. Prioritize synthetic cross-domain curricula for latent variables, relational role invention, and discriminating evidence; do not propose larger blind formula search."
        } else if cores.iter().all(|core| {
            core.exact_two_parameter_families.is_empty()
                && core.exact_three_parameter_families.is_empty()
        }) {
            "Simple one-, two-, and tested three-role scalar explanations are absent. Prioritize hierarchical entity relations, variable-arity constructors, bounded recursion, and conditional role composition across unrelated domains."
        } else {
            "Some compact anonymous mechanism families explain the evidence. Generate independently varied non-benchmark lessons and hard negatives for those families, while preserving anonymous role rebinding."
        };
        Ok(OfflineTeachingJob {
            job_name: job_name.to_owned(),
            model_identifier: model_identifier.to_owned(),
            teaching_contract: "Return one schema-valid, untrusted teaching packet. Generate non-benchmark synthetic lesson families, counterexamples, variation axes, and candidate mechanisms addressing the redacted structural failure cores. Do not provide benchmark tasks, identifiers, grids, answers, hidden reasoning, or memorized solutions."
                .to_owned(),
            curriculum_request: format!(
                "Construct a general cross-domain curriculum for these redacted failure cores. Require at least two independent families per proposed mechanism and include hard negative cases. {curriculum_focus}\n{redacted_cores}"
            ),
            packet_schema,
            maximum_output_tokens,
        })
    }
}

impl OfflineTeacherOrchestrator {
    /// Serialize a request artifact for manual/offline transport. This method
    /// cannot communicate with a provider.
    ///
    /// # Errors
    ///
    /// Rejects empty job fields, absent schema, zero output bounds, or JSON
    /// serialization failure.
    pub fn export(
        &self,
        job: &OfflineTeachingJob,
        protocol: WireProtocol,
    ) -> Result<SpoolArtifact, SpoolError> {
        validate_job(job)?;
        let request = match protocol {
            WireProtocol::ResponsesSchema => json!({
                "model": job.model_identifier,
                "input": [
                    {"role": "system", "content": job.teaching_contract},
                    {"role": "user", "content": job.curriculum_request}
                ],
                "text": {
                    "format": {
                        "type": "json_schema",
                        "name": "cairn_teacher_packet",
                        "strict": true,
                        "schema": job.packet_schema
                    }
                },
                "max_output_tokens": job.maximum_output_tokens,
                "store": false
            }),
            WireProtocol::MessagesTool => json!({
                "model": job.model_identifier,
                "max_tokens": job.maximum_output_tokens,
                "system": job.teaching_contract,
                "messages": [
                    {"role": "user", "content": job.curriculum_request}
                ],
                "tools": [{
                    "name": "emit_cairn_teacher_packet",
                    "description": "Emit one untrusted CAIRN teacher packet",
                    "input_schema": job.packet_schema
                }],
                "tool_choice": {
                    "type": "tool",
                    "name": "emit_cairn_teacher_packet"
                }
            }),
        };
        let request_bytes =
            serde_json::to_vec_pretty(&request).map_err(|_| SpoolError::Serialize)?;
        Ok(SpoolArtifact {
            protocol,
            request_hash: Hash32::digest(&request_bytes),
            request_bytes,
        })
    }

    /// Import a saved provider response and extract the schema-bound packet.
    /// No response is trusted merely because extraction succeeds.
    ///
    /// # Errors
    ///
    /// Rejects malformed response JSON, absent protocol payload, or a payload
    /// that fails the closed CAIRN packet decoder.
    pub fn import(
        &self,
        response_bytes: &[u8],
        protocol: WireProtocol,
    ) -> Result<ImportedTeacherPacket, SpoolError> {
        let response: Value =
            serde_json::from_slice(response_bytes).map_err(|_| SpoolError::ResponseJson)?;
        let packet_value = match protocol {
            WireProtocol::ResponsesSchema => responses_payload(&response)?,
            WireProtocol::MessagesTool => messages_payload(&response)?,
        };
        let packet_bytes = serde_json::to_vec(&packet_value).map_err(|_| SpoolError::Serialize)?;
        let packet = TeacherPacketCompiler
            .parse(&packet_bytes)
            .map_err(|_| SpoolError::InvalidPacket)?;
        Ok(ImportedTeacherPacket {
            response_hash: Hash32::digest(response_bytes),
            packet,
            packet_bytes,
        })
    }
}

fn validate_job(job: &OfflineTeachingJob) -> Result<(), SpoolError> {
    if job.job_name.trim().is_empty() {
        return Err(SpoolError::EmptyJob);
    }
    if job.model_identifier.trim().is_empty() {
        return Err(SpoolError::EmptyModel);
    }
    if job.teaching_contract.trim().is_empty() {
        return Err(SpoolError::EmptyContract);
    }
    if job.curriculum_request.trim().is_empty() {
        return Err(SpoolError::EmptyCurriculum);
    }
    if !job.packet_schema.is_object() {
        return Err(SpoolError::EmptySchema);
    }
    if job.maximum_output_tokens == 0 {
        return Err(SpoolError::EmptyOutputBound);
    }
    Ok(())
}

fn responses_payload(response: &Value) -> Result<Value, SpoolError> {
    if let Some(value) = response.get("output_parsed") {
        return Ok(value.clone());
    }
    let text = response
        .get("output")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .flat_map(|item| {
            item.get("content")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
        })
        .find_map(|content| content.get("text").and_then(Value::as_str))
        .ok_or(SpoolError::MissingPayload)?;
    serde_json::from_str(text).map_err(|_| SpoolError::InvalidPacket)
}

fn messages_payload(response: &Value) -> Result<Value, SpoolError> {
    response
        .get("content")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .find(|item| item.get("type").and_then(Value::as_str) == Some("tool_use"))
        .and_then(|item| item.get("input"))
        .cloned()
        .ok_or(SpoolError::MissingPayload)
}
