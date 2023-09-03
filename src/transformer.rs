use std::rc::Rc;

use log::info;

use crate::common::Validation;
use crate::error::Result;
use crate::json::Appliable;
use crate::operation::{Operation, OperationComponent, Operator};
use crate::path::{Path, PathElement};
use crate::sub_type::SubTypeFunctionsHolder;

fn is_equivalent_to_noop(op: &OperationComponent) -> bool {
    match &op.operator {
        Operator::Noop() => true,
        Operator::SubType(_, _) => false,
        Operator::SubType2(_, _, _) => false,
        Operator::AddNumber(_)
        | Operator::ListInsert(_)
        | Operator::ListDelete(_)
        | Operator::ObjectInsert(_)
        | Operator::ObjectDelete(_) => false,
        Operator::ListReplace(new_v, old_v) | Operator::ObjectReplace(new_v, old_v) => {
            new_v.eq(old_v)
        }
        Operator::ListMove(lm) => op
            .path
            .last()
            .map(|p| p == &PathElement::Index(*lm))
            .unwrap_or(false),
    }
}

#[derive(PartialEq)]
pub enum TransformSide {
    LEFT,
    RIGHT,
}

pub struct Transformer {
    sub_type_holder: Rc<SubTypeFunctionsHolder>,
}

impl Transformer {
    pub fn new(sub_type_holder: Rc<SubTypeFunctionsHolder>) -> Transformer {
        Transformer { sub_type_holder }
    }

    pub fn transform(
        &self,
        operation: &Operation,
        base_operation: &Operation,
    ) -> Result<(Operation, Operation)> {
        if base_operation.is_empty() {
            return Ok((operation.clone(), Operation::empty_operation()));
        }

        operation.validates()?;
        base_operation.validates()?;

        if operation.len() == 1 && base_operation.len() == 1 {
            let a = self.transform_component(
                operation.get(0).unwrap().clone(),
                base_operation.get(0).unwrap(),
                TransformSide::LEFT,
            )?;
            let b = self.transform_component(
                base_operation.get(0).unwrap().clone(),
                operation.get(0).unwrap(),
                TransformSide::RIGHT,
            )?;

            return Ok((a.into(), b.into()));
        }

        self.transform_matrix(operation.clone(), base_operation.clone())
    }

    fn transform_matrix(
        &self,
        operation: Operation,
        base_operation: Operation,
    ) -> Result<(Operation, Operation)> {
        if operation.is_empty() || base_operation.is_empty() {
            return Ok((operation, base_operation));
        }

        let mut out_b = vec![];
        let mut ops = operation;
        for base_op in base_operation {
            let (a, b) = self.transform_multi(ops, base_op)?;
            ops = a;

            if let Some(o) = b {
                out_b.push(o);
            }
        }

        Ok((ops, out_b.into()))
    }

    fn transform_multi(
        &self,
        operation: Operation,
        base_op: OperationComponent,
    ) -> Result<(Operation, Option<OperationComponent>)> {
        let mut out: Vec<OperationComponent> = vec![];

        let mut base = base_op.not_noop();
        for op in operation {
            match base {
                Some(b) => {
                    let backup = op.clone();
                    let mut a = self.transform_component(op, &b, TransformSide::LEFT)?;
                    let mut b = self.transform_component(b, &backup, TransformSide::RIGHT)?;
                    assert!(b.len() == 1);
                    base = b.pop();

                    out.append(&mut a);
                }
                None => {
                    out.push(op.clone());
                    continue;
                }
            }
        }

        Ok((out.into(), base))
    }

