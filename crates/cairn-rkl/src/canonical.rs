use crate::{BinaryOp, Definition, Expr, Type, TypedExpr, UnaryOp, VarName, RKL_SEMANTICS_VERSION};
use cairn_core::{CanonicalEncode, CanonicalError, CanonicalValue, Hash32};

/// Canonical semantic form with proof annotations removed.
#[derive(Clone, Debug, PartialEq)]
pub struct CanonicalForm {
    pub value: CanonicalValue,
    pub bytes: Vec<u8>,
    pub semantic_hash: Hash32,
}

pub trait Canonicalizer {
    /// Alpha-normalize binders, normalize commutative/associative operations,
    /// and include the checked type in semantic identity.
    ///
    /// # Errors
    ///
    /// Returns canonical-CBOR errors; checked RKL syntax has no other failure.
    fn canonicalize(&self, expression: &TypedExpr) -> Result<CanonicalForm, CanonicalError>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DefaultCanonicalizer;

impl Canonicalizer for DefaultCanonicalizer {
    fn canonicalize(&self, expression: &TypedExpr) -> Result<CanonicalForm, CanonicalError> {
        let value = tagged(
            "typed",
            vec![
                CanonicalValue::Text(RKL_SEMANTICS_VERSION.to_owned()),
                type_value(expression.ty()),
                expression_value(expression.expression(), &mut Vec::new())?,
            ],
        );
        let bytes = value.encode_canonical()?;
        let semantic_hash = Hash32::digest(&bytes);
        Ok(CanonicalForm {
            value,
            bytes,
            semantic_hash,
        })
    }
}

impl CanonicalEncode for Type {
    fn encode_canonical(&self) -> Result<Vec<u8>, CanonicalError> {
        type_value(self).encode_canonical()
    }
}

impl CanonicalEncode for TypedExpr {
    fn encode_canonical(&self) -> Result<Vec<u8>, CanonicalError> {
        DefaultCanonicalizer
            .canonicalize(self)
            .map(|form| form.bytes)
    }
}

impl CanonicalEncode for Definition {
    fn encode_canonical(&self) -> Result<Vec<u8>, CanonicalError> {
        let mut environment = self
            .parameters
            .iter()
            .map(|binder| binder.name.clone())
            .collect::<Vec<_>>();
        let parameters = self
            .parameters
            .iter()
            .map(|binder| type_value(&binder.ty))
            .collect();
        let recursion = if let Some(bound) = &self.recursion {
            tagged(
                "recursion",
                vec![
                    CanonicalValue::Unsigned(u64::from(bound.maximum_steps)),
                    expression_value(&bound.progress_measure, &mut environment)?,
                ],
            )
        } else {
            CanonicalValue::Null
        };
        tagged(
            "definition",
            vec![
                CanonicalValue::Text(self.name.clone()),
                CanonicalValue::Array(parameters),
                type_value(&self.return_type),
                expression_value(&self.body, &mut environment)?,
                recursion,
            ],
        )
        .encode_canonical()
    }
}

// The exhaustive match is intentionally kept together so every new AST tag
// causes one review-visible canonical-semantics change.
#[allow(clippy::too_many_lines)]
fn expression_value(
    expression: &Expr,
    environment: &mut Vec<VarName>,
) -> Result<CanonicalValue, CanonicalError> {
    match expression {
        Expr::Bool(value) => Ok(tagged("bool", vec![CanonicalValue::Bool(*value)])),
        Expr::Int(value) => Ok(tagged("int", vec![signed(*value)])),
        Expr::Color(value) => Ok(tagged(
            "color",
            vec![CanonicalValue::Unsigned(u64::from(*value))],
        )),
        Expr::Coord2 { row, column } => Ok(tagged(
            "coord2",
            vec![signed(i64::from(*row)), signed(i64::from(*column))],
        )),
        Expr::Var(name) => variable_value(name, environment),
        Expr::Tuple(values) => Ok(tagged(
            "tuple",
            vec![CanonicalValue::Array(
                values
                    .iter()
                    .map(|value| expression_value(value, environment))
                    .collect::<Result<Vec<_>, _>>()?,
            )],
        )),
        Expr::Let {
            binder,
            value,
            body,
        } => {
            let value = expression_value(value, environment)?;
            environment.push(binder.clone());
            let body = expression_value(body, environment)?;
            environment.pop();
            Ok(tagged("let", vec![value, body]))
        }
        Expr::If {
            condition,
            then_branch,
            else_branch,
        } => Ok(tagged(
            "if",
            vec![
                expression_value(condition, environment)?,
                expression_value(then_branch, environment)?,
                expression_value(else_branch, environment)?,
            ],
        )),
        Expr::Unary { op, argument } => Ok(tagged(
            unary_name(*op),
            vec![expression_value(argument, environment)?],
        )),
        Expr::Binary { op, left, right } if op.is_associative() => {
            let mut operands = Vec::new();
            collect_associative(*op, left, &mut operands);
            collect_associative(*op, right, &mut operands);
            let mut values = operands
                .into_iter()
                .map(|value| expression_value(value, environment))
                .collect::<Result<Vec<_>, _>>()?;
            if op.is_commutative() {
                sort_values(&mut values)?;
            }
            Ok(tagged(
                binary_name(*op),
                vec![CanonicalValue::Array(values)],
            ))
        }
        Expr::Binary { op, left, right } => {
            let mut values = vec![
                expression_value(left, environment)?,
                expression_value(right, environment)?,
            ];
            if op.is_commutative() {
                sort_values(&mut values)?;
            }
            Ok(tagged(binary_name(*op), values))
        }
        Expr::EmptySet { element, maximum } => Ok(tagged(
            "empty_set",
            vec![
                type_value(element),
                CanonicalValue::Unsigned(u64::from(*maximum)),
            ],
        )),
        Expr::SingletonSet { value, maximum } => Ok(tagged(
            "singleton_set",
            vec![
                expression_value(value, environment)?,
                CanonicalValue::Unsigned(u64::from(*maximum)),
            ],
        )),
        Expr::Count(value) => Ok(tagged("count", vec![expression_value(value, environment)?])),
        Expr::Call {
            function,
            arguments,
        } => Ok(tagged(
            "call",
            vec![
                CanonicalValue::Text(function.clone()),
                CanonicalValue::Array(
                    arguments
                        .iter()
                        .map(|value| expression_value(value, environment))
                        .collect::<Result<Vec<_>, _>>()?,
                ),
            ],
        )),
        Expr::BoundedFixpoint {
            binder,
            initial,
            step,
            maximum_steps,
            progress_measure,
        } => {
            let initial = expression_value(initial, environment)?;
            environment.push(binder.clone());
            let step = expression_value(step, environment)?;
            let progress = expression_value(progress_measure, environment)?;
            environment.pop();
            Ok(tagged(
                "bounded_fixpoint",
                vec![
                    initial,
                    step,
                    CanonicalValue::Unsigned(u64::from(*maximum_steps)),
                    progress,
                ],
            ))
        }
    }
}

fn variable_value(
    name: &VarName,
    environment: &[VarName],
) -> Result<CanonicalValue, CanonicalError> {
    let position = environment
        .iter()
        .rev()
        .position(|candidate| candidate == name);
    let value = position.map_or_else(
        || Ok(tagged("free", vec![CanonicalValue::Text(name.0.clone())])),
        |index| {
            u64::try_from(index)
                .map(|index| tagged("bound", vec![CanonicalValue::Unsigned(index)]))
                .map_err(|_| CanonicalError::LengthOverflow)
        },
    )?;
    Ok(value)
}

fn collect_associative<'a>(op: BinaryOp, expression: &'a Expr, output: &mut Vec<&'a Expr>) {
    if let Expr::Binary {
        op: nested,
        left,
        right,
    } = expression
    {
        if *nested == op {
            collect_associative(op, left, output);
            collect_associative(op, right, output);
            return;
        }
    }
    output.push(expression);
}

