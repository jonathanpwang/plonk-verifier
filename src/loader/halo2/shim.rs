use crate::util::arithmetic::{CurveAffine, FieldExt};
use halo2_proofs::{
    circuit::{Cell, Value},
    plonk::Error,
};
use std::fmt::Debug;

pub trait Context: Debug {
    fn constrain_equal(&mut self, lhs: Cell, rhs: Cell) -> Result<(), Error>;

    fn offset(&self) -> usize;
}

pub trait IntegerInstructions<'a, F: FieldExt>: Clone + Debug {
    type Context: Context;
    type Integer: Clone + Debug;
    type AssignedInteger: Clone + Debug;

    fn integer(&self, fe: F) -> Self::Integer;

    fn assign_integer(
        &self,
        ctx: &mut Self::Context,
        integer: Value<Self::Integer>,
    ) -> Result<Self::AssignedInteger, Error>;

    fn assign_constant(
        &self,
        ctx: &mut Self::Context,
        integer: F,
    ) -> Result<Self::AssignedInteger, Error>;

    fn sum_with_coeff_and_const(
        &self,
        ctx: &mut Self::Context,
        values: &[(F::Scalar, Self::AssignedInteger)],
        constant: F::Scalar,
    ) -> Result<Self::AssignedInteger, Error>;

    fn sum_products_with_coeff_and_const(
        &self,
        ctx: &mut Self::Context,
        values: &[(F::Scalar, Self::AssignedInteger, Self::AssignedInteger)],
        constant: F::Scalar,
    ) -> Result<Self::AssignedInteger, Error>;

    fn sub(
        &self,
        ctx: &mut Self::Context,
        a: &Self::AssignedInteger,
        b: &Self::AssignedInteger,
    ) -> Result<Self::AssignedInteger, Error>;

    fn neg(
        &self,
        ctx: &mut Self::Context,
        a: &Self::AssignedInteger,
    ) -> Result<Self::AssignedInteger, Error>;

    fn invert(
        &self,
        ctx: &mut Self::Context,
        a: &Self::AssignedInteger,
    ) -> Result<Self::AssignedInteger, Error>;

    fn assert_equal(
        &self,
        ctx: &mut Self::Context,
        a: &Self::AssignedInteger,
        b: &Self::AssignedInteger,
    ) -> Result<(), Error>;
}

pub trait EccInstructions<'a, C: CurveAffine>: Clone + Debug {
    type Context: Context;
    type ScalarChip: IntegerInstructions<
        'a,
        C::Scalar,
        Context = Self::Context,
        Integer = Self::Scalar,
        AssignedInteger = Self::AssignedScalar,
    >;
    type AssignedEcPoint: Clone + Debug;
    type Scalar: Clone + Debug;
    type AssignedScalar: Clone + Debug;

    fn scalar_chip(&self) -> &Self::ScalarChip;

    fn assign_constant(
        &self,
        ctx: &mut Self::Context,
        point: C,
    ) -> Result<Self::AssignedEcPoint, Error>;

    fn assign_point(
        &self,
        ctx: &mut Self::Context,
        point: Value<C>,
    ) -> Result<Self::AssignedEcPoint, Error>;

    fn sum_with_const(
        &self,
        ctx: &mut Self::Context,
        values: &[Self::AssignedEcPoint],
        constant: C,
    ) -> Result<Self::AssignedEcPoint, Error>;

    fn fixed_base_msm(
        &mut self,
        ctx: &mut Self::Context,
        pairs: &[(Self::AssignedScalar, C)],
    ) -> Result<Self::AssignedEcPoint, Error>;

    fn variable_base_msm(
        &mut self,
        ctx: &mut Self::Context,
        pairs: &[(Self::AssignedScalar, Self::AssignedEcPoint)],
    ) -> Result<Self::AssignedEcPoint, Error>;

    fn normalize(
        &self,
        ctx: &mut Self::Context,
        point: &Self::AssignedEcPoint,
    ) -> Result<Self::AssignedEcPoint, Error>;

    fn assert_equal(
        &self,
        ctx: &mut Self::Context,
        a: &Self::AssignedEcPoint,
        b: &Self::AssignedEcPoint,
    ) -> Result<(), Error>;
}

