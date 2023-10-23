use crate::cross::{CrossVec, CrossVecPair, CrossVecPairGuardState};

use alloc::vec;

#[test]
fn cross_vec_pair_guard_state() {
    let pair: CrossVecPair<()> = CrossVecPair(vec![], vec![]);
    assert!(CrossVecPairGuardState::<()>::NotTakenYet(pair).is_not_taken_yet());

    assert!(CrossVecPairGuardState::<()>::TakenOut.is_taken_out());
    assert!(CrossVecPairGuardState::<()>::MovedBack.is_moved_back());
}