fn sort_values(values: &mut [CanonicalValue]) -> Result<(), CanonicalError> {
    let mut keyed = values
        .iter()
        .cloned()
        .map(|value| Ok((value.encode_canonical()?, value)))
        .collect::<Result<Vec<_>, CanonicalError>>()?;
    keyed.sort_by(|(left, _), (right, _)| {
        left.len().cmp(&right.len()).then_with(|| left.cmp(right))
    });
    for (target, (_, value)) in values.iter_mut().zip(keyed) {
        *target = value;
    }
    Ok(())
}

fn type_value(ty: &Type) -> CanonicalValue {
    match ty {
        Type::Bool => tagged("bool_type", Vec::new()),
        Type::Int(range) => tagged("int_type", vec![signed(range.lower), signed(range.upper)]),
        Type::Color => tagged("color_type", Vec::new()),
        Type::Coord2 => tagged("coord2_type", Vec::new()),
        Type::Vector2 => tagged("vector2_type", Vec::new()),
        Type::Axis => tagged("axis_type", Vec::new()),
        Type::Transform2 => tagged("transform2_type", Vec::new()),
        Type::CellSet { maximum } => bounded_type("cell_set_type", *maximum),
        Type::Shape => tagged("shape_type", Vec::new()),
        Type::Entity => tagged("entity_type", Vec::new()),
        Type::EntitySet { maximum } => bounded_type("entity_set_type", *maximum),
        Type::Symbol => tagged("symbol_type", Vec::new()),
        Type::Action => tagged("action_type", Vec::new()),
        Type::State => tagged("state_type", Vec::new()),
        Type::Grid {
            maximum_width,
            maximum_height,
        } => tagged(
            "grid_type",
            vec![
                CanonicalValue::Unsigned(u64::from(*maximum_width)),
                CanonicalValue::Unsigned(u64::from(*maximum_height)),
            ],
        ),
        Type::Tuple(values) => tagged(
            "tuple_type",
            vec![CanonicalValue::Array(
                values.iter().map(type_value).collect(),
            )],
        ),
        Type::Option(value) => tagged("option_type", vec![type_value(value)]),
        Type::List { element, maximum } => tagged(
            "list_type",
            vec![
                type_value(element),
                CanonicalValue::Unsigned(u64::from(*maximum)),
            ],
        ),
        Type::Map {
            key,
            value,
            maximum,
        } => tagged(
            "map_type",
            vec![
                type_value(key),
                type_value(value),
                CanonicalValue::Unsigned(u64::from(*maximum)),
            ],
        ),
        Type::Relation { arguments, maximum } => tagged(
            "relation_type",
            vec![
                CanonicalValue::Array(arguments.iter().map(type_value).collect()),
                CanonicalValue::Unsigned(u64::from(*maximum)),
            ],
        ),
        Type::Operator { input, output } => {
            tagged("operator_type", vec![type_value(input), type_value(output)])
        }
        Type::Unit => tagged("unit_type", Vec::new()),
    }
}

