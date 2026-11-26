//! Call-signature verification helpers, kept out of the main verifier file
//! so each file stays within ADR-0016's size guidance.

use std::collections::HashMap;

use super::checks::value_type;
use super::ir::{BlockId, IrFunction, IrType, Value, ValueId};
use super::verify::{VerifyError, error};

pub(super) struct CallSignature<'a> {
    pub(super) params: Vec<IrType>,
    pub(super) return_ty: IrType,
    pub(super) args: &'a [Value],
    pub(super) result_ty: IrType,
    pub(super) defs: &'a HashMap<ValueId, (IrType, BlockId)>,
}

/// Check direct/indirect call arity and argument/result types.
pub(super) fn check_call_signature(
    function: &IrFunction,
    block: BlockId,
    signature: &CallSignature<'_>,
    errors: &mut Vec<VerifyError>,
) {
    let CallSignature {
        params,
        return_ty,
        args,
        result_ty,
        defs,
    } = signature;
    if params.len() != args.len() {
        errors.push(error(
            function,
            Some(block),
            format!(
                "call expects {} arguments, found {}",
                params.len(),
                args.len()
            ),
        ));
        return;
    }
    for (index, (expected, argument)) in params.iter().zip(*args).enumerate() {
        let actual = value_type(argument, defs);
        if !types_compatible(expected, &actual) {
            errors.push(error(
                function,
                Some(block),
                format!(
                    "argument {index}: expected '{}', found '{}'",
                    expected.display(),
                    actual.display()
                ),
            ));
        }
    }
    if *result_ty != IrType::Opaque && *return_ty != IrType::Opaque && *result_ty != *return_ty {
        errors.push(error(
            function,
            Some(block),
            format!(
                "call result: expected '{}', found '{}'",
                return_ty.display(),
                result_ty.display()
            ),
        ));
    }
}

/// Object/array/function layouts share a pointer representation; distinguish
/// only the representation class, not the exact field list or signature.
pub(super) fn types_compatible(expected: &IrType, actual: &IrType) -> bool {
    if *expected == IrType::Opaque || *actual == IrType::Opaque {
        return true;
    }
    match (expected, actual) {
        (IrType::Array(_), IrType::Array(_))
        | (IrType::Object(_), IrType::Object(_))
        | (IrType::Function { .. }, IrType::Function { .. }) => true,
        _ => expected == actual,
    }
}
