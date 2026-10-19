//! Integration tests for re-entrancy through the flash loan callback.
//!
//! The borrower contracts below call back into `flashloan` from inside
//! `exec_op`, the textbook flash loan attack. The lender must reject the nested
//! borrow, must not lose funds, and must stay usable afterwards.

use flashloan_token::{FlashloanToken, FlashloanTokenClient};
use soroban_sdk::{
    contract, contractimpl, symbol_short, testutils::Address as _, token, Address, Bytes, Env,
};

#[contract]
pub struct MockToken;

#[contractimpl]
impl MockToken {
    pub fn balance(e: Env, addr: Address) -> i128 {
        e.storage().instance().get(&addr).unwrap_or(0i128)
    }
    pub fn set_balance(e: Env, addr: Address, amount: i128) {
        e.storage().instance().set(&addr, &amount);
    }
    pub fn transfer(e: Env, from: Address, to: Address, amount: i128) {
        let from_bal: i128 = e.storage().instance().get(&from).unwrap_or(0);
        let to_bal: i128 = e.storage().instance().get(&to).unwrap_or(0);
        e.storage().instance().set(&from, &(from_bal - amount));
        e.storage().instance().set(&to, &(to_bal + amount));
    }
}

/// Mock borrower that re-enters `flashloan` mid-callback.
///
/// With `strict` set it aborts when the nested borrow is rejected; otherwise it
/// records the outcome of the nested borrow and repays the outer loan.
#[contract]
pub struct ReentrantBorrower;

#[contractimpl]
impl ReentrantBorrower {
    pub fn configure(e: Env, lender: Address, strict: bool) {
        e.storage()
            .instance()
            .set(&symbol_short!("LENDER"), &lender);
        e.storage()
            .instance()
            .set(&symbol_short!("STRICT"), &strict);
    }

    pub fn nested_attempted(e: Env) -> bool {
        e.storage()
            .instance()
            .get(&symbol_short!("TRIED"))
            .unwrap_or(false)
    }

    pub fn nested_succeeded(e: Env) -> bool {
        e.storage()
            .instance()
            .get(&symbol_short!("NESTED"))
            .unwrap_or(false)
    }

    pub fn exec_op(e: Env, token_id: Address, amount: i128, fee: i128, _params: Bytes) {
        let lender: Address = e
            .storage()
            .instance()
            .get(&symbol_short!("LENDER"))
            .unwrap();
        let strict: bool = e
            .storage()
            .instance()
            .get(&symbol_short!("STRICT"))
            .unwrap();

        // Re-enter the loan function while the outer loan is outstanding.
        let nested = FlashloanTokenClient::new(&e, &lender).try_flashloan(
            &e.current_contract_address(),
            &token_id,
            &amount,
            &Bytes::new(&e),
        );
        e.storage().instance().set(&symbol_short!("TRIED"), &true);
        e.storage()
            .instance()
            .set(&symbol_short!("NESTED"), &nested.is_ok());
        if strict && nested.is_err() {
            panic!("nested flashloan rejected");
        }

        let me = e.current_contract_address();
        token::Client::new(&e, &token_id).transfer(&me, &lender, &(amount + fee));
    }
}

/// Lives in its own module because the generated contract-spec identifiers are
/// not namespaced per contract and `exec_op` is also defined above.
mod honest {
    use super::*;

    #[contract]
    pub struct HonestBorrower;

    #[contractimpl]
    impl HonestBorrower {
        pub fn set_lender(e: Env, lender: Address) {
            e.storage()
                .instance()
                .set(&symbol_short!("LENDER"), &lender);
        }

        pub fn exec_op(e: Env, token_id: Address, amount: i128, fee: i128, _params: Bytes) {
            let lender: Address = e
                .storage()
                .instance()
                .get(&symbol_short!("LENDER"))
                .unwrap();
            let me = e.current_contract_address();
            token::Client::new(&e, &token_id).transfer(&me, &lender, &(amount + fee));
        }
    }
}

use honest::{HonestBorrower, HonestBorrowerClient};

const POOL: i128 = 1_000_000;
/// Fee on a `POOL`-sized loan at the 9 bps rate set in `setup`.
const FEE: i128 = 900;

