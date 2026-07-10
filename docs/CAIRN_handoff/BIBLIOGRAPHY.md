# Research and benchmark bibliography

This bibliography records the external material used to choose CAIRN's mechanisms. It is not a claim that the cited systems implement CAIRN or establish that CAIRN will meet its benchmark objectives.

## ARC benchmark specifications

1. ARC Prize Foundation. **ARC-AGI-2**. Official benchmark overview, capability categories, dataset partitions, calibration, pass@2 framing, and efficiency objective.  
   https://arcprize.org/arc-agi/2

2. ARC Prize Foundation. **ARC-AGI-2 official repository**. Public data format, 1,000 training tasks, 120 public evaluation tasks, and task success rule requiring all test inputs to be correct with two trials per test input.  
   https://github.com/arcprize/ARC-AGI-2

3. ARC Prize Foundation. **ARC-AGI-2: A New Challenge for Frontier AI Reasoning Systems**. Technical-report overview emphasizing symbolic interpretation, compositional reasoning, and contextual rule application.  
   https://arcprize.org/blog/arc-agi-2-technical-report

4. ARC Prize Foundation. **ARC-AGI-3**. Official interactive benchmark overview.  
   https://arcprize.org/arc-agi/3

5. ARC Prize Foundation. **ARC-AGI-3 Technical Report**. Interactive environment format, exploration, goal acquisition, world-model formation, planning, memory, and action-efficiency evaluation.  
   https://arcprize.org/media/ARC_AGI_3_Technical_Report.pdf

6. François Chollet. **On the Measure of Intelligence**. arXiv:1911.01547, 2019.  
   https://arxiv.org/abs/1911.01547

## Object-centric ARC systems and generalized planning

7. A. K. et al. **Graphs, Constraints, and Search for the Abstraction and Reasoning Corpus**. arXiv:2210.09880. Object-centric graph abstractions, constraint acquisition, graph transformation, and state hashing.  
   https://arxiv.org/abs/2210.09880

8. Miquel Rodríguez et al. **Generalized Planning for the Abstraction and Reasoning Corpus**. arXiv:2401.07426. Planning programs and pointer-based generalized planning for ARC.  
   https://arxiv.org/abs/2401.07426

## Program synthesis, counterexamples, and semantic search

9. Susmit Jha and Sanjit A. Seshia. **A Theory of Formal Synthesis via Inductive Learning**. arXiv:1505.03953. Formal treatment of oracle-guided and counterexample-guided inductive synthesis.  
   https://arxiv.org/abs/1505.03953

10. Albarghouthi et al. **Syntax-Guided Synthesis** and the SyGuS line of work. Candidate languages, constraints, and solver-guided synthesis.  
    https://sygus.org/

11. Rajeev Alur et al. **Scaling Enumerative Program Synthesis via Divide and Conquer**. STUN: synthesis through unification.  
    https://doi.org/10.1007/978-3-662-54577-5_17

12. Andrew Cropper and Rolf Morel. **Learning programs by learning from failures**. arXiv:2005.02259. Generate, test, and constrain search using generalized information from failed hypotheses.  
    https://arxiv.org/abs/2005.02259

13. Max Willsey, Chandrakana Nandi, Yisu Remy Wang, Oliver Flatt, Zachary Tatlock, and Pavel Panchekha. **egg: Fast and Extensible Equality Saturation**. POPL 2021; arXiv:2004.03082.  
    https://arxiv.org/abs/2004.03082

14. José Meseguer and related attributed-graph-grammar literature. Graph rewriting with attributes and constraints is used as background for CAIRN's typed event semantics; CAIRN does not import an unrestricted graph grammar runtime.

## Analogy, abstraction, and causal learning

15. Brian Falkenhainer, Kenneth D. Forbus, and Dedre Gentner. **The Structure-Mapping Engine: Algorithm and Examples**. Artificial Intelligence 41, 1989/1990. Relational alignment and systematicity in analogy.  
    https://www.qrg.northwestern.edu/papers/Files/smeff2%28searchable%29.pdf

16. Gordon D. Plotkin. **A Note on Inductive Generalization**. Machine Intelligence 5, 1970. Foundational anti-unification and least-general generalization.

17. Ehud Shapiro. **Algorithmic Program Debugging** and related inductive logic programming work. Failure localization and declarative hypothesis refinement.

18. Judea Pearl. **Causality: Models, Reasoning, and Inference**. Cambridge University Press. Intervention semantics and structural causal models.

19. Kevin Murphy. **Probabilistic Machine Learning: Advanced Topics**. MIT Press. Belief updates, model uncertainty, and decision-theoretic experiment selection.

20. **Active Causal Experimentalist (ACE): Learning Intervention Strategies via Direct Preference Optimization**. arXiv:2602.02451. Used only as recent evidence that intervention value depends on the quality of an explicit forward model; CAIRN uses symbolic world theories rather than the paper's neural implementation.  
    https://arxiv.org/abs/2602.02451

## CPU-oriented implementation references

21. Rust project documentation. Deterministic systems implementation, ownership, concurrency, and portable SIMD.  
    https://www.rust-lang.org/learn

22. BLAKE3 specification and implementation. Content-addressed artifacts.  
    https://github.com/BLAKE3-team/BLAKE3

23. RFC 8949. **Concise Binary Object Representation (CBOR)**. Canonical artifact serialization.  
    https://www.rfc-editor.org/rfc/rfc8949

24. CaDiCaL, Kissat, and MiniSat-family documentation. Candidate CPU SAT backends. Any selected solver must be isolated behind the `ConstraintBackend` trait and benchmarked under identical workloads.

## Research-use rule

A citation justifies a mechanism family, not a performance claim. Every CAIRN claim must be supported by its own deterministic test, benchmark artifact, or pre-registered evaluation. External scores may not be copied into CAIRN acceptance evidence.
