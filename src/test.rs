#![cfg(test)]

extern crate std;

use crate::{MarketplaceContract, MarketplaceContractClient};
use soroban_sdk::{
    testutils::{Address as _, AuthorizedFunction, AuthorizedInvocation, Events},
    token::{self, Client},
    vec, Address, Env, IntoVal, Symbol, Val, Vec,
};

fn assert_auth(
    auths: &[(Address, AuthorizedInvocation)],
    idx: usize,
    auth_addr: Address,
    call_addr: Address,
    func: Symbol,
    args: Vec<Val>,
) {
    let auth = auths.get(idx).unwrap();
    assert_eq!(auth.0, auth_addr);
    assert_eq!(
        auth.1.function,
        AuthorizedFunction::Contract((call_addr, func, args))
    );
}

fn setup_test<'a>() -> (
    Env,
    MarketplaceContractClient<'a>,
    Client<'a>,
    token::StellarAssetClient<'a>,
    Client<'a>,
    token::StellarAssetClient<'a>,
    Address,
    Address,
) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, MarketplaceContract);
    let contract_client: MarketplaceContractClient<'_> =
        MarketplaceContractClient::new(&env, &contract_id);

    let seller = Address::random(&env);
    let buyer = Address::random(&env);

    let token_admin_client = create_token_asset(&env, &Address::random(&env));
    let token_client = token::Client::new(&env, &token_admin_client.address);

    contract_client.init(&token_client.address, &Address::random(&env));
    let asset_admin_client = create_token_asset(&env, &Address::random(&env));
    let asset_client = token::Client::new(&env, &asset_admin_client.address);

    (
        env,
        contract_client,
        token_client,
        token_admin_client,
        asset_client,
        asset_admin_client,
        seller,
        buyer,
    )
}

fn create_token_asset<'a>(e: &Env, admin: &Address) -> token::StellarAssetClient<'a> {
    token::StellarAssetClient::new(e, &e.register_stellar_asset_contract(admin.clone()))
}

#[test]
#[should_panic(expected = "Error(Contract, #1)")]
fn cannot_initialize_marketplace_twice() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, MarketplaceContract);
    let client: MarketplaceContractClient<'_> = MarketplaceContractClient::new(&env, &contract_id);

    let address = Address::random(&env); // Address just for satisfying interfaces.
    client.init(&address, &address);
    client.init(&address, &address);
}

#[test]
fn can_create_listing() {
    let (
        env,
        contract_client,
        _token_client,
        _token_admin_client,
        asset_client,
        asset_admin_client,
        seller,
        _buyer,
    ) = setup_test();

    asset_admin_client.mint(&seller, &2); // Seller has 2 NFTs.

    let id = contract_client.create_listing(&seller, &asset_admin_client.address, &100, &2);

    assert_auth(
        &env.auths(),
        0,
        seller.clone(),
        contract_client.address.clone(),
        Symbol::new(&env, "create_listing"),
        (seller.clone(), asset_admin_client.address, 100i128, 2i128).into_val(&env),
    );

    let listing = contract_client.get_listing(&id).unwrap();

    assert_eq!(&listing.id, &1);
    assert_eq!(&listing.listed, &true);
    assert_eq!(&listing.owner, &seller);

    assert_eq!(asset_client.balance(&contract_client.address), 2); // Now the contract has the ownership of the NFTs.
    assert_eq!(&listing.price, &100);
    assert_eq!(&listing.quantity, &2);

    let last_events = env.events().all().slice(env.events().all().len() - 1..);
    assert_eq!(
        last_events,
        vec![
            &env,
            (
                contract_client.address.clone(),
                (Symbol::new(&env, "create_listing"), seller.clone()).into_val(&env),
                1u64.into_val(&env)
            ),
        ]
    )
}

#[test]
fn create_listing_increments_id() {
    let (
        _env,
        contract_client,
        _token_client,
        _token_admin_client,
        _asset_client,
        asset_admin_client,
        seller,
        _buyer,
    ) = setup_test();

    asset_admin_client.mint(&seller, &4);

    let id_1 = contract_client.create_listing(&seller, &asset_admin_client.address, &100, &2);
    let id_2 = contract_client.create_listing(&seller, &asset_admin_client.address, &100, &2);

    assert_eq!(1, id_1);
    assert_eq!(2, id_2);
}