mod halo2_lib {
    use crate::{
        loader::halo2::{Context, EccInstructions, IntegerInstructions},
        util::arithmetic::{CurveAffine, Field, PrimeField},
    };
    use halo2_base::{
        self,
        gates::{flex_gate::FlexGateConfig, GateInstructions, RangeInstructions},
        AssignedValue,
        QuantumCell::{Constant, Existing, Witness},
    };
    use halo2_curves::BigPrimeField;
    use halo2_ecc::{
        bigint::CRTInteger,
        ecc::{fixed::FixedEcPoint, BaseFieldEccChip, EcPoint},
        fields::FieldChip,
    };
    use halo2_proofs::{
        circuit::{Cell, Value},
        plonk::Error,
    };

    type AssignedInteger<'v, C> = CRTInteger<'v, <C as CurveAffine>::ScalarExt>;
    type AssignedEcPoint<'v, C> = EcPoint<<C as CurveAffine>::ScalarExt, AssignedInteger<'v, C>>;

    impl<'a, F: BigPrimeField> Context for halo2_base::Context<'a, F> {
        fn constrain_equal(&mut self, lhs: Cell, rhs: Cell) -> Result<(), Error> {
            self.region.constrain_equal(lhs, rhs)
        }

        fn offset(&self) -> usize {
            unreachable!()
        }
    }

    impl<'a, F: BigPrimeField> IntegerInstructions<'a, F> for FlexGateConfig<F> {
        type Context = halo2_base::Context<'a, F>;
        type Integer = F;
        type AssignedInteger = AssignedValue<'a, F>;

        fn integer(&self, scalar: F) -> Self::Integer {
            scalar
        }

        fn assign_integer(
            &self,
            ctx: &mut Self::Context,
            integer: Value<Self::Integer>,
        ) -> Result<Self::AssignedInteger, Error> {
            Ok(self.assign_region_last(ctx, vec![Witness(integer)], vec![]))
        }

        fn assign_constant(
            &self,
            ctx: &mut Self::Context,
            integer: F,
        ) -> Result<Self::AssignedInteger, Error> {
            Ok(self.assign_region_last(ctx, vec![Constant(integer)], vec![]))
        }

        fn sum_with_coeff_and_const(
            &self,
            ctx: &mut Self::Context,
            values: &[(F, Self::AssignedInteger)],
            constant: F,
        ) -> Result<Self::AssignedInteger, Error> {
            let mut a = Vec::with_capacity(values.len() + 1);
            let mut b = Vec::with_capacity(values.len() + 1);
            if constant != F::zero() {
                a.push(Constant(F::one()));
                b.push(Constant(constant));
            }
            a.extend(values.iter().map(|(_, a)| Existing(a)));
            b.extend(values.iter().map(|(c, _)| Constant(*c)));
            Ok(self.inner_product(ctx, a, b))
        }

        fn sum_products_with_coeff_and_const(
            &self,
            ctx: &mut Self::Context,
            values: &[(F, Self::AssignedInteger, Self::AssignedInteger)],
            constant: F,
        ) -> Result<Self::AssignedInteger, Error> {
            match values.len() {
                0 => self.assign_constant(ctx, constant),
                _ => Ok(self.sum_products_with_coeff_and_var(
                    ctx,
                    values
                        .iter()
                        .map(|(c, a, b)| (*c, Existing(a), Existing(b))),
                    Constant(constant),
                )),
            }
        }

        fn sub(
            &self,
            ctx: &mut Self::Context,
            a: &Self::AssignedInteger,
            b: &Self::AssignedInteger,
        ) -> Result<Self::AssignedInteger, Error> {
            Ok(GateInstructions::sub(self, ctx, Existing(a), Existing(b)))
        }

        fn neg(
            &self,
            ctx: &mut Self::Context,
            a: &Self::AssignedInteger,
        ) -> Result<Self::AssignedInteger, Error> {
            Ok(GateInstructions::neg(self, ctx, Existing(a)))
        }

        fn invert(
            &self,
            ctx: &mut Self::Context,
            a: &Self::AssignedInteger,
        ) -> Result<Self::AssignedInteger, Error> {
            // make sure scalar != 0
            let is_zero = self.is_zero(ctx, a);
            self.assert_is_const(ctx, &is_zero, F::zero());
            Ok(GateInstructions::div_unsafe(
                self,
                ctx,
                Constant(F::one()),
                Existing(a),
            ))
        }

        fn assert_equal(
            &self,
            ctx: &mut Self::Context,
            a: &Self::AssignedInteger,
            b: &Self::AssignedInteger,
        ) -> Result<(), Error> {
            ctx.region.constrain_equal(a.cell(), b.cell())
        }
    }

