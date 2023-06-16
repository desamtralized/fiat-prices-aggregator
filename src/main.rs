pub mod api;
pub mod fiat;
pub mod shared;

use crate::{
    api::yadio::{get_yadio_prices, Prices},
    shared::get_config,
};
use bip39::Mnemonic;
use cosmrs::{
    bip32,
    cosmwasm::MsgExecuteContract,
    crypto::secp256k1,
    rpc::HttpClient,
    tendermint::chain::Id,
    tx::{self, Fee, Msg, SignDoc, SignerInfo},
    AccountId, Coin, Denom,
};
use cosmwasm_std::Uint128;
use localmoney_protocol::price::ExecuteMsg as PriceExecuteMsg;
use localmoney_protocol::{currencies::FiatCurrency, price::CurrencyPrice};
use shared::AccountResponse;

#[tokio::main]
async fn main() {
    let yadio_request = get_yadio_prices().await;
    let price = match yadio_request {
        Ok(prices) => prices,
        Err(_) => Prices::default(),
    };

    let prices = vec![
        (price.ARS, FiatCurrency::ARS),
        (price.BRL, FiatCurrency::BRL),
        (price.CAD, FiatCurrency::CAD),
        (price.CLP, FiatCurrency::CLP),
        (price.COP, FiatCurrency::COP),
        (price.EUR, FiatCurrency::EUR),
        (price.GBP, FiatCurrency::GBP),
        (price.MXN, FiatCurrency::MXN),
        (price.NGN, FiatCurrency::NGN),
        (price.THB, FiatCurrency::THB),
        (price.VES, FiatCurrency::VES),
    ];
    let prices_json = serde_json::to_string(&prices).unwrap();
    println!("prices: {}", prices_json);
    // Derivate Wallet from Seed
    let path = "m/44'/330'/0'/0/0"
        .parse::<bip32::DerivationPath>()
        .unwrap();
    let cfg = get_config();
    let seed_words = cfg.get_string("admin_seed").unwrap();
    let mnemonic = Mnemonic::parse_normalized(&seed_words).unwrap();
    let seed = mnemonic.to_seed("");
    let sender_priv_key = secp256k1::SigningKey::derive_from_path(seed, &path).unwrap();
    let sender_pub_key = sender_priv_key.public_key();
    let addr_prefix = cfg.get_string("addr_prefix").unwrap();
    let sender_addr = sender_pub_key.account_id(&addr_prefix).unwrap();

    // Fetch Account details, we need the account sequence number
    let lcd_url = cfg.get_string("lcd").unwrap();
    let account_url = format!(
        "{}cosmos/auth/v1beta1/accounts/{}",
        lcd_url,
        sender_addr.to_string()
    );
    // print account_url
    println!("account_url: {}", account_url);
    let account_res = reqwest::get(account_url).await.unwrap();
    let account_data = account_res.json::<AccountResponse>().await;
    match &account_data {
        Ok(account_data) => println!("account_data: {:#?}", account_data),
        Err(e) => println!("account_data error: {}", e),
    }
    println!("account_data ok: {}", account_data.is_ok());
    let account_data = account_data.unwrap();
    println!("Account sequence is {}", account_data.account.sequence);

    // Send Tx to Contract
    let contract_addr = cfg.get_string("price_addr").unwrap();
    let mut tx_body_builder = tx::BodyBuilder::new();
    let mut currency_prices: Vec<CurrencyPrice> = vec![];
    prices.iter().for_each(|price_fiat| {
        // TODO: although most currencies have 2 decimals,
        // some currencies like JPY have 3 and some exotic currencies have zero.
        let usd_price = Uint128::from((price_fiat.0 * 100.0).round() as u64);
        let fiat_currency = price_fiat.1.clone();
        currency_prices.push(CurrencyPrice {
            currency: fiat_currency,
            usd_price,
            updated_at: 0,
        });
    });
    let update_prices_msg = PriceExecuteMsg::UpdatePrices(currency_prices);
    let json_msg = serde_json::to_string(&update_prices_msg).unwrap();
    println!("update_msg: {}", json_msg);
    let execute_msg = MsgExecuteContract {
        sender: sender_addr.clone(),
        contract: contract_addr.clone().parse::<AccountId>().unwrap(),
        msg: json_msg.into_bytes(),
        funds: vec![],
    };
    tx_body_builder.msg(execute_msg.into_any().unwrap());

    let signer_info = SignerInfo::single_direct(
        Some(sender_pub_key.clone()),
        account_data
            .account
            .sequence
            .clone()
            .parse::<i64>()
            .unwrap() as u64,
    );
    let denom = cfg.get_string("denom").unwrap().parse::<Denom>().unwrap();
    let gas_amount = Coin {
        amount: 4500u128,
        denom,
    };
    let auth_info = signer_info.auth_info(Fee::from_amount_and_gas(gas_amount, 300_000u64));
    let tx_body = tx_body_builder.finish();
    let account_number = account_data
        .account
        .account_number
        .clone()
        .parse::<i64>()
        .unwrap() as u64;
    let chain_id = cfg.get_string("chain_id").unwrap().parse::<Id>().unwrap();
    let sign_doc = SignDoc::new(&tx_body, &auth_info, &chain_id, account_number).unwrap();
    let tx_signed = sign_doc.sign(&sender_priv_key).unwrap();
    let rpc_url = cfg.get_string("rpc").unwrap();
    let client = HttpClient::new(rpc_url.as_str()).unwrap();
    let res = tx_signed.broadcast_commit(&client).await.unwrap();
    println!("{}", res.deliver_tx.info.to_string());
    println!("res: {:#?}", res);
}
