# G2 continual-improvement pass

Date: 2026-07-11

Status: frozen initial G2 suite pass

## Protocol

The same five related held-out tasks, demonstrations, execution limits, and
candidate budgets were run with semantic mechanism memory enabled and disabled.
Seven repetitions were timed and the median was used. Existing tests measured
candidate executions and the minimum demonstrations required for exact held-out
behavior.

Negative transfer was tested on 100 generated square-law tasks with changed
names and numeric bounds. These tasks have a superficially compatible scalar
signature but are structurally unrelated to the stored magnitude mechanism.
Any incorrect prediction or direct memory acceptance counted as negative
transfer; successful rejection followed by fresh search did not.

## Result

    fresh median candidates:           480
    memory median candidates:            1
    candidate reduction:            99.79%
    fresh median demonstrations:          2
    memory median demonstrations:         1
    demonstration reduction:            50%
    fresh median CPU:           272,558,119 ns
    memory median CPU:              553,242 ns
    observed CPU reduction:            493x
    related exactness fresh:            5/5
    related exactness memory:           5/5
    unrelated exactness:            100/100
    negative transfers:               0/100

Command:

    cargo test --locked -p cairn-memory --test gll2_gate -- --nocapture

## Gate judgment

The frozen suite exceeds every G2 threshold: candidate work improves by more
than 4x, median CPU by more than 2x, demonstrations by more than 30%, accuracy
does not fall, and negative transfer is below 2%.

Timing is machine- and load-dependent, so the raw observed nanoseconds are
evidence for this run rather than a portable performance promise. The margin is
large, and the test enforces only the pre-registered 2x threshold. G3-G6 remain
open under ADR-0007 even though earlier bounded GLL composition and world tests
exist; they must be regenerated through Mechanism IR and the new verifier
boundary.
