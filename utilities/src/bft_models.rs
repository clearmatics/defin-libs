//! BFT (Byzantine fault tolerance) models
//!
//! This is an abstraction for quorum and fault computing for primary BFT consensus:
//!
//! - [`N3F1`]: The system model needs: `n = 3*f + 1`, Tendermint, Simplex, etc...
//!
//! - [`N5F1`]: The system model needs: `n = 5*f + 1`, SNARE, etc...
//!
//! where `n` denotes the total weight of participants.
//! where `f` denotes the max weight of faulty participants.
//!
//! # Example
//!
//! ```
//! use utilities::bft_models::{BFTError, BFT, N3F1, N5F1};
//!
//! // n = 3*f + 1
//! let n = 21;
//! assert_eq!(N3F1::max_faults(n).unwrap(), 6);
//! assert_eq!(N3F1::quorum(n).unwrap(), 15);
//!
//! // n = 5*f + 1
//! let n = 21;
//! assert_eq!(N5F1::max_faults(n).unwrap(), 4);
//! assert_eq!(N5F1::quorum(n).unwrap(), 17);
//!```

use thiserror::Error;
use num_traits::ToPrimitive;

/// Errors of invalid parameter
#[derive(Debug, Error, PartialEq, Eq)]
pub enum BFTError {
    /// Wrong type of n.
    #[error("n must be an integer")]
    WrongType,

    /// Two small value of n.
    #[error("n must be at least {min}, got {got}")]
    TooSmall{min: u64, got: u64},
}

/// An abstraction of BFT model that computes quorum(Q) and F.
/// All the parameters are bounded by trait ToPrimitives, thus that
/// callers can use i16, i32, i64, u32, u64, usize, etc. There isn't
/// any explicit type conversion. The output is Result<u64, BFTError>.
pub trait BFT {
    /// Compute the quorum weight of participants in a BFT model.
    fn quorum(n: impl ToPrimitive) -> Result<u64, BFTError>;
    /// Compute the maximum weight of faulty participants in a BFT model.
    fn max_faults(n: impl ToPrimitive) -> Result<u64, BFTError>;
}

/// BFT model with `n = 3*f + 1` as the participant setup.
pub struct N3F1;

impl BFT for N3F1 {
    #[inline]
    fn quorum(n: impl ToPrimitive) -> Result<u64, BFTError> {
        let n = n.to_u64().ok_or(BFTError::WrongType)?;
        if n < 4 {
            return Err(BFTError::TooSmall {min: 4, got: n})
        }
        Ok(n - (n-1)/3)
    }

    #[inline]
    fn max_faults(n: impl ToPrimitive) -> Result<u64, BFTError> {
        let n = n.to_u64().ok_or(BFTError::WrongType)?;
        if n < 4 {
            return Err(BFTError::TooSmall {min: 4, got: n})
        }
        Ok((n-1) / 3)
    }
}