fn bounded_type(name: &str, maximum: u32) -> CanonicalValue {
    tagged(name, vec![CanonicalValue::Unsigned(u64::from(maximum))])
}

fn signed(value: i64) -> CanonicalValue {
    if value >= 0 {
        CanonicalValue::Unsigned(value.unsigned_abs())
    } else {
        CanonicalValue::Negative(value)
    }
}

fn tagged(name: &str, values: Vec<CanonicalValue>) -> CanonicalValue {
    let mut output = Vec::with_capacity(values.len() + 1);
    output.push(CanonicalValue::Text(name.to_owned()));
    output.extend(values);
    CanonicalValue::Array(output)
}

const fn unary_name(op: UnaryOp) -> &'static str {
    match op {
        UnaryOp::Not => "not",
        UnaryOp::Negate => "negate",
    }
}

const fn binary_name(op: BinaryOp) -> &'static str {
    match op {
        BinaryOp::Add => "add",
        BinaryOp::Subtract => "subtract",
        BinaryOp::Multiply => "multiply",
        BinaryOp::Minimum => "minimum",
        BinaryOp::Maximum => "maximum",
        BinaryOp::And => "and",
        BinaryOp::Or => "or",
        BinaryOp::Equal => "equal",
        BinaryOp::LessThan => "less_than",
        BinaryOp::Union => "union",
        BinaryOp::Intersection => "intersection",
        BinaryOp::Difference => "difference",
    }
}