    impl<'a, 'b, C: CurveAffine> EccInstructions<'a, C> for BaseFieldEccChip<'b, C>
    where
        C::Scalar: BigPrimeField,
        C::Base: BigPrimeField,
    {
        type Context = halo2_base::Context<'a, C::Scalar>;
        type ScalarChip = FlexGateConfig<C::Scalar>;
        type AssignedEcPoint = AssignedEcPoint<'a, C>;
        type Scalar = C::Scalar;
        type AssignedScalar = AssignedValue<'a, C::Scalar>;

        fn scalar_chip(&self) -> &Self::ScalarChip {
            self.field_chip.range().gate()
        }

        fn assign_constant(
            &self,
            ctx: &mut Self::Context,
            point: C,
        ) -> Result<Self::AssignedEcPoint, Error> {
            let fixed = FixedEcPoint::<C::Scalar, C>::from_g1(
                &point,
                self.field_chip.num_limbs,
                self.field_chip.limb_bits,
            );
            Ok(FixedEcPoint::assign(fixed, self.field_chip, ctx))
        }

        fn assign_point(
            &self,
            ctx: &mut Self::Context,
            point: Value<C>,
        ) -> Result<Self::AssignedEcPoint, Error> {
            let assigned = self.assign_point(ctx, point);
            let is_on_curve_or_infinity = self.is_on_curve_or_infinity::<C>(ctx, &assigned);
            self.field_chip.range.gate.assert_is_const(
                ctx,
                &is_on_curve_or_infinity,
                C::Scalar::one(),
            );
            Ok(assigned)
        }

        fn sum_with_const(
            &self,
            ctx: &mut Self::Context,
            values: &[Self::AssignedEcPoint],
            constant: C,
        ) -> Result<Self::AssignedEcPoint, Error> {
            let constant = if bool::from(constant.is_identity()) {
                None
            } else {
                let constant = EccInstructions::<C>::assign_constant(self, ctx, constant).unwrap();
                Some(constant)
            };
            Ok(self.sum::<C>(ctx, constant.iter().chain(values.iter())))
        }

        fn variable_base_msm(
            &mut self,
            ctx: &mut Self::Context,
            pairs: &[(Self::AssignedScalar, Self::AssignedEcPoint)],
        ) -> Result<Self::AssignedEcPoint, Error> {
            let (scalars, points): (Vec<_>, Vec<_>) = pairs.iter().cloned().unzip();
            Ok(self.multi_scalar_mult::<C>(
                ctx,
                &points,
                &scalars.into_iter().map(|scalar| vec![scalar]).collect(),
                <C::Scalar as PrimeField>::NUM_BITS as usize,
                4, // empirically clump factor of 4 seems to be best
            ))
        }

        fn fixed_base_msm(
            &mut self,
            ctx: &mut Self::Context,
            pairs: &[(Self::AssignedScalar, C)],
        ) -> Result<Self::AssignedEcPoint, Error> {
            let (scalars, points): (Vec<_>, Vec<_>) = pairs.iter().cloned().unzip();
            Ok(BaseFieldEccChip::<C>::fixed_base_msm::<C>(
                self,
                ctx,
                &points,
                &scalars.into_iter().map(|scalar| vec![scalar]).collect(),
                <C::Scalar as PrimeField>::NUM_BITS as usize,
                0,
                4,
            ))
        }

        fn normalize(
            &self,
            _: &mut Self::Context,
            point: &Self::AssignedEcPoint,
        ) -> Result<Self::AssignedEcPoint, Error> {
            Ok(point.clone())
        }

        fn assert_equal(
            &self,
            ctx: &mut Self::Context,
            a: &Self::AssignedEcPoint,
            b: &Self::AssignedEcPoint,
        ) -> Result<(), Error> {
            self.assert_equal(ctx, a, b);
            Ok(())
        }
    }
}

mod halo2_wrong {
    use crate::{
        loader::halo2::{Context, EccInstructions, IntegerInstructions},
        util::{
            arithmetic::{CurveAffine, FieldExt, Group},
            Itertools,
        },
    };
    use halo2_proofs::{
        circuit::{AssignedCell, Cell, Value},
        plonk::Error,
    };
    use halo2_wrong_ecc::{
        integer::rns::Common,
        maingate::{
            CombinationOption, CombinationOptionCommon, MainGate, MainGateInstructions, RegionCtx,
            Term,
        },
        AssignedPoint, BaseFieldEccChip,
    };
    use rand::rngs::OsRng;
    use std::iter;

    impl<'a, F: FieldExt> Context for RegionCtx<'a, F> {
        fn constrain_equal(&mut self, lhs: Cell, rhs: Cell) -> Result<(), Error> {
            self.constrain_equal(lhs, rhs)
        }

        fn offset(&self) -> usize {
            self.offset()
        }
    }