/// BFT model with `n = 5*f + 1` as the participant setup.
pub struct N5F1;
impl BFT for N5F1 {
    #[inline]
    fn quorum(n: impl ToPrimitive) -> Result<u64, BFTError> {
        let n = n.to_u64().ok_or(BFTError::WrongType)?;
        if n < 6 {
            return Err(BFTError::TooSmall {min: 6, got: n})
        }
        Ok(n - ((n-1)/5))
    }
    #[inline]
    fn max_faults(n: impl ToPrimitive) -> Result<u64, BFTError> {
        let n = n.to_u64().ok_or(BFTError::WrongType)?;
        if n < 6 {
            return Err(BFTError::TooSmall {min: 6, got: n})
        }
        Ok((n-1) / 5)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;
    use proptest::prelude::*;

    #[test]
    fn test_wrong_type() {
        assert_eq!(N3F1::quorum(-1), Err(BFTError::WrongType));
        assert_eq!(N3F1::quorum(-3.14), Err(BFTError::WrongType));
        assert_eq!(N3F1::max_faults(-1), Err(BFTError::WrongType));
        assert_eq!(N3F1::max_faults(-3.14), Err(BFTError::WrongType));

        assert_eq!(N5F1::quorum(-1), Err(BFTError::WrongType));
        assert_eq!(N5F1::quorum(-3.14), Err(BFTError::WrongType));
        assert_eq!(N5F1::max_faults(-1), Err(BFTError::WrongType));
        assert_eq!(N5F1::max_faults(-3.14), Err(BFTError::WrongType));
    }

    #[test]
    fn test_two_small() {
        assert_eq!(N3F1::quorum(0), Err(BFTError::TooSmall {min: 4, got: 0}));
        assert_eq!(N3F1::quorum(1), Err(BFTError::TooSmall {min: 4, got: 1}));
        assert_eq!(N3F1::quorum(2), Err(BFTError::TooSmall {min: 4, got: 2}));
        assert_eq!(N3F1::quorum(3), Err(BFTError::TooSmall {min: 4, got: 3}));

        assert_eq!(N5F1::quorum(0), Err(BFTError::TooSmall {min: 6, got: 0}));
        assert_eq!(N5F1::quorum(1), Err(BFTError::TooSmall {min: 6, got: 1}));
        assert_eq!(N5F1::quorum(2), Err(BFTError::TooSmall {min: 6, got: 2}));
        assert_eq!(N5F1::quorum(3), Err(BFTError::TooSmall {min: 6, got: 3}));
        assert_eq!(N5F1::quorum(4), Err(BFTError::TooSmall {min: 6, got: 4}));
        assert_eq!(N5F1::quorum(5), Err(BFTError::TooSmall {min: 6, got: 5}));
    }

    #[rstest]
    // case(N, F, Quorum) for N3F1.
    #[case(4, 1, 3)]
    #[case(5, 1, 4)]
    #[case(6, 1, 5)]
    #[case(7, 2, 5)]
    #[case(8, 2, 6)]
    #[case(9, 2, 7)]
    #[case(10, 3, 7)]
    #[case(11, 3, 8)]
    #[case(12, 3, 9)]
    #[case(13, 4, 9)]
    #[case(14, 4, 10)]
    #[case(15, 4, 11)]
    #[case(16, 5, 11)]
    #[case(17, 5, 12)]
    #[case(18, 5, 13)]
    #[case(19, 6, 13)]
    #[case(20, 6, 14)]
    #[case(21, 6, 15)]
    fn test_bft_n3f1(
        #[case] n: u64,
        #[case] expected_f: u64,
        #[case] expected_q: u64,
    ) {
        assert_eq!(N3F1::max_faults(n).unwrap(), expected_f);
        assert_eq!(N3F1::quorum(n).unwrap(), expected_q);
        assert_eq!(n, expected_f + expected_q);
    }


    #[rstest]
    // case(N, F, Quorum)
    #[case(6, 1, 5)]
    #[case(7, 1, 6)]
    #[case(8, 1, 7)]
    #[case(9, 1, 8)]
    #[case(10, 1, 9)]
    #[case(11, 2, 9)]
    #[case(12, 2, 10)]
    #[case(13, 2, 11)]
    #[case(14, 2, 12)]
    #[case(15, 2, 13)]
    #[case(16, 3, 13)]
    #[case(17, 3, 14)]
    #[case(18, 3, 15)]
    #[case(19, 3, 16)]
    #[case(20, 3, 17)]
    #[case(21, 4, 17)]
    fn test_bft_n5f1(
        #[case] n: u64,
        #[case] expected_f: u64,
        #[case] expected_quorum: u64,
    ) {
        assert_eq!(N5F1::max_faults(n).unwrap(), expected_f);
        assert_eq!(N5F1::quorum(n).unwrap(), expected_quorum);
        assert_eq!(n, expected_f + expected_quorum);
    }

    #[test]
    fn test_with_integer_types() {
        assert_eq!(N3F1::max_faults(10u8).unwrap(), 3);
        assert_eq!(N3F1::max_faults(10u16).unwrap(), 3);
        assert_eq!(N3F1::max_faults(10u32).unwrap(), 3);
        assert_eq!(N3F1::max_faults(10u64).unwrap(), 3);
        assert_eq!(N3F1::max_faults(10usize).unwrap(), 3);
        assert_eq!(N3F1::max_faults(10i32).unwrap(), 3);
        assert_eq!(N3F1::max_faults(10i64).unwrap(), 3);

        assert_eq!(N3F1::quorum(10u8).unwrap(), 7);
        assert_eq!(N3F1::quorum(10u16).unwrap(), 7);
        assert_eq!(N3F1::quorum(10u64).unwrap(), 7);
        assert_eq!(N3F1::quorum(10usize).unwrap(), 7);
        assert_eq!(N3F1::quorum(10i32).unwrap(), 7);
        assert_eq!(N3F1::quorum(10i64).unwrap(), 7);

        assert_eq!(N5F1::max_faults(10u64).unwrap(), 1);
        assert_eq!(N5F1::quorum(10usize).unwrap(), 9);
    }

    /// BFT safety property: there is at least one honest participant overlapped
    /// between two quorum of participants.
    #[test]
    fn test_bft_model_safety_property() -> Result<(), TestCaseError> {

        for n in 6u64..5_000 {
            // N3F1 safety
            let f_3f1 = N3F1::max_faults(n).unwrap();
            let q_3f1 = N3F1::quorum(n).unwrap();
            prop_assert!(
                2 * q_3f1 > n + f_3f1,
                "N3f1 safety property violated for n={}: 2*{} <= {} + {}",
                n, q_3f1, n, f_3f1
            );

            // N5F1 safety
            let f_5f1 = N5F1::max_faults(n).unwrap();
            let q_5f1 = N5F1::quorum(n).unwrap();
            prop_assert!(
                2 * q_5f1 > n + f_5f1,
                "N5f1 safety property violated for n={}: 2*{} <= {} + {}",
                n, q_5f1, n, f_5f1
            );
        }
        Ok(())
    }
}