struct Setup<'a> {
    env: Env,
    lender: FlashloanTokenClient<'a>,
    lender_addr: Address,
    token_addr: Address,
    token: MockTokenClient<'a>,
}

fn setup<'a>() -> Setup<'a> {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let token_addr = env.register_contract(None, MockToken);
    let lender_addr = env.register_contract(None, FlashloanToken);
    let token = MockTokenClient::new(&env, &token_addr);
    token.set_balance(&lender_addr, &POOL);
    let lender = FlashloanTokenClient::new(&env, &lender_addr);
    lender.initialize(&admin, &9);
    Setup {
        env,
        lender,
        lender_addr,
        token_addr,
        token,
    }
}

/// A borrower that re-enters the loan function must see the nested borrow
/// rejected, and the outer loan must still settle with no funds lost.
#[test]
fn nested_flashloan_from_callback_is_rejected() {
    let s = setup();
    let borrower_addr = s.env.register_contract(None, ReentrantBorrower);
    let borrower = ReentrantBorrowerClient::new(&s.env, &borrower_addr);
    borrower.configure(&s.lender_addr, &false);
    s.token.set_balance(&borrower_addr, &FEE);

    let fee = s
        .lender
        .flashloan(&borrower_addr, &s.token_addr, &POOL, &Bytes::new(&s.env));

    assert!(borrower.nested_attempted(), "callback never re-entered");
    assert!(
        !borrower.nested_succeeded(),
        "nested flashloan was accepted while the outer loan was outstanding"
    );
    assert_eq!(fee, FEE);
    // Only the outer loan's fee moved: the rejected borrow took nothing.
    assert_eq!(s.token.balance(&s.lender_addr), POOL + FEE);
    assert_eq!(s.token.balance(&borrower_addr), 0);
    assert_eq!(s.lender.stats(), (FEE, 1));
}

/// A borrower that aborts when its nested borrow is rejected reverts the whole
/// loan: nothing is lent, no fee is booked and the loan counter does not move.
#[test]
fn loan_reverts_when_borrower_aborts_after_rejected_reentry() {
    let s = setup();
    let borrower_addr = s.env.register_contract(None, ReentrantBorrower);
    let borrower = ReentrantBorrowerClient::new(&s.env, &borrower_addr);
    borrower.configure(&s.lender_addr, &true);
    s.token.set_balance(&borrower_addr, &FEE);

    let result = s
        .lender
        .try_flashloan(&borrower_addr, &s.token_addr, &POOL, &Bytes::new(&s.env));

    assert!(result.is_err(), "loan should fail when the callback aborts");
    assert_eq!(s.token.balance(&s.lender_addr), POOL);
    assert_eq!(s.token.balance(&borrower_addr), FEE);
    assert_eq!(s.lender.stats(), (0, 0));
}

/// After a re-entry attempt, in either outcome, the guard must be released so a
/// well-behaved borrower can still take a loan.
#[test]
fn lender_stays_usable_after_reentry_attempts() {
    let s = setup();
    let attacker_addr = s.env.register_contract(None, ReentrantBorrower);
    let attacker = ReentrantBorrowerClient::new(&s.env, &attacker_addr);
    s.token.set_balance(&attacker_addr, &FEE);

    attacker.configure(&s.lender_addr, &true);
    assert!(s
        .lender
        .try_flashloan(&attacker_addr, &s.token_addr, &POOL, &Bytes::new(&s.env))
        .is_err());

    attacker.configure(&s.lender_addr, &false);
    s.lender
        .flashloan(&attacker_addr, &s.token_addr, &POOL, &Bytes::new(&s.env));

    let honest_addr = s.env.register_contract(None, HonestBorrower);
    HonestBorrowerClient::new(&s.env, &honest_addr).set_lender(&s.lender_addr);
    s.token.set_balance(&honest_addr, &FEE);
    let fee = s
        .lender
        .flashloan(&honest_addr, &s.token_addr, &POOL, &Bytes::new(&s.env));

    assert_eq!(fee, FEE);
    assert_eq!(s.lender.stats(), (2 * FEE, 2));
}
