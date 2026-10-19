//! Fixture for the `missing_ttl_bump` rule: a notebook that writes to
//! persistent storage in `save_note` and never extends the entry's TTL, so the
//! note silently expires. The caller is authenticated and an event is emitted,
//! so `missing_ttl_bump` is the only rule that should fire.
use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env, String};

#[contract]
pub struct Notebook;

#[contractimpl]
impl Notebook {
    pub fn save_note(env: Env, author: Address, note: String) {
        author.require_auth();
        let key = symbol_short!("note");
        env.storage().persistent().set(&key, &note);
        env.events().publish((symbol_short!("saved"), author), note);
    }

    pub fn read_note(env: Env) -> Option<String> {
        env.storage().persistent().get(&symbol_short!("note"))
    }
}