    impl<'a, F: FieldExt> IntegerInstructions<'a, F> for MainGate<F> {
        type Context = RegionCtx<'a, F>;
        type Integer = F;
        type AssignedInteger = AssignedCell<F, F>;

        fn integer(&self, scalar: F) -> Self::Integer {
            scalar
        }

        fn assign_integer(
            &self,
            ctx: &mut Self::Context,
            integer: Value<Self::Integer>,
        ) -> Result<Self::AssignedInteger, Error> {
            self.assign_value(ctx, integer)
        }

        fn assign_constant(
            &self,
            ctx: &mut Self::Context,
            integer: F,
        ) -> Result<Self::AssignedInteger, Error> {
            MainGateInstructions::assign_constant(self, ctx, integer)
        }

        fn sum_with_coeff_and_const(
            &self,
            ctx: &mut Self::Context,
            values: &[(F, Self::AssignedInteger)],
            constant: F,
        ) -> Result<Self::AssignedInteger, Error> {
            self.compose(
                ctx,
                &values
                    .iter()
                    .map(|(coeff, assigned)| Term::Assigned(assigned, *coeff))
                    .collect_vec(),
                constant,
            )
        }

        fn sum_products_with_coeff_and_const(
            &self,
            ctx: &mut Self::Context,
            values: &[(F, Self::AssignedInteger, Self::AssignedInteger)],
            constant: F,
        ) -> Result<Self::AssignedInteger, Error> {
            match values.len() {
                0 => MainGateInstructions::assign_constant(self, ctx, constant),
                1 => {
                    let (scalar, lhs, rhs) = &values[0];
                    let output = lhs
                        .value()
                        .zip(rhs.value())
                        .map(|(lhs, rhs)| *scalar * lhs * rhs + constant);

                    Ok(self
                        .apply(
                            ctx,
                            [
                                Term::Zero,
                                Term::Zero,
                                Term::assigned_to_mul(lhs),
                                Term::assigned_to_mul(rhs),
                                Term::unassigned_to_sub(output),
                            ],
                            constant,
                            CombinationOption::OneLinerDoubleMul(*scalar),
                        )?
                        .swap_remove(4))
                }
                _ => {
                    let (scalar, lhs, rhs) = &values[0];
                    self.apply(
                        ctx,
                        [Term::assigned_to_mul(lhs), Term::assigned_to_mul(rhs)],
                        constant,
                        CombinationOptionCommon::CombineToNextScaleMul(-F::one(), *scalar).into(),
                    )?;
                    let acc =
                        Value::known(*scalar) * lhs.value() * rhs.value() + Value::known(constant);
                    let output = values.iter().skip(1).fold(
                        Ok::<_, Error>(acc),
                        |acc, (scalar, lhs, rhs)| {
                            acc.and_then(|acc| {
                                self.apply(
                                    ctx,
                                    [
                                        Term::assigned_to_mul(lhs),
                                        Term::assigned_to_mul(rhs),
                                        Term::Zero,
                                        Term::Zero,
                                        Term::Unassigned(acc, F::one()),
                                    ],
                                    F::zero(),
                                    CombinationOptionCommon::CombineToNextScaleMul(
                                        -F::one(),
                                        *scalar,
                                    )
                                    .into(),
                                )?;
                                Ok(acc + Value::known(*scalar) * lhs.value() * rhs.value())
                            })
                        },
                    )?;
                    self.apply(
                        ctx,
                        [
                            Term::Zero,
                            Term::Zero,
                            Term::Zero,
                            Term::Zero,
                            Term::Unassigned(output, F::zero()),
                        ],
                        F::zero(),
                        CombinationOptionCommon::OneLinerAdd.into(),
                    )
                    .map(|mut outputs| outputs.swap_remove(4))
                }
            }
        }

        fn sub(
            &self,
            ctx: &mut Self::Context,
            a: &Self::AssignedInteger,
            b: &Self::AssignedInteger,
        ) -> Result<Self::AssignedInteger, Error> {
            MainGateInstructions::sub(self, ctx, a, b)
        }

        fn neg(
            &self,
            ctx: &mut Self::Context,
            a: &Self::AssignedInteger,
        ) -> Result<Self::AssignedInteger, Error> {
            MainGateInstructions::neg_with_constant(self, ctx, a, F::zero())
        }

        fn invert(
            &self,
            ctx: &mut Self::Context,
            a: &Self::AssignedInteger,
        ) -> Result<Self::AssignedInteger, Error> {
            MainGateInstructions::invert_unsafe(self, ctx, a)
        }