#[test]
#[should_panic(expected = "Error(Contract, #2)")]
fn cannot_create_negative_price_listing() {
    let (
        _env,
        contract_client,
        _token_client,
        _token_admin_client,
        _asset_client,
        asset_admin_client,
        seller,
        _buyer,
    ) = setup_test();
    contract_client.create_listing(&seller, &asset_admin_client.address, &-100, &2);
}

#[test]
#[should_panic(expected = "Error(Contract, #3)")]
fn cannot_create_listing_if_not_balance() {
    let (
        _env,
        contract_client,
        _token_client,
        _token_admin_client,
        _asset_client,
        asset_admin_client,
        seller,
        _buyer,
    ) = setup_test();

    contract_client.create_listing(&seller, &asset_admin_client.address, &100, &2);
}

#[test]
#[should_panic(expected = "Error(Contract, #2)")]
fn cannot_create_zero_price_listing() {
    let (
        _env,
        contract_client,
        _token_client,
        _token_admin_client,
        _asset_client,
        asset_admin_client,
        seller,
        _buyer,
    ) = setup_test();

    asset_admin_client.mint(&seller, &2);
    contract_client.create_listing(&seller, &asset_admin_client.address, &0, &2);
}

#[test]
#[should_panic(expected = "Error(Contract, #7)")]
fn cannot_create_negative_quantity_listing() {
    let (
        _env,
        contract_client,
        _token_client,
        _token_admin_client,
        _asset_client,
        asset_admin_client,
        seller,
        _buyer,
    ) = setup_test();

    asset_admin_client.mint(&seller, &2);
    contract_client.create_listing(&seller, &asset_admin_client.address, &100, &-1);
}

#[test]
#[should_panic(expected = "Error(Contract, #7)")]
fn cannot_create_zero_quantity_listing() {
    let (
        _env,
        contract_client,
        _token_client,
        _token_admin_client,
        _asset_client,
        asset_admin_client,
        seller,
        _buyer,
    ) = setup_test();

    asset_admin_client.mint(&seller, &2);
    contract_client.create_listing(&seller, &asset_admin_client.address, &100, &0);
}

#[test]
fn can_complete_a_sell_operation() {
    let (
        env,
        contract_client,
        token_client,
        token_admin_client,
        asset_client,
        asset_admin_client,
        seller,
        buyer,
    ) = setup_test();

    // Prepare the marketplace
    asset_admin_client.mint(&seller, &2);
    let id = contract_client.create_listing(&seller, &asset_client.address, &100, &2);
    token_admin_client.mint(&buyer, &200);

    // Buy !
    contract_client.buy_listing(&buyer, &id);

    assert_auth(
        &env.auths(),
        0,
        buyer.clone(),
        contract_client.address.clone(),
        Symbol::new(&env, "buy_listing"),
        (buyer.clone(), id).into_val(&env),
    );

    assert_eq!(asset_client.balance(&contract_client.address), 0); 
    assert_eq!(asset_client.balance(&seller), 0); 
    assert_eq!(asset_client.balance(&buyer), 2); 

    assert_eq!(
        &token_client.balance(&seller),
        &200
    );
    assert_eq!(
        &token_client.balance(&buyer), 
        &0
    );

    let last_events = env.events().all().slice(env.events().all().len() - 1..);
    assert_eq!(
        last_events,
        vec![
            &env,
            (
                contract_client.address.clone(),
                (Symbol::new(&env, "buy_listing"), buyer.clone()).into_val(&env),
                id.into_val(&env)
            ),
        ]
    );


    assert!(contract_client.get_listing(&id).is_none())
}

#[test]
#[should_panic(expected = "Error(Contract, #3)")]
fn cannot_buy_if_not_enough_balance() {
    let (
        _env,
        contract_client,
        _token_client,
        token_admin_client,
        asset_client,
        asset_admin_client,
        seller,
        buyer,
    ) = setup_test();

    // Prepare the marketplace
    asset_admin_client.mint(&seller, &2);
    let id = contract_client.create_listing(&seller, &asset_client.address, &100, &2);
    token_admin_client.mint(&buyer, &199);

    // Buy !
    contract_client.buy_listing(&buyer, &id);
}

