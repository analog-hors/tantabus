use std::fmt::{Display, Formatter};

mod eval;
mod pst;
mod eval_set;
mod mob;
mod trace;
mod eval_consts;
pub mod phased_eval;

pub use eval::*;
pub use eval_consts::*;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Eval(i16);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EvalKind {
    Centipawn(i16),
    MateIn(u8),
    MatedIn(u8)
}

impl Display for Eval {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.kind().fmt(f)
    }
}

impl Display for EvalKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match *self {
            EvalKind::Centipawn(cp) => {
                if cp < 0 {
                    write!(f, "-")?;
                }
                write!(f, "{}.{}", cp.abs() / 100, cp.abs() % 100)
            },
            EvalKind::MateIn(m) => write!(f, "M{}", (m + 1) / 2),
            EvalKind::MatedIn(m) => write!(f, "-M{}", (m + 1) / 2)
        }
    }
}

impl Eval {
    pub const ZERO: Self = Self(0);

    pub const DRAW: Self = Self::ZERO;

    pub const MAX: Self = Self(i16::MAX);

    pub const MIN: Self = Self(-Self::MAX.0);

    pub const UNIT: Self = Self(1);

    const MATE_IN_ZERO: Self = Self(i16::MAX - 100);

    pub const fn cp(centipawns: i16) -> Self {
        Self(centipawns)
    }

    pub const fn mate_in(plies_to_mate: u8) -> Self {
        Self(Self::MATE_IN_ZERO.0 - plies_to_mate as i16)
    }

    pub const fn mated_in(plies_to_mate: u8) -> Self {
        Self(-Self::mate_in(plies_to_mate).0)
    }

    pub const fn kind(self) -> EvalKind {
        const MAX_MATE_IN: i16 = Eval::mate_in(u8::MAX).0;
        const MIN_MATE_IN: i16 = Eval::mate_in(u8::MIN).0;
        const MAX_MATED_IN: i16 = Eval::mated_in(u8::MAX).0;
        const MIN_MATED_IN: i16 = Eval::mated_in(u8::MIN).0;
        
        match self.0 {
            v if v >= MAX_MATE_IN => EvalKind::MateIn((MIN_MATE_IN - v) as u8),
            v if v <= MAX_MATED_IN => EvalKind::MatedIn((v - MIN_MATED_IN) as u8),
            v => EvalKind::Centipawn(v),
        }
    }

    pub const fn as_cp(self) -> Option<i16> {
        if let EvalKind::Centipawn(cp) = self.kind() {
            Some(cp)
        } else {
            None
        }
    }
}

macro_rules! impl_math_ops {
    ($($trait:ident::$fn:ident),*) => {
        $(
            impl std::ops::$trait for Eval {
                type Output = Self;

                #[inline(always)]
                fn $fn(self, other: Self) -> Self::Output {
                    Self(std::ops::$trait::$fn(self.0, other.0))
                }
            }
        )*
    };
}
impl_math_ops! {
    Add::add,
    Sub::sub,
    Mul::mul,
    Div::div
}

macro_rules! impl_math_assign_ops {
    ($($trait:ident::$fn:ident),*) => {
        $(impl std::ops::$trait for Eval {
            #[inline(always)]
            fn $fn(&mut self, other: Self) {
                std::ops::$trait::$fn(&mut self.0, other.0)
            }
        })*
    };
}
impl_math_assign_ops! {
    AddAssign::add_assign,
    SubAssign::sub_assign,
    MulAssign::mul_assign,
    DivAssign::div_assign
}

impl std::ops::Neg for Eval {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self(-self.0)
    }
}

macro_rules! impl_saturating_math_ops {
    ($($fn:ident),*) => {
        impl Eval {$(
            #[inline(always)]
            pub const fn $fn(self, other: Self) -> Self {
                Self(self.0.$fn(other.0))
            }
        )*}
    };
}
impl_saturating_math_ops! {
    saturating_add,
    saturating_mul
}

impl Eval {
    #[inline(always)]
    pub const fn saturating_sub(self, other: Self) -> Self {
        let mut result = self.0.saturating_sub(other.0);
        if result == i16::MIN {
            result += 1;
        }
        Self(result)
    }
}
