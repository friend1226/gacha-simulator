use crate::ir::Expr;
use crate::rational::{parse_literal, Rational};
use num_traits::{One, Signed, Zero};
use serde_json::Value as JsonValue;
use thiserror::Error;

#[derive(Debug, Clone)]
pub enum Op {
    PushLit(Rational),
    PushVar(String),
    PushTrial,
    Add,
    Sub,
    Mul,
    Div,
    Neg,
    Abs,
    Min,
    Max,
    Clamp,
    Floor,
    Ceil,
    Round,
    PowInt(i32),
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    Not,
    Xor,
    JumpIfFalse(usize),
    Jump(usize),
}

#[derive(Debug, Clone)]
pub struct Program {
    pub ops: Vec<Op>,
    pub exact_safe: bool,
    pub trial_dependent: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EvalValue {
    Number(Rational),
    Bool(bool),
}

impl EvalValue {
    pub fn number(self) -> Result<Rational, ExprError> {
        match self {
            Self::Number(n) => Ok(n),
            _ => Err(ExprError::Type("expected number".into())),
        }
    }
    pub fn boolean(self) -> Result<bool, ExprError> {
        match self {
            Self::Bool(v) => Ok(v),
            _ => Err(ExprError::Type("expected boolean".into())),
        }
    }
}

#[derive(Debug, Error)]
pub enum ExprError {
    #[error("invalid expression: {0}")]
    Invalid(String),
    #[error("unknown or forbidden operation: {0}")]
    Forbidden(String),
    #[error("expression type error: {0}")]
    Type(String),
    #[error("unknown variable: {0}")]
    UnknownVariable(String),
    #[error("division by zero")]
    DivisionByZero,
    #[error("stack underflow")]
    StackUnderflow,
}

pub fn compile_expr(expr: &Expr) -> Result<Program, ExprError> {
    let mut program = Program { ops: Vec::new(), exact_safe: true, trial_dependent: false };
    emit(expr, &mut program)?;
    Ok(program)
}

fn emit(expr: &Expr, p: &mut Program) -> Result<(), ExprError> {
    let object = expr
        .as_object()
        .ok_or_else(|| ExprError::Invalid(expr.to_string()))?;
    if let Some(lit) = object.get("lit") {
        let text = lit
            .as_str()
            .ok_or_else(|| ExprError::Invalid("lit must be a string".into()))?;
        p.ops.push(Op::PushLit(parse_literal(text).map_err(|e| ExprError::Invalid(e.to_string()))?));
        return Ok(());
    }
    if let Some(var) = object.get("var") {
        p.ops.push(Op::PushVar(var.as_str().ok_or_else(|| ExprError::Invalid("var must be a string".into()))?.into()));
        return Ok(());
    }
    if object.get("trial").is_some() {
        p.trial_dependent = true;
        p.ops.push(Op::PushTrial);
        return Ok(());
    }
    if object.contains_key("if") {
        emit(&object["if"], p)?;
        let jump_false = p.ops.len();
        p.ops.push(Op::JumpIfFalse(usize::MAX));
        emit(object.get("then").ok_or_else(|| ExprError::Invalid("if missing then".into()))?, p)?;
        let jump_end = p.ops.len();
        p.ops.push(Op::Jump(usize::MAX));
        let else_start = p.ops.len();
        emit(object.get("else").ok_or_else(|| ExprError::Invalid("if missing else".into()))?, p)?;
        let end = p.ops.len();
        p.ops[jump_false] = Op::JumpIfFalse(else_start);
        p.ops[jump_end] = Op::Jump(end);
        return Ok(());
    }

    let known = [
        "add", "sub", "mul", "div", "neg", "abs", "floor", "ceil", "round",
        "min", "max", "clamp", "pow", "eq", "ne", "lt", "le", "gt", "ge",
        "and", "or", "not", "xor",
    ];
    let (name, value) = object
        .iter()
        .find(|(k, _)| known.contains(&k.as_str()))
        .ok_or_else(|| {
            let name = object.keys().next().cloned().unwrap_or_default();
            ExprError::Forbidden(name)
        })?;
    let args = match value {
        JsonValue::Array(values) => values.as_slice(),
        _ => std::slice::from_ref(value),
    };
    let arity = match name.as_str() {
        "neg" | "abs" | "floor" | "ceil" | "round" | "not" => 1,
        "clamp" => 3,
        _ => 2,
    };
    if args.len() != arity {
        return Err(ExprError::Invalid(format!("{name} expects {arity} argument(s)")));
    }
    if name == "pow" {
        emit(&args[0], p)?;
        let exponent = args[1]
            .get("lit")
            .and_then(JsonValue::as_str)
            .ok_or_else(|| ExprError::Invalid("pow exponent must be an integer literal".into()))?
            .parse::<i32>()
            .map_err(|_| ExprError::Invalid("pow exponent must be an integer literal".into()))?;
        p.ops.push(Op::PowInt(exponent));
        return Ok(());
    }
    for arg in args { emit(arg, p)?; }
    p.ops.push(match name.as_str() {
        "add" => Op::Add, "sub" => Op::Sub, "mul" => Op::Mul, "div" => Op::Div,
        "neg" => Op::Neg, "abs" => Op::Abs, "floor" => Op::Floor, "ceil" => Op::Ceil,
        "round" => Op::Round, "min" => Op::Min, "max" => Op::Max, "clamp" => Op::Clamp,
        "eq" => Op::Eq, "ne" => Op::Ne, "lt" => Op::Lt, "le" => Op::Le,
        "gt" => Op::Gt, "ge" => Op::Ge, "and" => Op::And, "or" => Op::Or,
        "not" => Op::Not, "xor" => Op::Xor,
        _ => unreachable!(),
    });
    Ok(())
}

pub fn eval(
    program: &Program,
    mut variable: impl FnMut(&str) -> Option<Rational>,
    trial: u32,
) -> Result<EvalValue, ExprError> {
    let mut stack = Vec::with_capacity(16);
    let mut pc = 0;
    while pc < program.ops.len() {
        match &program.ops[pc] {
            Op::PushLit(v) => stack.push(EvalValue::Number(v.clone())),
            Op::PushVar(name) => stack.push(EvalValue::Number(variable(name).ok_or_else(|| ExprError::UnknownVariable(name.clone()))?)),
            Op::PushTrial => stack.push(EvalValue::Number(Rational::from_integer(trial.into()))),
            Op::Neg => unary_number(&mut stack, |a| -a)?,
            Op::Abs => unary_number(&mut stack, |a| a.abs())?,
            Op::Floor => unary_number(&mut stack, |a| Rational::from_integer(a.floor().to_integer()))?,
            Op::Ceil => unary_number(&mut stack, |a| Rational::from_integer(a.ceil().to_integer()))?,
            Op::Round => unary_number(&mut stack, round_rational)?,
            Op::Not => {
                let v = pop(&mut stack)?.boolean()?;
                stack.push(EvalValue::Bool(!v));
            }
            Op::Add => binary_number(&mut stack, |a, b| Ok(a + b))?,
            Op::Sub => binary_number(&mut stack, |a, b| Ok(a - b))?,
            Op::Mul => binary_number(&mut stack, |a, b| Ok(a * b))?,
            Op::Div => binary_number(&mut stack, |a, b| if b.is_zero() { Err(ExprError::DivisionByZero) } else { Ok(a / b) })?,
            Op::Min => binary_number(&mut stack, |a, b| Ok(if a <= b { a } else { b }))?,
            Op::Max => binary_number(&mut stack, |a, b| Ok(if a >= b { a } else { b }))?,
            Op::Clamp => {
                let hi = pop(&mut stack)?.number()?;
                let lo = pop(&mut stack)?.number()?;
                let value = pop(&mut stack)?.number()?;
                stack.push(EvalValue::Number(if value < lo { lo } else if value > hi { hi } else { value }));
            }
            Op::PowInt(exp) => unary_number(&mut stack, |a| {
                if *exp >= 0 { a.pow(*exp) } else { Rational::one() / a.pow(-*exp) }
            })?,
            Op::Eq => binary_compare(&mut stack, |a, b| a == b)?,
            Op::Ne => binary_compare(&mut stack, |a, b| a != b)?,
            Op::Lt => binary_compare(&mut stack, |a, b| a < b)?,
            Op::Le => binary_compare(&mut stack, |a, b| a <= b)?,
            Op::Gt => binary_compare(&mut stack, |a, b| a > b)?,
            Op::Ge => binary_compare(&mut stack, |a, b| a >= b)?,
            Op::And => binary_bool(&mut stack, |a, b| a && b)?,
            Op::Or => binary_bool(&mut stack, |a, b| a || b)?,
            Op::Xor => binary_bool(&mut stack, |a, b| a ^ b)?,
            Op::JumpIfFalse(target) => {
                if !pop(&mut stack)?.boolean()? { pc = *target; continue; }
            }
            Op::Jump(target) => { pc = *target; continue; }
        }
        pc += 1;
    }
    if stack.len() != 1 {
        return Err(ExprError::Invalid("expression did not yield exactly one value".into()));
    }
    Ok(stack.pop().unwrap())
}

fn pop(stack: &mut Vec<EvalValue>) -> Result<EvalValue, ExprError> {
    stack.pop().ok_or(ExprError::StackUnderflow)
}
fn unary_number(stack: &mut Vec<EvalValue>, f: impl FnOnce(Rational) -> Rational) -> Result<(), ExprError> {
    let a = pop(stack)?.number()?;
    stack.push(EvalValue::Number(f(a)));
    Ok(())
}
fn binary_number(stack: &mut Vec<EvalValue>, f: impl FnOnce(Rational, Rational) -> Result<Rational, ExprError>) -> Result<(), ExprError> {
    let b = pop(stack)?.number()?;
    let a = pop(stack)?.number()?;
    stack.push(EvalValue::Number(f(a, b)?));
    Ok(())
}
fn binary_compare(stack: &mut Vec<EvalValue>, f: impl FnOnce(Rational, Rational) -> bool) -> Result<(), ExprError> {
    let b = pop(stack)?.number()?;
    let a = pop(stack)?.number()?;
    stack.push(EvalValue::Bool(f(a, b)));
    Ok(())
}
fn binary_bool(stack: &mut Vec<EvalValue>, f: impl FnOnce(bool, bool) -> bool) -> Result<(), ExprError> {
    let b = pop(stack)?.boolean()?;
    let a = pop(stack)?.boolean()?;
    stack.push(EvalValue::Bool(f(a, b)));
    Ok(())
}
fn round_rational(value: Rational) -> Rational {
    let floor = value.floor();
    let fraction = &value - &floor;
    let half = Rational::new(1.into(), 2.into());
    if fraction >= half { Rational::from_integer(floor.to_integer() + 1) }
    else { Rational::from_integer(floor.to_integer()) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn compiles_and_evaluates_branch() {
        let expr = json!({
            "if": {"ge": [{"var":"pity"}, {"lit":"65"}]},
            "then": {"add": [{"lit":"0.03"}, {"mul":[{"lit":"0.03"},{"sub":[{"var":"pity"},{"lit":"65"}]}]}]},
            "else": {"lit":"0.03"}
        });
        let program = compile_expr(&expr).unwrap();
        let value = eval(&program, |name| (name == "pity").then(|| Rational::from_integer(66.into())), 1)
            .unwrap().number().unwrap();
        assert_eq!(value, Rational::new(6.into(), 100.into()));
    }
}

