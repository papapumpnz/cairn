# Residual constructor promotion

Date: 2026-07-11

Status: verifier-gated persistent mechanism pass

## Protocol

For each demonstration, the frontend subtracts the explained input state from
the output and identifies created cells. It supplies the learner only generic
row, column, row-count, and column-count bindings plus Boolean created/not
created labels. No diagonal, line, repetition, or paint operator is available.

The task learner searches the generic RKL substrate for an emission condition.
Two independent source families vary field sizes and categorical paint values.
Their executable definitions are independently admitted, structurally compared,
and passed through strict promotion. A third family provides one positive
binding, changes to 7x7 geometry and a new paint value, and uses the persistent
condition to construct its full output.

## Invented mechanism

    emit(row, column, rows, columns) := row == column

The paint remains a rebound effect parameter rather than being embedded in the
predicate.

## Result

    source families:             2
    persistent mechanisms:       1
    fresh candidates:           29
    memory proposals:             1
    examples without memory:     34
    examples with memory:         1
    source residual cells:        8
    held-out geometry:          7x7
    held-out cells exact:       49/49
    changed categorical value:  yes
    mixed-paint negative test:  rejected

Commands:

    cargo test --locked -p cairn-field --test residual_constructor -- --nocapture
    cargo test --locked -p cairn-field --test residual_promotion -- --nocapture

## Claim boundary

This is executable residual-driven constructor invention with verified reuse,
not a named geometry macro. It currently covers a single-valued emission effect
over unchanged field shape. Variable cardinality, copied motifs, recursive
growth, deletion/update residuals, overlapping constructors, and effect
scheduling remain open support dimensions.
