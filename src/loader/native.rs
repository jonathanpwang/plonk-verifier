use crate::{
    loader::{EcPointLoader, LoadedEcPoint, LoadedScalar, Loader, ScalarLoader},
    util::arithmetic::{Curve, CurveAffine, FieldOps, PrimeField},
    Error,
};
use lazy_static::lazy_static;
use std::fmt::Debug;

lazy_static! {
    pub static ref LOADER: NativeLoader = NativeLoader;
}

#[derive(Clone, Debug)]
pub struct NativeLoader;

impl<C: CurveAffine> LoadedEcPoint<C> for C {
    type Loader = NativeLoader;

    fn loader(&self) -> &NativeLoader {
        &LOADER
    }

    fn multi_scalar_multiplication(pairs: impl IntoIterator<Item = (C::Scalar, C)>) -> Self {
        pairs
            .into_iter()
            .map(|(scalar, base)| base * scalar)
            .reduce(|acc, value| acc + value)
            .unwrap()
            .to_affine()
    }
}

impl<F: PrimeField> FieldOps for F {
    fn invert(&self) -> Option<F> {
        self.invert().into()
    }
}

impl<F: PrimeField> LoadedScalar<F> for F {
    type Loader = NativeLoader;

    fn loader(&self) -> &NativeLoader {
        &LOADER
    }

    fn mul_add(a: &F, b: &F, c: &F) -> Self {
        *a * *b + *c
    }

    fn mul_add_constant(a: &F, b: &F, c: &F) -> Self {
        *a * *b + *c
    }
}

impl<C: CurveAffine> EcPointLoader<C> for NativeLoader {
    type LoadedEcPoint = C;

    fn ec_point_load_const(&self, value: &C) -> Self::LoadedEcPoint {
        *value
    }

    fn ec_point_assert_eq(
        &self,
        annotation: &str,
        lhs: &Self::LoadedEcPoint,
        rhs: &Self::LoadedEcPoint,
    ) -> Result<(), Error> {
        lhs.eq(rhs).then_some(()).ok_or_else(|| Error::AssertionFailure(annotation.to_string()))
    }
}

impl<F: PrimeField> ScalarLoader<F> for NativeLoader {
    type LoadedScalar = F;

    fn load_const(&self, value: &F) -> Self::LoadedScalar {
        *value
    }

    fn assert_eq(
        &self,
        annotation: &str,
        lhs: &Self::LoadedScalar,
        rhs: &Self::LoadedScalar,
    ) -> Result<(), Error> {
        lhs.eq(rhs).then_some(()).ok_or_else(|| Error::AssertionFailure(annotation.to_string()))
    }
}

impl<C: CurveAffine> Loader<C> for NativeLoader {}