#[test]
#[should_panic(expected = "Error(Contract, #4)")]
fn cannot_sell_when_unlisted() {
    let (
        _env,
        contract_client,
        _token_client,
        token_admin_client,
        _asset_client,
        asset_admin_client,
        seller,
        buyer,
    ) = setup_test();

    asset_admin_client.mint(&seller, &2);
    token_admin_client.mint(&buyer, &400);

    let id = contract_client.create_listing(&seller, &asset_admin_client.address, &100, &2);
    contract_client.pause_listing(&id);
    contract_client.buy_listing(&buyer, &id);
}

#[test]
fn can_update_a_listing() {
    let (
        env,
        contract_client,
        _token_client,
        _token_admin_client,
        _asset_client,
        asset_admin_client,
        seller,
        _buyer,
    ) = setup_test();

    asset_admin_client.mint(&seller, &10);
    let id = contract_client.create_listing(&seller, &asset_admin_client.address, &100, &2);

    contract_client.update_price(&id, &200);

    assert_auth(
        &env.auths(),
        0,
        seller.clone(),
        contract_client.address.clone(),
        Symbol::new(&env, "update_price"),
        (id, 200i128).into_val(&env),
    );

    let listing = contract_client.get_listing(&id).unwrap();
    assert_eq!(listing.price, 200);

    let last_events = env.events().all().slice(env.events().all().len() - 1..);
    assert_eq!(
        last_events,
        vec![
            &env,
            (
                contract_client.address.clone(),
                (Symbol::new(&env, "update_price"), seller.clone()).into_val(&env),
                id.into_val(&env)
            ),
        ]
    );
}

#[test]
fn can_pause_a_listing() {
    let (
        env,
        contract_client,
        _token_client,
        _token_admin_client,
        _asset_client,
        asset_admin_client,
        seller,
        _buyer,
    ) = setup_test();

    asset_admin_client.mint(&seller, &2);
    let id = contract_client.create_listing(&seller, &asset_admin_client.address, &100, &2);

    contract_client.pause_listing(&id);

    assert_auth(
        &env.auths(),
        0,
        seller.clone(),
        contract_client.address.clone(),
        Symbol::new(&env, "pause_listing"),
        (id,).into_val(&env),
    );

    let listing = contract_client.get_listing(&id).unwrap();
    assert_eq!(&listing.listed, &false);

    let last_events = env.events().all().slice(env.events().all().len() - 1..);
    assert_eq!(
        last_events,
        vec![
            &env,
            (
                contract_client.address.clone(),
                (Symbol::new(&env, "pause_listing"), seller.clone()).into_val(&env),
                id.into_val(&env)
            ),
        ]
    )
}

#[test]
fn can_unpause_a_listing() {
    let (
        env,
        contract_client,
        _token_client,
        _token_admin_client,
        _asset_client,
        asset_admin_client,
        seller,
        _buyer,
    ) = setup_test();

    asset_admin_client.mint(&seller, &2);
    let id = contract_client.create_listing(&seller, &asset_admin_client.address, &100, &2);

    contract_client.pause_listing(&id);
    contract_client.unpause_listing(&id);

    assert_auth(
        &env.auths(),
        0,
        seller.clone(),
        contract_client.address.clone(),
        Symbol::new(&env, "unpause_listing"),
        (id,).into_val(&env),
    );

    let listing = contract_client.get_listing(&id).unwrap();
    assert_eq!(&listing.listed, &true);

    let last_events = env.events().all().slice(env.events().all().len() - 1..);
    assert_eq!(
        last_events,
        vec![
            &env,
            (
                contract_client.address.clone(),
                (Symbol::new(&env, "unpause_listing"), seller.clone()).into_val(&env),
                id.into_val(&env)
            ),
        ]
    )
}