    fn transform_component(
        &self,
        new_op: OperationComponent,
        base_op: &OperationComponent,
        side: TransformSide,
    ) -> Result<Vec<OperationComponent>> {
        let mut new_op = new_op;
        if is_equivalent_to_noop(&new_op) || is_equivalent_to_noop(base_op) {
            return Ok(vec![new_op]);
        }

        let max_common_path = base_op.path.max_common_path(&new_op.path);
        let new_operate_path_len = new_op.operate_path_len();
        let base_operate_path_len = base_op.operate_path_len();

        if max_common_path.len() < new_operate_path_len
            && max_common_path.len() < base_operate_path_len
        {
            // common path must be equal to new_op's or base_op's operate path
            // or base_op and new_op is operating on orthogonal value
            // they don't need transform
            return Ok(vec![new_op]);
        }

        // such as:
        // new_op, base_op
        // [p1,p2,p3], [p1,p2,p4,p5]
        // [p1,p2,p3], [p1,p2,p3,p5]
        if base_operate_path_len > new_operate_path_len {
            // if base_op's path is longger and contains new_op's path, new_op should include base_op's effect
            if new_op.path.is_prefix_of(&base_op.path) {
                self.consume(&mut new_op, &max_common_path, base_op)?;
            }
            return Ok(vec![new_op]);
        }

        // from here, base_op's path is shorter or equal to new_op, such as:
        // new_op, base_op
        // [p1,p2,p3], [p1,p2,p3]. same operand and base_op is prefix of new_op
        // [p1,p2,p4], [p1,p2,p3]. same operand
        // [p1,p2,p3,p4,..], [p1,p2,p3], base_op is prefix of new_op
        // [p1,p2,p4,p5,..], [p1,p2,p3]
        let same_operand = base_op.path.len() == new_op.path.len();
        let base_op_is_prefix = base_op.path.is_prefix_of(&new_op.path);
        match &base_op.operator {
            Operator::SubType2(base_sub_type, base_op_operand, base_f) => {
                if let Operator::SubType2(new_op_subtype, new_op_operand, _) = &new_op.operator {
                    if base_sub_type.eq(new_op_subtype) {
                        return base_f
                            .transform(new_op_operand, base_op_operand, side)?
                            .into_iter()
                            .map(|new_operand| {
                                OperationComponent::new(
                                    base_op.path.clone(),
                                    Operator::SubType2(
                                        base_sub_type.clone(),
                                        new_operand,
                                        base_f.box_clone(),
                                    ),
                                )
                            })
                            .collect::<Result<Vec<OperationComponent>>>();
                    }
                }
            }
            Operator::ListReplace(li_v, _) => {
                if base_op_is_prefix {
                    if !same_operand {
                        return Ok(vec![]);
                    }
                    if let Operator::ListReplace(new_li, _) = &new_op.operator {
                        if side == TransformSide::LEFT {
                            return Ok(vec![OperationComponent::new(
                                new_op.path,
                                Operator::ListReplace(new_li.clone(), li_v.clone()),
                            )?]);
                        } else {
                            return Ok(vec![]);
                        }
                    }
                    if let Operator::ListDelete(_) = &new_op.operator {
                        return Ok(vec![]);
                    }
                }
            }
            Operator::ListInsert(_) => {
                if let Operator::ListInsert(_) = &new_op.operator {
                    if same_operand && base_op_is_prefix {
                        if side == TransformSide::RIGHT {
                            new_op.path.increase_index(base_operate_path_len);
                        }
                        return Ok(vec![new_op]);
                    }
                }

                if base_op
                    .path
                    .get(base_operate_path_len)
                    .and_then(|p1| new_op.path.get(base_operate_path_len).map(|p2| p1 <= p2))
                    .unwrap_or(false)
                {
                    new_op.path.increase_index(base_operate_path_len);
                }

                if let Operator::ListMove(lm) = &mut new_op.operator {
                    if same_operand
                        && base_op
                            .path
                            .get(base_operate_path_len)
                            .map(|p| p <= &PathElement::Index(*lm))
                            .unwrap_or(false)
                    {
                        new_op.operator = Operator::ListMove(*lm + 1);
                    }
                }
            }
            Operator::ListDelete(_) => {
                let base_op_operate_path = base_op.path.get(base_operate_path_len).unwrap();
                let new_op_operate_path = new_op.path.get(base_operate_path_len).unwrap();
                if let Operator::ListMove(lm) = new_op.operator {
                    if same_operand {
                        if base_op_is_prefix {
                            // base_op deleted the thing we're trying to move
                            return Ok(vec![]);
                        }
                        let to = lm.into();
                        if base_op_operate_path < &to
                            || (base_op_operate_path.eq(&to) && new_op_operate_path < &to)
                        {
                            new_op.operator = Operator::ListMove(lm - 1);
                        }
                    }
                }

                if base_op_operate_path < new_op_operate_path {
                    new_op.path.decrease_index(base_operate_path_len);
                } else if base_op_is_prefix {
                    if !same_operand {
                        // we're below the deleted element, so -> noop
                        return Ok(vec![]);
                    }
                    if let Operator::ListDelete(_) = new_op.operator {
                        // we're trying to delete the same element, -> noop
                        return Ok(vec![]);
                    }
                    if let Operator::ListReplace(li, _) = new_op.operator {
                        // we're replacing, they're deleting. we become an insert.
                        return Ok(vec![OperationComponent::new(
                            new_op.path.clone(),
                            Operator::ListInsert(li.clone()),
                        )?]);
                    }
                }
            }
            Operator::ObjectReplace(oi, _) => {
                if base_op_is_prefix {
                    if !same_operand {
                        return Ok(vec![]);
                    }

                    match &new_op.operator {
                        Operator::ObjectReplace(new_oi, _) | Operator::ObjectInsert(new_oi) => {
                            if side == TransformSide::RIGHT {
                                return Ok(vec![]);
                            }
                            return Ok(vec![OperationComponent {
                                path: new_op.path.clone(),
                                operator: Operator::ObjectReplace(new_oi.clone(), oi.clone()),
                            }]);
                        }
                        _ => {
                            return Ok(vec![]);
                        }
                    }
                }
            }
            Operator::ObjectInsert(base_oi) => {
                if base_op_is_prefix {
                    if let Operator::ObjectReplace(new_oi, _) | Operator::ObjectInsert(new_oi) =
                        &new_op.operator
                    {
                        if side == TransformSide::LEFT {
                            if same_operand {
                                return Ok(vec![OperationComponent {
                                    path: base_op.path.clone(),
                                    operator: Operator::ObjectReplace(
                                        new_oi.clone(),
                                        base_oi.clone(),
                                    ),
                                }]);
                            }
                            // Here, we are different from original json0
                            // eg: new_op = [{"p": ["p1", "p2"],"oi": "v1"}], base_op = [{"p": ["p1"],"oi": "v2"}]
                            // after execution of these op, the result should be {"p1":{"p2":"v1"}}, so new_op after left transform
                            // is [{"p": ["p1"],"od": "v2"}, {"p": ["p1", "p2"],"oi": "v1"}]
                            // but original json0 is [{"p": ["p1", "p2"],"od": "v2"}, {"p": ["p1", "p2"],"oi": "v1"}]
                            // the problem of original json0 is "v2" inserted by base_op is under path p1, not [p1, p2]
                            return Ok(vec![
                                OperationComponent {
                                    path: base_op.path.clone(),
                                    operator: Operator::ObjectDelete(base_oi.clone()),
                                },
                                new_op,
                            ]);
                        } else {
                            return Ok(vec![]);
                        }
                    } else if let Operator::ObjectDelete(_) = &new_op.operator {
                        if side == TransformSide::RIGHT {
                            return Ok(vec![]);
                        }
                    }
                }
            }
            Operator::ObjectDelete(_) => {
                if base_op_is_prefix {
                    if !same_operand {
                        return Ok(vec![]);
                    }
                    if let Operator::ObjectReplace(new_oi, _) | Operator::ObjectInsert(new_oi) =
                        &new_op.operator
                    {
                        if side == TransformSide::LEFT {
                            return Ok(vec![OperationComponent {
                                path: new_op.path.clone(),
                                operator: Operator::ObjectInsert(new_oi.clone()),
                            }]);
                        } else {
                            return Ok(vec![]);
                        }
                    } else {
                        return Ok(vec![]);
                    }
                }
            }
            Operator::ListMove(lm) => {
                if same_operand {
                    match &mut new_op.operator {
                        Operator::ListMove(new_op_lm) => {
                            let other_from = base_op.path.get(new_operate_path_len).unwrap();
                            let other_to = PathElement::Index(*lm);

                            if other_from == &other_to {
                                return Ok(vec![new_op]);
                            }

                            let from = new_op.path.get(new_operate_path_len).unwrap().clone();
                            let to: PathElement = PathElement::Index(*new_op_lm);

                            if &from == other_from {
                                if to == other_to {
                                    // already moved to where we want
                                    return Ok(vec![]);
                                }
                                if side == TransformSide::LEFT {
                                    new_op.path.replace(base_operate_path_len, other_to.clone());
                                    if from == to {
                                        new_op.operator = base_op.operator.clone();
                                    }
                                } else {
                                    return Ok(vec![]);
                                }
                            } else {
                                let mut n_lm = *new_op_lm;
                                if &from > other_from {
                                    new_op.path.decrease_index(base_operate_path_len);
                                }
                                if from > other_to {
                                    new_op.path.increase_index(base_operate_path_len);
                                } else if from == other_to && other_from > &other_to {
                                    new_op.path.increase_index(base_operate_path_len);
                                    if from == to {
                                        n_lm += 1;
                                    }
                                }
                                if &to > other_from || (&to == other_from && to > from) {
                                    n_lm -= 1;
                                }
                                if to > other_to {
                                    n_lm += 1;
                                } else if to == other_to {
                                    if (&other_to > other_from && to > from)
                                        || (&other_to < other_from && to < from)
                                    {
                                        if side == TransformSide::RIGHT {
                                            n_lm += 1;
                                        }
                                    } else if to > from {
                                        n_lm += 1;
                                    } else if &to == other_from {
                                        n_lm -= 1;
                                    }
                                }
                                new_op.operator = Operator::ListMove(n_lm);
                            }
                            return Ok(vec![new_op]);
                        }
                        Operator::ListInsert(_) => {
                            let operate_index = base_operate_path_len;
                            let from = base_op.path.get(operate_index).unwrap();
                            let to = *lm;
                            let p = new_op.path.get(operate_index).unwrap().clone();
                            if &p > from {
                                new_op.path.decrease_index(operate_index);
                            }
                            if p > PathElement::Index(to) {
                                new_op.path.increase_index(operate_index);
                            }
                            return Ok(vec![new_op]);
                        }
                        _ => {}
                    }
                }
                let from = base_op.path.get(base_operate_path_len).unwrap();
                let to = PathElement::Index(*lm);
                let p = new_op.path.get(base_operate_path_len).unwrap().clone();
                if &p == from {
                    new_op.path.replace(base_operate_path_len, to.clone());
                } else {
                    if &p > from {
                        new_op.path.decrease_index(base_operate_path_len);
                    }
                    if p > to || (p == to && from > &to) {
                        new_op.path.increase_index(base_operate_path_len);
                    }
                }
            }
            _ => {}
        }

        Ok(vec![new_op])
    }

    pub fn consume(
        &self,
        op: &mut OperationComponent,
        common_path: &Path,
        other: &OperationComponent,
    ) -> Result<()> {
        match &mut op.operator {
            Operator::ListDelete(v)
            | Operator::ListReplace(_, v)
            | Operator::ObjectDelete(v)
            | Operator::ObjectReplace(_, v) => {
                let (_, p2) = other.path.split_at(common_path.len());
                // v maybe cannot apply other.operator
                // if that happen we do not consume other just leave origin op
                _ = v.apply(p2, other.operator.clone());
            }
            _ => {}
        }
        Ok(())
    }
}
