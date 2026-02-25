#![no_std]

macro_rules! extreme_macro {
    ($($t:tt)*) => { $($t)* };
}

extreme_macro! {
    use soroban_sdk::{contract, contractimpl, Env, Address};
    
    #[contract]
    pub struct EmptyMacroContract;

    #[contractimpl]
    impl EmptyMacroContract {
    }
}