#[test]
fn can_remove_a_listing() {
    let (
        env,
        contract_client,
        _token_client,
        _token_admin_client,
        asset_client,
        asset_admin_client,
        seller,
        _buyer,
    ) = setup_test();

    asset_admin_client.mint(&seller, &2);

    let id: u64 = contract_client.create_listing(&seller, &asset_admin_client.address, &100, &2);
    contract_client.remove_listing(&id);

    assert_auth(
        &env.auths(),
        0,
        seller.clone(),
        contract_client.address.clone(),
        Symbol::new(&env, "remove_listing"),
        (id,).into_val(&env),
    );

    let listing = contract_client.get_listing(&id);
    assert!(listing.is_none());

    // Ownership is returned to the original owners (sellers)
    assert_eq!(0, asset_client.balance(&contract_client.address));
    assert_eq!(2, asset_client.balance(&seller));

    let last_events = env.events().all().slice(env.events().all().len() - 1..);
    assert_eq!(
        last_events,
        vec![
            &env,
            (
                contract_client.address.clone(),
                (Symbol::new(&env, "remove_listing"), seller.clone()).into_val(&env),
                id.into_val(&env)
            ),
        ]
    )
}

#[test]
#[should_panic(expected = "Error(Contract, #2)")]
fn cannot_do_negative_update() {
    let (
        _env,
        contract_client,
        _token_client,
        _token_admin_client,
        _asset_client,
        asset_admin_client,
        seller,
        _buyer,
    ) = setup_test();

    asset_admin_client.mint(&seller, &2);

    let id = contract_client.create_listing(&seller, &asset_admin_client.address, &100, &2);
    contract_client.update_price(&id, &-100)
}

#[test]
#[should_panic(expected = "Error(Contract, #2)")]
fn cannot_do_zero_update() {
    let (
        _env,
        contract_client,
        _token_client,
        _token_admin_client,
        _asset_client,
        asset_admin_client,
        seller,
        _buyer,
    ) = setup_test();

    asset_admin_client.mint(&seller, &2);

    let id = contract_client.create_listing(&seller, &asset_admin_client.address, &100, &2);
    contract_client.update_price(&id, &0)
}

#[test]
#[should_panic(expected = "Error(Contract, #6)")]
fn cannot_create_listing_without_initialize() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, MarketplaceContract);
    let client: MarketplaceContractClient<'_> = MarketplaceContractClient::new(&env, &contract_id);
    client.create_listing(&Address::random(&env), &Address::random(&env), &1, &1);
}

#[test]
#[should_panic(expected = "Error(Contract, #6)")]
fn cannot_buy_listing_without_initialize() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, MarketplaceContract);
    let client: MarketplaceContractClient<'_> = MarketplaceContractClient::new(&env, &contract_id);
    client.buy_listing(&Address::random(&env), &1);
}

#[test]
#[should_panic(expected = "Error(Contract, #6)")]
fn cannot_get_listing_without_initialize() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, MarketplaceContract);
    let client: MarketplaceContractClient<'_> = MarketplaceContractClient::new(&env, &contract_id);
    client.get_listing(&1).unwrap();
}

#[test]
#[should_panic(expected = "Error(Contract, #6)")]
fn cannot_pause_listing_without_initialize() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, MarketplaceContract);
    let client: MarketplaceContractClient<'_> = MarketplaceContractClient::new(&env, &contract_id);
    client.pause_listing(&1);
}

#[test]
#[should_panic(expected = "Error(Contract, #6)")]
fn cannot_unpause_listing_without_initialize() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, MarketplaceContract);
    let client: MarketplaceContractClient<'_> = MarketplaceContractClient::new(&env, &contract_id);
    client.unpause_listing(&1);
}

#[test]
#[should_panic(expected = "Error(Contract, #6)")]
fn cannot_update_price_listing_without_initialize() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, MarketplaceContract);
    let client: MarketplaceContractClient<'_> = MarketplaceContractClient::new(&env, &contract_id);
    client.update_price(&1, &1);
}

#[test]
#[should_panic(expected = "Error(Contract, #6)")]
fn cannot_remove_listing_without_initialize() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, MarketplaceContract);
    let client: MarketplaceContractClient<'_> = MarketplaceContractClient::new(&env, &contract_id);
    client.remove_listing(&1);
}