        fn assert_equal(
            &self,
            ctx: &mut Self::Context,
            a: &Self::AssignedInteger,
            b: &Self::AssignedInteger,
        ) -> Result<(), Error> {
            let mut eq = true;
            a.value().zip(b.value()).map(|(lhs, rhs)| {
                eq &= lhs == rhs;
            });
            MainGateInstructions::assert_equal(self, ctx, a, b)
                .and(eq.then_some(()).ok_or(Error::Synthesis))
        }
    }

    impl<'a, C: CurveAffine, const LIMBS: usize, const BITS: usize> EccInstructions<'a, C>
        for BaseFieldEccChip<C, LIMBS, BITS>
    {
        type Context = RegionCtx<'a, C::Scalar>;
        type ScalarChip = MainGate<C::Scalar>;
        type AssignedEcPoint = AssignedPoint<C::Base, C::Scalar, LIMBS, BITS>;
        type Scalar = C::Scalar;
        type AssignedScalar = AssignedCell<C::Scalar, C::Scalar>;

        fn scalar_chip(&self) -> &Self::ScalarChip {
            self.main_gate()
        }

        fn assign_constant(
            &self,
            ctx: &mut Self::Context,
            point: C,
        ) -> Result<Self::AssignedEcPoint, Error> {
            self.assign_constant(ctx, point)
        }

        fn assign_point(
            &self,
            ctx: &mut Self::Context,
            point: Value<C>,
        ) -> Result<Self::AssignedEcPoint, Error> {
            self.assign_point(ctx, point)
        }

        fn sum_with_const(
            &self,
            ctx: &mut Self::Context,
            values: &[Self::AssignedEcPoint],
            constant: C,
        ) -> Result<Self::AssignedEcPoint, Error> {
            if values.is_empty() {
                return self.assign_constant(ctx, constant);
            }

            iter::empty()
                .chain(
                    (!bool::from(constant.is_identity()))
                        .then(|| self.assign_constant(ctx, constant)),
                )
                .chain(values.iter().cloned().map(Ok))
                .reduce(|acc, ec_point| self.add(ctx, &acc?, &ec_point?))
                .unwrap()
        }

        fn fixed_base_msm(
            &mut self,
            ctx: &mut Self::Context,
            pairs: &[(Self::AssignedScalar, C)],
        ) -> Result<Self::AssignedEcPoint, Error> {
            // FIXME: Implement fixed base MSM in halo2_wrong
            let pairs = pairs
                .iter()
                .map(|(scalar, base)| {
                    Ok::<_, Error>((scalar.clone(), self.assign_constant(ctx, *base)?))
                })
                .collect::<Result<Vec<_>, _>>()?;
            self.variable_base_msm(ctx, &pairs)
        }

        fn variable_base_msm(
            &mut self,
            ctx: &mut Self::Context,
            pairs: &[(Self::AssignedScalar, Self::AssignedEcPoint)],
        ) -> Result<Self::AssignedEcPoint, Error> {
            const WINDOW_SIZE: usize = 3;
            let pairs = pairs
                .iter()
                .map(|(scalar, base)| (base.clone(), scalar.clone()))
                .collect_vec();
            match self.mul_batch_1d_horizontal(ctx, pairs.clone(), WINDOW_SIZE) {
                Err(_) => {
                    if self.assign_aux(ctx, WINDOW_SIZE, pairs.len()).is_err() {
                        let aux_generator = Value::known(C::Curve::random(OsRng).into());
                        self.assign_aux_generator(ctx, aux_generator)?;
                        self.assign_aux(ctx, WINDOW_SIZE, pairs.len())?;
                    }
                    self.mul_batch_1d_horizontal(ctx, pairs, WINDOW_SIZE)
                }
                result => result,
            }
        }

        fn normalize(
            &self,
            ctx: &mut Self::Context,
            point: &Self::AssignedEcPoint,
        ) -> Result<Self::AssignedEcPoint, Error> {
            self.normalize(ctx, point)
        }

        fn assert_equal(
            &self,
            ctx: &mut Self::Context,
            a: &Self::AssignedEcPoint,
            b: &Self::AssignedEcPoint,
        ) -> Result<(), Error> {
            let mut eq = true;
            [(a.x(), b.x()), (a.y(), b.y())].map(|(lhs, rhs)| {
                lhs.integer().zip(rhs.integer()).map(|(lhs, rhs)| {
                    eq &= lhs.value() == rhs.value();
                });
            });
            self.assert_equal(ctx, a, b)
                .and(eq.then_some(()).ok_or(Error::Synthesis))
        }
    }
}
